#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
extern crate either;
extern crate gramma;

#[cfg(feature = "std")]
extern crate thiserror;

pub(crate) mod ast;
pub(crate) mod escape;
pub(crate) mod ir;
pub(crate) mod output;
pub(crate) mod utils;

use alloc::{borrow::Cow, string::String, vec::Vec};
use core::fmt;

use ir::{Document, ToStatic};
use output::OutputError;

pub use ir::{SyntaxError, SyntaxErrorKind};
pub use output::OutputConfig;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum ConvertError<'src> {
    #[cfg_attr(feature = "std", error("{}", utils::join_display(syntax_errors.iter().map(|x| x.display_with_src(src)), "; ")))]
    Syntax {
        syntax_errors: Vec<SyntaxError>,
        src: Cow<'src, str>,
    },
    #[cfg_attr(feature = "std", error("Unknown"))]
    Unknown,
}

impl<'src> ConvertError<'src> {
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

pub fn convert(src: &str, config: OutputConfig) -> Result<String, ConvertError> {
    let document = Document::parse(src).map_err(|e| ConvertError::Syntax {
        syntax_errors: e,
        src: src.into(),
    })?;
    let mut out = String::new();
    output::output_html_to(&document, &mut out, config).map_err(|e| match e {
        OutputError::WriteError(fmt::Error) => ConvertError::Unknown,
    })?;
    Ok(out)
}
