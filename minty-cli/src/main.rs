extern crate anyhow;
extern crate clap;
extern crate mintyml;
extern crate rayon;

mod key_value;

use std::{
    fs::{self, read_dir, OpenOptions},
    io::{self, Read, Seek, Write},
    iter,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Result as AnyResult};
use clap::{
    builder::{EnumValueParser, StringValueParser},
    value_parser, ArgAction, Args, Parser, Subcommand, ValueEnum,
};
use key_value::KeyValueParser;
use mintyml::MetadataConfig;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use utils::UtilExt as _;

use crate::utils::default;

/// Processes MinTyML, a minimalist alternative syntax for HTML.
///
/// For more information, see https://youngspe.github.io/mintyml
/// and https://github.com/youngspe/mintyml
#[derive(Debug, Parser)]
#[command(bin_name = "mintyml-cli", max_term_width = 100)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Convert(Convert),
}

/// Convert MinTyML to HTML.
#[derive(Debug, Args)]
struct Convert {
    #[command(flatten)]
    src: ConvertSource,
    /// Whether to recursively search subdirectories when searching a directory for source files.
    /// If specified, the search will be limited to `DEPTH` levels of nested subdirectories.
    #[arg(
        short = 'r',
        long,
        name = "DEPTH",
        conflicts_with = "src_files",
        requires = "src_dir"
    )]
    recurse: Option<Option<u32>>,
    #[command(flatten)]
    dest: ConvertDest,
    #[command(flatten)]
    options: ConvertOptions,
}

#[derive(Debug, Args)]
#[group(multiple = false)]
#[command(next_help_heading = "Output Destination")]
struct ConvertDest {
    /// Write the converted HTML to the given filename or directory
    #[arg(short = 'o', long)]
    out: Option<PathBuf>,
    /// Write the converted HTML to stdout.
    #[arg(long, conflicts_with = "src_dir")]
    stdout: bool,
}

#[derive(Debug, Args)]
#[group(required = true)]
#[command(next_help_heading = "Input Source")]
struct ConvertSource {
    /// Read MinTyML source from stdin.
    #[arg(long, conflicts_with_all = ["src_dir", "src_files"], requires = "ConvertDest")]
    stdin: bool,
    /// Search for MinTyML files in the given directory.
    #[arg(short = 'd', long = "dir")]
    src_dir: Option<PathBuf>,
    /// Convert the specified MinTyML file(s).
    #[arg(num_args = 1..)]
    src_files: Option<Vec<PathBuf>>,
}

#[derive(Debug, Args)]
#[command(next_help_heading = "Output Options")]
struct ConvertOptions {
    /// Produce XHTML5 instead of HTML
    #[arg(short, long)]
    xml: bool,
    /// Produce HTML with line breaks and indentation for readability.
    #[arg(short, long)]
    pretty: bool,
    /// Number of spaces for each indentation level when `--pretty` is enabled.
    #[arg(long, requires = "pretty", value_parser = value_parser!(u8).range(0..=16), default_value = "2")]
    indent: u8,
    /// Make a complete HTML page by wrapping the contents in `<html>` tags.
    ///
    /// * If the source document already has an `html` element at the top level, no changes will be made.
    ///
    /// * If the source document has a `body` element at the top level, no changes will be made
    /// beyond wrapping the document in `<html>` tags.
    ///
    /// * Otherwise, a `head` element will be created containing all top-level elements that belong in `head`
    /// (e.g. `title`, `meta`, `style`), and a `body` element will be created containing all other top-level elements.
    ///
    /// [default: true]
    #[arg(
        long, num_args = 0..=1, value_name = "ENABLE",
        require_equals = true, action = ArgAction::Set,
        default_missing_value = "true",
    )]
    complete_page: Option<bool>,
    /// Convert a MinTyML fragment without wrapping it in `<html>` tags.
    /// Equivalent to `--complete-page=false`
    #[arg(long, conflicts_with = "complete_page")]
    fragment: bool,
    /// Override the element types used when converting special tags.
    ///
    /// This argument may be used multiple times to allow multiple overrides.
    /// Additionally, multiple overrides can be specified per argument, separated by commas.
    ///
    /// Example:
    ///     --special_tag underline=ins,strike=del
    #[arg(long,
        value_parser = KeyValueParser::<EnumValueParser<SpecialTag>, StringValueParser>::default(),
        action = ArgAction::Append,
        value_delimiter = ','
    )]
    special_tag: Vec<(SpecialTag, String)>,

    /// EXPERIMENTAL: If enabled, parsing metadata will be added to the output.
    ///
    /// See https://github.com/youngspe/mintyml/blob/main/documentation/general/metadata.md
    /// for more information.
    #[arg(
        long, num_args = 0..=1, value_name = "ENABLE",
        require_equals = true, action = ArgAction::Set,
        default_missing_value = "true",
    )]
    metadata: Option<bool>,

    /// EXPERIMENTAL: Generate elements for nodes that don't correspond directly to HTML elements,
    /// like comments and text segments.
    /// Implies `--metadata`
    #[arg(
        long, num_args = 0..=1, value_name = "ENABLE",
        require_equals = true, action = ArgAction::Set,
        default_missing_value = "true", overrides_with = "metadata",
    )]
    metadata_elements: Option<bool>,
}

#[derive(Debug, ValueEnum, Clone, Copy, PartialEq, Eq)]
enum SpecialTag {
    #[value(help = "<# strong #> (default: 'strong')")]
    Strong,
    #[value(help = "</ emphasis /> (default: 'em')")]
    Emphasis,
    #[value(help = "<_ underline _> (default: 'u')")]
    Underline,
    #[value(help = "<~ strike ~> (default: 's')")]
    Strike,
    #[value(help = "<\" quote \"> (default: 'q')")]
    Quote,
    #[value(help = "<` code `> (default: 'code')")]
    Code,
    #[value(help = "``` code block ``` (default: 'pre')")]
    CodeBlockContainer,
}

impl Cli {
    fn execute(self, cx: AppCx<impl CxType>) -> AnyResult<()> {
        match self.command {
            Command::Convert(convert) => convert.execute(&cx),
        }
    }
}

impl Convert {
    fn execute_stdin(self, cx: &AppCx<impl CxType>) -> AnyResult<()> {
        let src = cx.io.stdin()?;
        match &self.dest {
            ConvertDest { stdout: true, .. } => Self::convert(src, cx.io.stdout()?, &self.options),
            ConvertDest {
                out: Some(path), ..
            } => Self::convert(src, cx.io.open_write(path)?, &self.options),
            ConvertDest {
                out: None,
                stdout: false,
            } => bail!("Output not specified."),
        }
    }

    fn execute_dir(self, cx: &AppCx<impl CxType>) -> AnyResult<()> {
        let recurse = Self::get_recursion(self.recurse);

        let dir = self.src.src_dir.as_deref().unwrap_or(Path::new(""));
        let out = self.dest.out.as_deref().unwrap_or(dir);

        if out.is_file() {
            bail!("<out> should be a directory when no source files are listed.")
        }

        let paths = search_dir(&dir, recurse)?;

        Self::convert_relative_paths(cx, &dir, &out, paths, &self.options)
    }

    fn get_recursion(recurse: Option<Option<u32>>) -> u32 {
        let recurse = match recurse {
            Some(Some(r)) => r,
            Some(None) => u32::MAX,
            None => 0,
        };
        recurse
    }

    fn execute_flatten(self, cx: &AppCx<impl CxType>) -> AnyResult<()> {
        let dir = self.src.src_dir;

        let recurse = Self::get_recursion(self.recurse);

        let paths = match (dir, self.src.src_files) {
            (Some(dir), Some(files)) => files
                .into_iter()
                .map(|f| if f.is_absolute() { f } else { dir.join(f) })
                .collect(),
            (Some(dir), None) => search_dir(&dir, recurse)?,
            (None, Some(files)) => files,
            (None, None) => bail!("No source files provided."),
        };
        let converted = paths
            .into_par_iter()
            .map_with(
                (cx, String::new(), &self.options),
                |(cx, buf, options), path| {
                    buf.clear();
                    cx.io.open_read(&path)?.read_to_string(buf)?;
                    mintyml::convert(&buf, Self::get_convert_config(options))
                        .map_err(|e| e.to_static().into())
                },
            )
            .collect::<AnyResult<Vec<_>>>()?;

        if self.dest.stdout {
            let mut out = cx.io.stdout()?;
            converted
                .into_iter()
                .try_for_each(|s| out.write_all(s.as_bytes()))?;
            return Ok(());
        }

        let out_path = self.dest.out.unwrap();
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut out = cx.io.open_write(&out_path)?;
        converted
            .into_iter()
            .try_for_each(|s| out.write_all(s.as_bytes()))?;
        Ok(())
    }

    fn convert_relative_paths(
        cx: &AppCx<impl CxType>,
        src: &Path,
        dest: &Path,
        paths: Vec<PathBuf>,
        options: &ConvertOptions,
    ) -> AnyResult<()> {
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

        paths
            .into_par_iter()
            .try_for_each_with((src, dest, cx), |(src, dest, cx), path| {
                let src_file = src.join(&path);
                let dest_file = output_name(&path, dest, options)?;

                Self::convert(
                    cx.io.open_read(&src_file)?,
                    cx.io.open_write(&dest_file)?,
                    &options,
                )
            })
    }

    fn get_convert_config(options: &ConvertOptions) -> mintyml::OutputConfig {
        mintyml::OutputConfig::new()
            .xml(options.xml)
            .complete_page(options.complete_page.unwrap_or(!options.fragment))
            .update(|cfg| {
                if options.pretty {
                    cfg.indent = Some(iter::repeat(' ').take(options.indent as usize).collect());
                }
            })
            .update(|config| {
                for (key, value) in &options.special_tag {
                    let target = match key {
                        SpecialTag::Strong => &mut config.special_tags.strong,
                        SpecialTag::Emphasis => &mut config.special_tags.emphasis,
                        SpecialTag::Underline => &mut config.special_tags.underline,
                        SpecialTag::Strike => &mut config.special_tags.strike,
                        SpecialTag::Quote => &mut config.special_tags.quote,
                        SpecialTag::Code => &mut config.special_tags.code,
                        SpecialTag::CodeBlockContainer => {
                            &mut config.special_tags.code_block_container
                        }
                    };
                    *target = Some(value.into())
                }
            })
            .update(|config| {
                match options {
                    ConvertOptions {
                        metadata: None | Some(true),
                        metadata_elements: Some(_),
                        ..
                    }
                    | ConvertOptions {
                        metadata: Some(true),
                        ..
                    } => {
                        config.metadata = MetadataConfig::new()
                            .elements(options.metadata_elements.unwrap_or(false))
                            .some();
                    }
                    _ => {}
                }
                if options.metadata_elements.is_some() || options.metadata == Some(true) {}
            })
    }

    fn convert(src: impl Read, mut dest: impl Write, options: &ConvertOptions) -> AnyResult<()> {
        let src = io::read_to_string(src)?;
        let out = mintyml::convert(&src, Self::get_convert_config(options))
            .map_err(|e| anyhow!("{}", e))?;

        dest.write_all(out.as_bytes())?;
        Ok(())
    }

    fn execute(self, cx: &AppCx<impl CxType>) -> AnyResult<()> {
        if self.src.stdin {
            return self.execute_stdin(cx);
        }

        if self.src.src_files.is_none() {
            return self.execute_dir(cx);
        }

        if self.dest.stdout {
            return self.execute_flatten(cx);
        }

        if self.src.src_files.as_ref().is_some_and(|s| s.len() == 1) {
            if self.dest.out.as_ref().is_some_and(|o| !o.is_dir()) {
                return self.execute_flatten(cx);
            }

            return self.execute_dir(cx);
        }

        if self.dest.out.as_ref().is_some_and(|o| o.is_file()) {
            return self.execute_flatten(cx);
        }

        self.execute_dir(cx)
    }
}

fn output_name(orig: &Path, out_dir: &Path, options: &ConvertOptions) -> AnyResult<PathBuf> {
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

fn change_extension(path: &mut PathBuf, options: &ConvertOptions) {
    if has_minty_extension(&path) {
        path.set_extension("");
    }
    path.as_mut_os_string()
        .push(if options.xml { ".xhtml" } else { ".html" });
}

fn has_minty_extension<P: AsRef<Path>>(path: P) -> bool {
    const EXTENSIONS: [&'static str; 2] = ["mty", "minty"];
    let Some(ext) = path.as_ref().extension() else {
        return false;
    };
    EXTENSIONS.iter().any(|ext2| ext.eq_ignore_ascii_case(ext2))
}

fn search_dir(dir: &Path, recurse: u32) -> AnyResult<Vec<PathBuf>> {
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

trait CxType: Send + Sync {
    type Io: IoHelper;
}

trait IoHelper: Send + Sync {
    fn open_read(&self, path: &Path) -> AnyResult<impl Read + Seek + '_>;
    fn open_write(&self, path: &Path) -> AnyResult<impl Write + Seek + '_>;
    fn read(&self, path: &Path) -> AnyResult<String> {
        io::read_to_string(self.open_read(path)?).err_into()
    }
    fn write(&self, path: &Path, value: &str) -> AnyResult<()> {
        self.open_write(path)?
            .write_all(value.as_bytes())
            .err_into()
    }
    fn stdin(&self) -> AnyResult<impl Read>;
    fn stdout(&self) -> AnyResult<impl Write>;
}

struct DefaultCx;

impl CxType for DefaultCx {
    type Io = DefaultIoHelper;
}

struct DefaultIoHelper;

impl IoHelper for DefaultIoHelper {
    fn open_read(&self, path: &Path) -> AnyResult<impl Read + Seek + '_> {
        std::fs::File::open(path).err_into()
    }

    fn open_write(&self, path: &Path) -> AnyResult<impl Write + Seek + '_> {
        let f = OpenOptions::new().create(true).write(true).open(path)?;
        f.set_len(0)?;
        Ok(f)
    }

    fn stdin(&self) -> AnyResult<impl Read> {
        std::io::stdin().ok()
    }

    fn stdout(&self) -> AnyResult<impl Write> {
        std::io::stdout().ok()
    }
}

struct AppCx<Cx: CxType> {
    #[allow(unused)]
    pub cx_type: Cx,
    pub io: Cx::Io,
}

fn main() -> AnyResult<()> {
    Cli::parse().execute(AppCx {
        cx_type: DefaultCx,
        io: DefaultIoHelper,
    })
}

mod utils {
    pub fn default<T: Default>() -> T {
        Default::default()
    }

    pub trait ResultExt:
        Into<Result<Self::OkVal, Self::ErrVal>> + From<Result<Self::OkVal, Self::ErrVal>>
    {
        type OkVal;
        type ErrVal;
        fn into_result(self) -> Result<Self::OkVal, Self::ErrVal> {
            self.into()
        }
    }

    impl<T, E> ResultExt for Result<T, E> {
        type OkVal = T;
        type ErrVal = E;
    }

    pub trait UtilExt: Sized {
        fn also(mut self, f: impl FnOnce(&mut Self)) -> Self {
            f(&mut self);
            self
        }

        fn also_with<T>(mut self, f: impl FnOnce(&mut Self) -> T) -> (Self, T) {
            let out = f(&mut self);
            (self, out)
        }

        fn also_with_ok<T, E>(
            mut self,
            f: impl FnOnce(&mut Self) -> Result<T, E>,
        ) -> Result<(Self, T), E> {
            f(&mut self).map(|out| (self, out))
        }

        fn also_with_some<T>(
            mut self,
            f: impl FnOnce(&mut Self) -> Option<T>,
        ) -> Option<(Self, T)> {
            f(&mut self).map(|out| (self, out))
        }

        fn also_ok<E>(mut self, f: impl FnOnce(&mut Self) -> Result<(), E>) -> Result<Self, E> {
            f(&mut self).map(|()| self)
        }

        fn pipe<R>(self, f: impl FnOnce(Self) -> R) -> R {
            f(self)
        }

        fn ok<E>(self) -> Result<Self, E> {
            Ok(self)
        }

        fn err<T>(self) -> Result<T, Self> {
            Err(self)
        }

        fn some(self) -> Option<Self> {
            Some(self)
        }

        fn pipe_ok<T, E>(self, f: impl FnOnce(Self) -> Result<T, E>) -> Result<T, E> {
            self.pipe(f)
        }

        fn pipe_some<T>(self, f: impl FnOnce(Self) -> Option<T>) -> Option<T> {
            self.pipe(f)
        }

        fn into_<T>(self) -> T
        where
            Self: Into<T>,
        {
            self.into()
        }

        fn try_into_<T>(self) -> Result<T, Self::Error>
        where
            Self: TryInto<T>,
        {
            self.try_into()
        }

        fn wrap_ok<E>(self) -> Result<Self, E> {
            Ok(self)
        }

        fn drop_ok(self) -> Result<(), Self::ErrVal>
        where
            Self: ResultExt,
        {
            self.into_result().map(|_| ())
        }

        fn err_into<E>(self) -> Result<Self::OkVal, E>
        where
            Self: ResultExt,
            Self::ErrVal: Into<E>,
        {
            self.into_result().map_err(Into::into)
        }
    }

    impl<T> UtilExt for T {}
}

#[test]
fn cli_valid() {
    use clap::CommandFactory;
    Cli::command().debug_assert()
}
