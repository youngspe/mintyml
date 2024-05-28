use core::fmt;
use std::{
    fmt::Write as _,
    iter,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context};
use mintyml::MetadataConfig;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{
    args::{self, FailFast},
    error_reporter::{ErrorCategory, OwnedStreamName},
    utils::{default, ArcPath, PathExt, UtilExt},
    AppCx, CxType, IoHelper, Result,
};

struct ConvertCx<'cx, Cx: CxType> {
    cx: &'cx AppCx<Cx>,
    args: super::args::Convert,
    dot_path: ArcPath,
    empty_path: ArcPath,
}

impl<'cx, Cx: CxType> ConvertCx<'cx, Cx> {
    pub(crate) fn get_recursion(&self) -> u32 {
        let recurse = match self.args.recurse {
            Some(Some(r)) => r,
            Some(None) => u32::MAX,
            None => 0,
        };
        recurse
    }

    pub(crate) fn convert(
        &self,
        source_name: OwnedStreamName,
        dest_name: OwnedStreamName,
        config: Option<&mintyml::OutputConfig>,
    ) -> Result<bool> {
        let src = try_with_context!(self.cx.io.read(&source_name), source_name);

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
            self.cx
                .reporter
                .conversion_error(source_name.clone(), error);
        }

        if let Some(out) = out {
            self.cx
                .io
                .write(&dest_name, out.as_str())
                .context(source_name)?;
        }

        match (success, self.args.options.fail_fast.unwrap_or_default()) {
            (false, FailFast::True) => Err(anyhow!(ErrorCategory::Hidden)),
            (success, _) => Ok(success),
        }
    }

    fn execute(mut self) -> Result<bool> {
        let src = self.conversion_src_type()?;
        let dest = self.conversion_dest_type()?;

        self.convert_src_to_dest(src, dest)
    }

    fn search_dir_inner(
        &self,
        base: &ArcPath,
        exact: &mut PathBuf,
        recurse: u32,
        rel: &mut PathBuf,
        out: &mut Vec<SourceFileLocation>,
    ) -> Result {
        let entries: Vec<_> = self.cx.io.read_dir(&exact)?.collect::<Result<_, _>>()?;

        for (ref name, info) in entries {
            if info.is_file() {
                if has_minty_extension(name) {
                    out.push(SourceFileLocation {
                        base: base.clone(),
                        relative: rel.join(name).into(),
                    })
                }
            } else if recurse > 0 {
                rel.push(name);
                exact.push(name);
                self.search_dir_inner(base, exact, recurse - 1, rel, out)?;
                exact.pop();
                rel.pop();
            }
        }
        Ok(())
    }
    fn search_dir(
        &self,
        base: &ArcPath,
        mut rel: PathBuf,
        out: &mut Vec<SourceFileLocation>,
    ) -> Result {
        let mut exact = base.join(&rel);
        self.search_dir_inner(base, &mut exact, self.get_recursion(), &mut rel, out)
    }

    fn conversion_src_type(&mut self) -> Result<SourceType> {
        let mut out = default();

        match (self.args.src.src_files.take(), self.args.src.src_dir.take()) {
            _ if self.args.src.stdin => SourceType::Stdin,
            (Some(mut src_files), dir) if src_files.len() == 1 => {
                let single_argument = src_files.pop().unwrap_or_default();

                let path_type = PathType::for_path(&self.cx, &single_argument)?;
                let (base, relative) = self.get_search_parts(dir, Some(single_argument));

                SourceType::File(SourceFileLocation { base, relative }, path_type)
            }
            (Some(src_files), dir) => {
                for src_file in src_files {
                    let path_info = self.cx.io.path_info(&src_file)?;
                    let (base, relative) = self.get_search_parts(dir.as_ref(), Some(src_file));

                    let Some(path_info) = path_info else {
                        return Err(anyhow!(
                            "'{}' does not exist",
                            SourceFileLocation { base, relative }
                        )
                        .context(ErrorCategory::Argument));
                    };

                    if path_info.is_dir() {
                        self.search_dir(&base, relative.into(), &mut out)?;
                    } else {
                        out.push(SourceFileLocation { base, relative });
                    }
                }

                SourceType::Files(out)
            }
            (None, Some(dir)) => {
                self.search_dir(&dir.into(), default(), &mut out)?;
                SourceType::Dir(out)
            }
            (None, None) => {
                self.search_dir(&self.dot_path, default(), &mut out)?;
                SourceType::Implicit(out)
            }
        }
        .wrap_ok()
    }

    fn conversion_dest_type(&mut self) -> Result<DestinationType> {
        if self.args.dest.stdout {
            DestinationType::Stdout
        } else if let Some(out) = self.args.dest.out.take() {
            let path_type = PathType::for_path(&self.cx, &out)?;
            DestinationType::File(out.into(), path_type)
        } else {
            DestinationType::Implicit
        }
        .wrap_ok()
    }

    fn convert_multiple(
        &mut self,
        src: impl IntoParallelIterator<Item = SourceFileLocation>,
        // if None, keep the original SourceFileLocation.base
        dest: Option<ArcPath>,
    ) -> Result<bool> {
        let config = self.args.options.as_config();
        src.into_par_iter()
            .map(|src| {
                let mut dest_buf;

                if let Some(dest) = dest.as_ref() {
                    dest_buf = dest.join(&src.relative);
                } else {
                    dest_buf = (*src.base).clone();
                    dest_buf.push(&src.relative);
                }

                change_extension(&mut dest_buf, &self.args.options);
                let dest_file: ArcPath = dest_buf.into();
                let src_file = src.joined();

                self.convert(
                    OwnedStreamName::File(src_file),
                    OwnedStreamName::File(dest_file),
                    Some(&config),
                )
            })
            .try_reduce(|| true, |lhs, rhs| Ok(lhs & rhs))
    }

    fn convert_src_to_dest(mut self, src: SourceType, dest: DestinationType) -> Result<bool> {
        match (src, dest) {
            (
                SourceType::File(src, PathType::ProbablyDir | PathType::Dir { .. }),
                DestinationType::File(_, PathType::File) | DestinationType::Stdout,
            ) => return Err(anyhow!("'{src}' is not a file").context(ErrorCategory::Argument)),
            (
                SourceType::File(
                    path,
                    PathType::ProbablyFile | PathType::Unknown | PathType::ProbablyDir,
                ),
                _,
            ) => return Err(anyhow!("'{path}' does not exist").context(ErrorCategory::Argument)),
            (
                SourceType::Dir(_) | SourceType::Implicit(_) | SourceType::Files(_),
                DestinationType::File(out, PathType::File),
            ) => {
                return Err(anyhow!("'{}' is not a directory", out.display())
                    .context(ErrorCategory::Argument))
            }
            (SourceType::File(src, PathType::File), DestinationType::Stdout) => self.convert(
                OwnedStreamName::File(src.joined()),
                OwnedStreamName::Stdio,
                None,
            ),
            (
                SourceType::Dir(_) | SourceType::Implicit(_) | SourceType::Files(_),
                DestinationType::Stdout,
            ) => {
                return Err(anyhow!(
                    "--stdout requires --stdin or exactly one source file"
                ))
            }
            (SourceType::Stdin, DestinationType::Stdout) => {
                self.convert(OwnedStreamName::Stdio, OwnedStreamName::Stdio, None)
            }
            (SourceType::Stdin, DestinationType::File(dest, dest_path_type))
                if dest_path_type >= PathType::ProbablyDir =>
            {
                return Err(
                    anyhow!("'{}' is not a file", dest.display()).context(ErrorCategory::Argument)
                );
            }
            (SourceType::Stdin, DestinationType::File(dest, _)) => {
                self.convert(OwnedStreamName::Stdio, OwnedStreamName::File(dest), None)
            }
            (SourceType::Stdin, DestinationType::Implicit) => {
                return Err(
                    anyhow!("--stdin requires --stdout or exactly one output file")
                        .context(ErrorCategory::Argument),
                )
            }
            (SourceType::File(src, PathType::File), DestinationType::Implicit) => {
                self.convert_multiple([src], None)
            }
            (
                SourceType::File(SourceFileLocation { base, relative }, PathType::Dir { .. }),
                DestinationType::Implicit,
            ) => {
                let mut resolved = default();
                self.search_dir(&base, relative.into_inner(), &mut resolved)?;
                self.convert_multiple(resolved, None)
            }
            (
                SourceType::Dir(resolved)
                | SourceType::Files(resolved)
                | SourceType::Implicit(resolved),
                DestinationType::Implicit,
            ) => self.convert_multiple(resolved, None),
            (
                SourceType::Dir(resolved)
                | SourceType::Files(resolved)
                | SourceType::Implicit(resolved),
                DestinationType::File(dest, _),
            ) => self.convert_multiple(resolved, Some(dest)),

            (
                SourceType::File(_, PathType::File),
                DestinationType::File(
                    dest,
                    PathType::Dir {
                        trailing_slash: false,
                    },
                ),
            ) => {
                return Err(
                    anyhow!("'{}' is a directory", dest.display()).context(ErrorCategory::Argument)
                )
            }
            (
                SourceType::File(src, PathType::File),
                DestinationType::File(dest, dest_path_type),
            ) if dest_path_type >= PathType::ProbablyDir => {
                self.convert_multiple([src], Some(dest))
            }
            (SourceType::File(src, PathType::File), DestinationType::File(dest, _)) => self
                .convert(
                    OwnedStreamName::File(src.joined()),
                    OwnedStreamName::File(dest.into()),
                    None,
                ),

            (
                SourceType::File(
                    SourceFileLocation { mut base, relative },
                    PathType::Dir { trailing_slash },
                ),
                DestinationType::File(dest, _),
            ) => {
                let mut resolved = default();
                let dir: PathBuf;

                if !trailing_slash && dest.has_trailing_slash() && base.as_os_str().is_empty() {
                    // copy the children of the source path rather than the path itself to the dest path
                    base = dest
                        .parent()
                        .map(ArcPath::from)
                        .unwrap_or(self.dot_path.clone());
                    dir = relative.file_name().unwrap_or_default().into();
                } else {
                    base.push(relative);
                    dir = default();
                }

                self.search_dir(&base, dir, &mut resolved)?;
                self.convert_multiple(resolved, Some(dest))
            }
        }
    }

    fn get_search_parts(
        &self,
        dir: Option<impl PathExt>,
        path: Option<impl PathExt>,
    ) -> (ArcPath, ArcPath) {
        if let Some(dir) = dir {
            return (
                dir.into_arc(),
                path.map(PathExt::into_arc)
                    .unwrap_or_else(|| self.empty_path.clone()),
            );
        }

        let Some(path) = path else {
            return (self.empty_path.clone(), self.empty_path.clone());
        };

        let file_name = path
            .as_path()
            .file_name()
            .map(Into::into)
            .unwrap_or_else(|| self.empty_path.clone());

        let mut path = path.into_buf();
        path.pop();

        (path.into_arc(), file_name.into_arc())
    }
}

struct SourceFileLocation {
    base: ArcPath,
    relative: ArcPath,
}

impl SourceFileLocation {
    fn joined(mut self) -> ArcPath {
        self.base.push(self.relative);
        self.base
    }
}

impl fmt::Display for SourceFileLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.base.display())?;
        if !self.base.has_trailing_slash() {
            f.write_char(std::path::MAIN_SEPARATOR)?;
        }
        write!(f, "{}", self.relative.display())?;
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PathType {
    File,
    ProbablyFile,
    Unknown,
    ProbablyDir,
    Dir { trailing_slash: bool },
}

impl PathType {
    fn for_path(cx: &AppCx<impl CxType>, path: &Path) -> Result<Self> {
        if let Some(info) = cx.io.path_info(path)? {
            if info.is_dir() {
                Self::Dir {
                    trailing_slash: path.has_trailing_slash(),
                }
            } else {
                // Here we'll assume if it's not a dir it's a file
                Self::File
            }
        } else if path.has_trailing_slash() {
            Self::ProbablyDir
        } else if path.extension().is_some() {
            Self::ProbablyFile
        } else {
            Self::Unknown
        }
        .wrap_ok()
    }
}

enum SourceType {
    Stdin,
    Dir(Vec<SourceFileLocation>),
    Implicit(Vec<SourceFileLocation>),
    File(SourceFileLocation, PathType),
    Files(Vec<SourceFileLocation>),
}

enum DestinationType {
    Stdout,
    Implicit,
    File(ArcPath, PathType),
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
                            .wrap_some();
                    }
                    _ => {}
                }
                if self.metadata_elements.is_some() || self.metadata == Some(true) {}
            })
            .update(|config| config.fail_fast = self.fail_fast.map(|f| f != FailFast::False))
    }
}

impl super::args::Convert {
    pub(crate) fn execute(self, cx: &AppCx<impl CxType>) -> Result<bool> {
        cx.reporter.set_mode(self.error_mode);

        ConvertCx {
            cx,
            args: self,
            dot_path: ".".into(),
            empty_path: "".into(),
        }
        .execute()
    }
}

pub(crate) fn output_name(
    orig: &Path,
    out_dir: &Path,
    options: &args::ConvertOptions,
) -> Result<PathBuf> {
    let mut path = out_dir.join(if orig.is_absolute() {
        orig.file_name()
            .map(Path::new)
            .ok_or_else(|| anyhow!("Invalid file path \"{}\".", orig.display()))?
    } else {
        orig
    });

    change_extension(&mut path, options);
    path.wrap_ok()
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
