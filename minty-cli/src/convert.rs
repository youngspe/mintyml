use std::{
    fs::{self, read_dir},
    io::Write,
    iter, mem,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result as AnyResult};
use either::Either;
use mintyml::MetadataConfig;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{
    args::{self, FailFast},
    error_reporter::{ErrorCategory, OwnedStreamName},
    utils::{default, UtilExt},
    AppCx, CxType, IoHelper,
};

struct ConvertCx<'cx, Cx: CxType> {
    app_cx: &'cx AppCx<Cx>,
    args: super::args::Convert,
}

impl<'cx, Cx: CxType> ConvertCx<'cx, Cx> {
    pub(crate) fn execute_stdin(mut self) -> AnyResult<bool> {
        let dest_name = self.get_dest_name()?;
        self.convert(
            OwnedStreamName::Stdio,
            Either::Left::<_, &mut dyn Write>(dest_name),
            None,
        )
    }

    fn get_dest_name(&mut self) -> AnyResult<OwnedStreamName> {
        Ok(match self.args.dest {
            args::ConvertDest { stdout: true, .. } => OwnedStreamName::Stdio,
            args::ConvertDest {
                out: Some(ref mut path),
                ..
            } => OwnedStreamName::File(mem::take(path).into()),
            args::ConvertDest {
                out: None,
                stdout: false,
            } => return Err(anyhow!("Output not specified.").context(ErrorCategory::Argument)),
        })
    }

    pub(crate) fn execute_dir(self) -> AnyResult<bool> {
        let recurse = self.get_recursion();
        let dir = self.args.src.src_dir.as_deref().unwrap_or(Path::new(""));
        let out = self.args.dest.out.as_deref().unwrap_or(dir);

        if out.is_file() {
            return Err(
                anyhow!("<out> should be a directory when no source files are listed.")
                    .context(ErrorCategory::Argument),
            );
        }

        let paths = search_dir(&dir, recurse)?;

        self.convert_relative_paths(&dir, &out, paths)
    }

    pub(crate) fn get_recursion(&self) -> u32 {
        let recurse = match self.args.recurse {
            Some(Some(r)) => r,
            Some(None) => u32::MAX,
            None => 0,
        };
        recurse
    }

    pub(crate) fn execute_flatten(mut self) -> AnyResult<bool> {
        let recurse = self.get_recursion();
        let dest_name = self.get_dest_name()?;
        let dir = self.args.src.src_dir.as_ref();

        let paths = match (dir, self.args.src.src_files.take()) {
            (Some(dir), Some(files)) => files
                .into_iter()
                .map(|f| if f.is_absolute() { f } else { dir.join(f) })
                .collect(),
            (Some(dir), None) => search_dir(&dir, recurse)?,
            (None, Some(files)) => files,
            (None, None) => bail!("No source files provided."),
        };

        let config = self.args.options.as_config();

        let outputs: Vec<_> = paths
            .into_par_iter()
            .try_fold(
                || (Vec::<u8>::new(), true),
                |(mut buf, success), path| {
                    self.convert(
                        OwnedStreamName::File(path.into()),
                        Either::Right(&mut buf),
                        Some(&config),
                    )
                    .map(|success2| (buf, success & success2))
                },
            )
            .collect::<Result<_, _>>()?;

        let mut out = self.app_cx.io.open_write(&dest_name)?;

        outputs
            .into_iter()
            .try_fold(true, |success1, (buf, success2)| {
                out.write_all(&buf)?;
                Ok(success1 & success2)
            })
    }

    pub(crate) fn convert_relative_paths(
        &self,
        src: &Path,
        dest: &Path,
        paths: Vec<PathBuf>,
    ) -> AnyResult<bool> {
        fs::create_dir_all(&dest)?;

        let mut last_dir: &Path = "".as_ref();
        let mut new_dir_buf = default();

        for path in paths.iter().filter_map(|p| p.parent()) {
            if path != last_dir {
                dest.clone_into(&mut new_dir_buf);
                new_dir_buf.push(path);
                fs::create_dir_all(path)?;
                last_dir = path;
            }
        }
        drop(new_dir_buf);

        let config = self.args.options.as_config();

        paths
            .into_par_iter()
            .map(|path| {
                let src_file = src.join(&path);
                let dest_file = output_name(&path, dest, &self.args.options)?;

                self.convert(
                    OwnedStreamName::File(src_file.into()),
                    Either::Left::<_, &mut dyn Write>(OwnedStreamName::File(dest_file.into())),
                    Some(&config),
                )
            })
            .try_reduce(|| true, |lhs, rhs| Ok(lhs & rhs))
    }

    pub(crate) fn convert(
        &self,
        source_name: OwnedStreamName,
        dest_name: Either<OwnedStreamName, impl Write>,
        config: Option<&mintyml::OutputConfig>,
    ) -> AnyResult<bool> {
        let src = try_with_context!(self.app_cx.io.read(&source_name), source_name);

        let mut config_buf = None;
        let config = config.unwrap_or_else(|| config_buf.insert(self.args.options.as_config()));

        let (out, error) = if self.args.options.forgiving.unwrap_or(false) {
            match mintyml::convert_forgiving(&src, config) {
                Ok(out) => (Some(out), None),
                Err((out, error)) => (out, Some(error)),
            }
        } else {
            match mintyml::convert(&src, config) {
                Ok(out) => (Some(out), None),
                Err(error) => (None, Some(error)),
            }
        };

        let success = error.is_none() && out.is_some();

        if let Some(error) = error {
            self.app_cx
                .reporter
                .conversion_error(source_name.clone(), error);
        }

        if let Some(out) = out {
            match dest_name {
                Either::Left(name) => self.app_cx.io.write(&name, out.as_str()),
                Either::Right(mut dest) => dest.write_all(out.as_bytes()).err_into(),
            }
            .context(source_name)?;
        }

        match (success, self.args.options.fail_fast.unwrap_or_default()) {
            (false, FailFast::True) => Err(anyhow!(ErrorCategory::Hidden)),
            (success, _) => Ok(success),
        }
    }

    fn execute(self) -> AnyResult<bool> {
        if self.args.src.stdin {
            return self.execute_stdin().context(OwnedStreamName::Stdio);
        }

        if self.args.src.src_files.is_none() {
            return self.execute_dir();
        }

        if self.args.dest.stdout {
            return self.execute_flatten();
        }

        if self
            .args
            .src
            .src_files
            .as_ref()
            .is_some_and(|s| s.len() == 1)
        {
            if self.args.dest.out.as_ref().is_some_and(|o| !o.is_dir()) {
                return self.execute_flatten();
            }

            return self.execute_dir();
        }

        if self.args.dest.out.as_ref().is_some_and(|o| o.is_file()) {
            return self.execute_flatten();
        }

        self.execute_dir()
    }
}

impl args::ConvertOptions {
    pub(crate) fn as_config(&self) -> mintyml::OutputConfig {
        mintyml::OutputConfig::new()
            .xml(self.xml)
            .complete_page(self.complete_page.unwrap_or(!self.fragment))
            .update(|cfg| {
                if self.pretty {
                    cfg.indent = Some(iter::repeat(' ').take(self.indent as usize).collect());
                }
            })
            .update(|config| {
                for (key, value) in &self.special_tag {
                    let target = match key {
                        args::SpecialTag::Strong => &mut config.special_tags.strong,
                        args::SpecialTag::Emphasis => &mut config.special_tags.emphasis,
                        args::SpecialTag::Underline => &mut config.special_tags.underline,
                        args::SpecialTag::Strike => &mut config.special_tags.strike,
                        args::SpecialTag::Quote => &mut config.special_tags.quote,
                        args::SpecialTag::Code => &mut config.special_tags.code,
                        args::SpecialTag::CodeBlockContainer => {
                            &mut config.special_tags.code_block_container
                        }
                    };
                    *target = Some(value.into())
                }
            })
            .update(|config| {
                match self {
                    args::ConvertOptions {
                        metadata: None | Some(true),
                        metadata_elements: Some(_),
                        ..
                    }
                    | args::ConvertOptions {
                        metadata: Some(true),
                        ..
                    } => {
                        config.metadata = MetadataConfig::new()
                            .elements(self.metadata_elements.unwrap_or(false))
                            .some();
                    }
                    _ => {}
                }
                if self.metadata_elements.is_some() || self.metadata == Some(true) {}
            })
            .update(|config| config.fail_fast = self.fail_fast.map(|f| f != FailFast::False))
    }
}

impl super::args::Convert {
    pub(crate) fn execute(self, cx: &AppCx<impl CxType>) -> AnyResult<bool> {
        cx.reporter.set_mode(self.error_mode);

        ConvertCx {
            app_cx: cx,
            args: self,
        }
        .execute()
    }
}

pub(crate) fn output_name(
    orig: &Path,
    out_dir: &Path,
    options: &args::ConvertOptions,
) -> AnyResult<PathBuf> {
    let mut path = out_dir.join(if orig.is_absolute() {
        orig.file_name()
            .map(Path::new)
            .ok_or_else(|| anyhow!("Invalid file path \"{}\".", orig.display()))?
    } else {
        orig
    });

    change_extension(&mut path, options);
    path.ok()
}

pub(crate) fn change_extension(path: &mut PathBuf, options: &args::ConvertOptions) {
    if has_minty_extension(&path) {
        path.set_extension("");
    }
    path.as_mut_os_string()
        .push(if options.xml { ".xhtml" } else { ".html" });
}

pub(crate) fn has_minty_extension<P: AsRef<Path>>(path: P) -> bool {
    const EXTENSIONS: [&'static str; 2] = ["mty", "minty"];
    let Some(ext) = path.as_ref().extension() else {
        return false;
    };
    EXTENSIONS.iter().any(|ext2| ext.eq_ignore_ascii_case(ext2))
}

pub(crate) fn search_dir(dir: &Path, recurse: u32) -> AnyResult<Vec<PathBuf>> {
    fn inner(
        dir: &mut PathBuf,
        recurse: u32,
        rel: &mut PathBuf,
        out: &mut Vec<PathBuf>,
    ) -> AnyResult<()> {
        for entry in read_dir(&dir)? {
            let entry = entry?;
            let name = &entry.file_name();
            if entry.metadata()?.is_file() {
                if has_minty_extension(name) {
                    out.push(rel.join(name))
                }
            } else if recurse > 0 {
                rel.push(name);
                dir.push(name);
                inner(dir, recurse - 1, rel, out)?;
                dir.pop();
                rel.pop();
            }
        }
        Ok(())
    }

    Vec::new().also_ok(|v| inner(&mut dir.into(), recurse, &mut default(), v))
}
