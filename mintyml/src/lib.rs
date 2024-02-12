#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
extern crate either;
extern crate gramma;

pub(crate) mod ast;
pub(crate) mod escape;
pub(crate) mod ir;
pub(crate) mod output;
pub(crate) mod utils;

use alloc::{string::String, vec, vec::Vec};
use core::fmt;

use ir::Document;
use output::OutputError;

pub use ir::SyntaxError;
pub use output::OutputConfig;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConvertError {
    Syntax(Vec<SyntaxError>),
    Unknown,
}

impl From<Vec<SyntaxError>> for ConvertError {
    fn from(value: Vec<SyntaxError>) -> Self {
        Self::Syntax(value)
    }
}

impl From<SyntaxError> for ConvertError {
    fn from(value: SyntaxError) -> Self {
        Self::Syntax(vec![value])
    }
}

impl From<OutputError> for ConvertError {
    fn from(value: OutputError) -> Self {
        match value {
            output::OutputError::WriteError(fmt::Error) => Self::Unknown,
        }
    }
}

pub fn convert(src: &str, config: OutputConfig) -> Result<String, ConvertError> {
    let document = Document::parse(src)?;
    let mut out = String::new();
    output::output_html_to(&document, &mut out, config)?;
    Ok(out)
}
