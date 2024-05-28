use clap::{
    builder::{EnumValueParser, StringValueParser},
    value_parser, ArgAction, Args, Parser, Subcommand, ValueEnum,
};
use std::path::PathBuf;

use crate::key_value::KeyValueParser;

/// Processes MinTyML, a minimalist alternative syntax for HTML.
///
/// For more information, see https://youngspe.github.io/mintyml
/// and https://github.com/youngspe/mintyml
#[derive(Debug, Parser)]
#[command(
    bin_name = "mintyml-cli",
    max_term_width = 100,
    args_override_self = true
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    Convert(Convert),
}

/// Convert MinTyML to HTML.
#[derive(Debug, Args)]
pub(crate) struct Convert {
    #[command(flatten)]
    pub(crate) src: ConvertSource,
    /// Whether to recursively search subdirectories when searching a directory for source files.
    /// If specified, the search will be limited to `DEPTH` levels of nested subdirectories.
    #[arg(
        short = 'r',
        long,
        name = "DEPTH",
        conflicts_with = "src_files",
    )]
    pub(crate) recurse: Option<Option<u32>>,
    #[command(flatten)]
    pub(crate) dest: ConvertDest,
    #[command(flatten)]
    pub(crate) options: ConvertOptions,
    /// Determines how errors should be written to stderr.
    #[arg(long, default_value = "default")]
    pub(crate) error_mode: ErrorMode,
}

#[derive(Debug, ValueEnum, Clone, Copy, PartialEq, Eq)]
pub enum ErrorMode {
    /// stderr will contain human-readable errors.
    Default,
    /// stderr will contain a JSON stream of errors.
    Json,
    /// No errors will be written to stderr
    Silent,
}

#[derive(Debug, Args)]
#[group(multiple = false)]
#[command(next_help_heading = "Output Destination")]
pub(crate) struct ConvertDest {
    /// Write the converted HTML to the given filename or directory
    #[arg(short = 'o', long)]
    pub(crate) out: Option<PathBuf>,
    /// Write the converted HTML to stdout.
    #[arg(long, conflicts_with = "src_dir")]
    pub(crate) stdout: bool,
}

#[derive(Debug, Args)]
#[group(required = true)]
#[command(next_help_heading = "Input Source")]
pub(crate) struct ConvertSource {
    /// Read MinTyML source from stdin.
    #[arg(long, conflicts_with_all = ["src_dir", "src_files"], requires = "ConvertDest")]
    pub(crate) stdin: bool,
    /// Search for MinTyML files in the given directory.
    #[arg(short = 'd', long = "dir")]
    pub(crate) src_dir: Option<PathBuf>,
    /// Convert the specified MinTyML file(s).
    #[arg(num_args = 1..)]
    pub(crate) src_files: Option<Vec<PathBuf>>,
}

#[derive(Debug, Args)]
#[command(next_help_heading = "Output Options")]
pub(crate) struct ConvertOptions {
    /// Produce XHTML5 instead of HTML
    #[arg(short, long)]
    pub(crate) xml: bool,
    /// Produce HTML with line breaks and indentation for readability.
    #[arg(short, long)]
    pub(crate) pretty: bool,
    /// Number of spaces for each indentation level when `--pretty` is enabled.
    #[arg(long, requires = "pretty", value_parser = value_parser!(u8).range(0..=16), default_value = "2")]
    pub(crate) indent: u8,
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
        default_missing_value = "true"
    )]
    pub(crate) complete_page: Option<bool>,
    /// Convert a MinTyML fragment without wrapping it in `<html>` tags.
    /// Equivalent to `--complete-page=false`
    #[arg(long, conflicts_with = "complete_page")]
    pub(crate) fragment: bool,
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
    pub(crate) special_tag: Vec<(SpecialTag, String)>,
    /// If enabled, a best-effort conversion will be attempted for files with errors.
    #[arg(
        long, num_args = 0..=1,
        require_equals = true, action = ArgAction::Set,
        default_missing_value = "true",
    )]
    pub(crate) forgiving: Option<bool>,
    /// If enabled, stop processing after an error is found.
    #[arg(
        long, num_args = 0..=1,
        require_equals = true, action = ArgAction::Set,
        default_missing_value = "true", conflicts_with = "forgiving"
    )]
    pub(crate) fail_fast: Option<FailFast>,
    /// EXPERIMENTAL: If enabled, parsing metadata will be added to the output.
    ///
    /// See https://github.com/youngspe/mintyml/blob/main/documentation/general/metadata.md
    /// for more information.
    #[arg(
        long, num_args = 0..=1, value_name = "ENABLE",
        require_equals = true, action = ArgAction::Set,
        default_missing_value = "true",
    )]
    pub(crate) metadata: Option<bool>,
    /// EXPERIMENTAL: Generate elements for nodes that don't correspond directly to HTML elements,
    /// like comments and text segments.
    /// Implies `--metadata`
    #[arg(
        long, num_args = 0..=1, value_name = "ENABLE",
        require_equals = true, action = ArgAction::Set,
        default_missing_value = "true", overrides_with = "metadata",
    )]
    pub(crate) metadata_elements: Option<bool>,
}

#[derive(Debug, Default, ValueEnum, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FailFast {
    /// Continue processing after first error found.
    #[default]
    False,
    /// Stop processing after first error found.
    True,
    /// Stop processing a specific file after first error found, but continue processing additional
    /// files.
    File,
}

#[derive(Debug, ValueEnum, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpecialTag {
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

#[test]
fn cli_valid() {
    use clap::CommandFactory;
    Cli::command().debug_assert()
}
