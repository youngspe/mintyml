//! This library exists to convert [MinTyML](https://youngspe.github.io/mintyml)
//! (for <u>Min</u>imalist H<u>TML</u>) markup to its equivalent HTML.
//!
//! This should be considered the reference implementation for MinTyML.
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
extern crate derive_more;
extern crate either;
extern crate gramma;

#[cfg(feature = "error-trait")]
extern crate thiserror;

pub(crate) mod ast;
pub(crate) mod config;
pub(crate) mod document;
pub mod error;
pub(crate) mod escape;
pub(crate) mod inference;
pub(crate) mod output;
pub(crate) mod transform;
pub(crate) mod utils;

use alloc::{borrow::Cow, string::String};
use core::{borrow::Borrow, fmt};
use error::{Errors, InternalError};

use document::Document;
use output::OutputError;

pub use config::{MetadataConfig, OutputConfig, SpecialTagConfig};

pub use error::ConvertError;
#[deprecated]
#[doc(hidden)]
pub use error::{SyntaxError, SyntaxErrorKind};

type Src<'src> = Cow<'src, str>;

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
    convert_to_internal(src, config.borrow(), &mut out, false)?;
    Ok(out)
}

/// Similar to [`convert`], but may return a best-effort conversion of an ill-formed document
/// in the event of an error.
///
/// # Example
///
/// ```
///
/// # use mintyml::OutputConfig;
/// let (out, _err) = mintyml::convert_forgiving(r#"
/// table {
///   {
///     > Cell A
///     > Cell B
///   {
///     > Cell C
///     > Cell D
///   }
/// }
/// "#, OutputConfig::new()).unwrap_err();
///
/// assert_eq!(out.unwrap(), concat!(
///     r#"<table>"#,
///     r#"<tr><td>Cell A</td> <td>Cell B</td>"#,
///     r#" <td><p>Cell C</p> <p>Cell D</p></td></tr>"#,
///     r#"</table>"#,
/// ));
/// ```
pub fn convert_forgiving<'src>(
    src: &'src str,
    config: impl Borrow<OutputConfig<'src>>,
) -> Result<String, (Option<String>, ConvertError<'src>)> {
    let mut out = String::new();
    match convert_to_internal(src, config.borrow(), &mut out, true) {
        Ok(()) => Ok(out),
        Err(err) if out.is_empty() => Err((None, err)),
        Err(err) => Err((Some(out), err)),
    }
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
    convert_to_internal(src, config.borrow(), out, true)
}

fn convert_to_internal<'src>(
    src: &'src str,
    config: &OutputConfig<'src>,
    out: &mut impl fmt::Write,
    forgive: bool,
) -> Result<(), ConvertError<'src>> {
    let mut errors = Errors::new(config);

    let (Ok(()) | Err(InternalError)) = (|| {
        let mut document = Document::parse(src, &mut errors)?;
        document = transform::transform_document(document, config, &mut errors)?;
        inference::engine::infer(src, &mut document.content);

        if errors.is_empty() || forgive {
            output::output_html_to(src, &document, out, config)
                .map_err(|e| match e {
                    OutputError::WriteError(fmt::Error) => ConvertError::Unknown,
                })
                .or_else(|_| errors.unknown())?;
        }

        Ok(())
    })();

    errors.to_convert_error(src)
}
