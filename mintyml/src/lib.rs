//! This library exists to convert [MinTyML](https://youngspe.github.io/mintyml)
//! (for <u>Min</u>imalist H<u>TML</u>) markup to its equivalent HTML.
//!
//! This should be considered the reference implementation for MinTyML.
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
extern crate either;
extern crate gramma;

#[cfg(feature = "error-trait")]
extern crate thiserror;

pub(crate) mod ast;
pub(crate) mod config;
pub(crate) mod document;
pub(crate) mod escape;
pub(crate) mod output;
pub(crate) mod transform;
pub(crate) mod utils;

use alloc::{string::String, vec::Vec};
use core::{borrow::Borrow, fmt};

use document::{Document, Src, ToStatic};
use output::OutputError;

pub use config::{MetadataConfig, OutputConfig, SpecialTagConfig};
pub use document::{SyntaxError, SyntaxErrorKind};

/// Represents an error that occurred while converting MinTyML.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "error-trait", derive(thiserror::Error))]
pub enum ConvertError<'src> {
    /// The conversion failed due to one or more syntax errors.
    #[cfg_attr(feature = "error-trait", error("{}", utils::join_display(syntax_errors.iter().map(|x| x.display_with_src(src)), "; ")))]
    Syntax {
        syntax_errors: Vec<SyntaxError>,
        src: Src<'src>,
    },
    /// The conversion failed for some other reason.
    #[cfg_attr(feature = "error-trait", error("Unknown"))]
    Unknown,
}

impl<'src> ConvertError<'src> {
    /// Copies all borrowed data so the error can outlive the source str.
    pub fn to_static(self) -> ConvertError<'static> {
        match self {
            ConvertError::Syntax { syntax_errors, src } => ConvertError::Syntax {
                syntax_errors,
                src: src.to_static(),
            },
            Self::Unknown => ConvertError::Unknown,
        }
    }
}

impl<'src> ToStatic for ConvertError<'src> {
    type Static = ConvertError<'static>;
    fn to_static(self) -> ConvertError<'static> {
        self.to_static()
    }
}

impl From<OutputError> for ConvertError<'_> {
    fn from(value: OutputError) -> Self {
        match value {
            output::OutputError::WriteError(fmt::Error) => Self::Unknown,
        }
    }
}

/// Converts the given MinTyML string `src` using `config` for configuration options.
/// If successful, returns a string containing the converted HTML document.
///
/// # Example
///
/// ```
/// # use mintyml::OutputConfig;
/// let out = mintyml::convert(r#"
/// {
///     Hello there,
///     world!
///     img[src="./pic.png"]>
///
///     > Click <(a.example-link[href=www.example.com]> here )>
///     for more.
///     .empty>
///     .foo#bar.baz> Goodbye
/// }
/// "#, OutputConfig::new()).unwrap();
///
/// assert_eq!(out, concat!(
///     r#"<div>"#,
///     r#"<p>Hello there, world!</p>"#,
///     r#" <img src="./pic.png">"#,
///     r#" <p>Click <a class="example-link" href="www.example.com">here</a> for more.</p>"#,
///     r#" <p class="empty"></p>"#,
///     r#" <p id="bar" class="foo baz">Goodbye</p>"#,
///     r#"</div>"#,
/// ));
/// ```
pub fn convert<'src>(
    src: &'src str,
    config: impl Borrow<OutputConfig<'src>>,
) -> Result<String, ConvertError<'src>> {
    let mut out = String::new();
    convert_to(src, config, &mut out)?;
    Ok(out)
}

/// Converts the given MinTyML string `src` using `config` for configuration options.
/// The converted HTML document will be written to `out`.
///
/// # Example
///
/// ```
/// # use mintyml::OutputConfig;
/// let mut out = String::new();
/// mintyml::convert_to(r#"
/// {
///     Hello there,
///     world!
///     img[src="./pic.png"]>
///
///     > Click <(a.example-link[href=www.example.com]> here )>
///     for more.
///     .empty>
///     .foo#bar.baz> Goodbye
/// }
/// "#, OutputConfig::new(), &mut out).unwrap();
///
/// assert_eq!(out, concat!(
///     r#"<div>"#,
///     r#"<p>Hello there, world!</p>"#,
///     r#" <img src="./pic.png">"#,
///     r#" <p>Click <a class="example-link" href="www.example.com">here</a> for more.</p>"#,
///     r#" <p class="empty"></p>"#,
///     r#" <p id="bar" class="foo baz">Goodbye</p>"#,
///     r#"</div>"#,
/// ));
/// ```
pub fn convert_to<'src>(
    src: &'src str,
    config: impl Borrow<OutputConfig<'src>>,
    out: &mut impl fmt::Write,
) -> Result<(), ConvertError<'src>> {
    let config: &OutputConfig = config.borrow();
    let mut document = Document::parse(src).map_err(|e| ConvertError::Syntax {
        syntax_errors: e,
        src: src.into(),
    })?;

    if config.complete_page.unwrap_or(false) {
        transform::complete_page::complete_page(&mut document, &config);
    }

    transform::infer_elements::infer_elements(&mut document, &config.special_tags);
    transform::apply_lang(&mut document, &config.lang);

    if let Some(ref metadata) = config.metadata {
        transform::metadata::add_metadata(&mut document, metadata);
    }

    output::output_html_to(&document, out, config).map_err(|e| match e {
        OutputError::WriteError(fmt::Error) => ConvertError::Unknown,
    })?;

    Ok(())
}
