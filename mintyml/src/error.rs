use core::fmt::{self, Display};

use derive_more::Display;
use gramma::{parse::LocationRange, ParseError};

use crate::{
    document::SpecialKind,
    escape::EscapeError,
    output::OutputError,
    utils::{join_display, DisplayFn},
    Src,
};

/// Represents a syntax error in the MinTyML source.
#[non_exhaustive]
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "error-trait", derive(thiserror::Error), error("{kind:?} at character {}", range.start.position))]
pub struct SyntaxError {
    /// The [LocationRange] encapsulating the syntax error.
    pub range: LocationRange,
    pub kind: SyntaxErrorKind,
}

impl SyntaxError {
    pub(crate) fn display_with_src<'data>(
        &'data self,
        src: &'data str,
    ) -> impl fmt::Display + 'data {
        DisplayFn(move |f| {
            let mut inner = |sample| {
                match &self.kind {
                    SyntaxErrorKind::InvalidEscape { .. } => {
                        write!(f, "Invalid escape sequence {sample}.")
                    }
                    SyntaxErrorKind::ParseFailed { expected } => {
                        write!(
                            f,
                            "Unexpected {sample}. Expected {}",
                            join_display(expected.iter().map(|t| t.name()), " | ")
                        )
                    }
                    SyntaxErrorKind::Unclosed { .. } => {
                        write!(f, "Unclosed delimiter {sample}")
                    }
                    kind => write!(f, "{kind}"),
                }?;

                write!(f, " at character {}", self.range.start.position)
            };

            if self.range.start.position >= src.len() {
                inner(format_args!("end-of-file"))
            } else {
                inner(format_args!(
                    "{:?}",
                    src.get(self.range.start.position..self.range.end.position)
                        .unwrap_or_default()
                ))
            }
        })
    }
}

/// Indicates what caused a syntax error.
#[non_exhaustive]
#[derive(Debug, Display, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SyntaxErrorKind {
    /// An unknown error occurred.
    #[default]
    Unknown,
    /// An invalid escape sequence was found.
    #[non_exhaustive]
    InvalidEscape {},
    /// The document could not be parsed into an abstract syntax tree.
    #[non_exhaustive]
    #[display(
        fmt = "Expected {}",
        r#"join_display(expected.iter().map(|t| t.name()), " | ")"#
    )]
    ParseFailed {
        expected: Vec<gramma::error::ExpectedParse>,
    },
    /// An opening delimiter does not have an accompanying closing delimiter.
    #[non_exhaustive]
    #[display(fmt = "Unclosed {} delimiter", delimiter)]
    Unclosed { delimiter: UnclosedDelimiterKind },
    /// A syntactic item is ill-formed.
    #[non_exhaustive]
    #[display(fmt = "Invalid {}", item)]
    InvalidItem { item: ItemType },
    #[non_exhaustive]
    MisplacedItem { kind: MisplacedKind },
}

impl From<EscapeError> for SyntaxError {
    fn from(value: EscapeError) -> Self {
        Self {
            range: value.range,
            kind: SyntaxErrorKind::InvalidEscape {},
        }
    }
}

impl From<ParseError<'_>> for SyntaxError {
    fn from(value: ParseError<'_>) -> Self {
        let start = value.location;
        let end = start + value.actual.len();
        Self {
            range: LocationRange { start, end },
            kind: SyntaxErrorKind::ParseFailed {
                expected: value.expected,
            },
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Display, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnclosedDelimiterKind {
    /// "{"
    #[display(fmt = "block")]
    #[non_exhaustive]
    Block {},
    /// "<("
    #[display(fmt = "inline")]
    #[non_exhaustive]
    Inline {},
    #[display(fmt = "{} tag", kind)]
    #[non_exhaustive]
    SpecialInline { kind: SpecialKind },
    #[display(fmt = "coment")]
    #[non_exhaustive]
    Comment {},
    /// "["
    #[display(fmt = "attribute list")]
    #[non_exhaustive]
    AttributeList {},
}

#[non_exhaustive]
#[derive(Debug, Display, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ItemType {
    #[non_exhaustive]
    #[display(fmt = "selector")]
    Selector {},
    #[non_exhaustive]
    #[display(fmt = "element")]
    Element {},
}

fn display_conjunction_list<I: IntoIterator>(list: I, conjunction: impl Display) -> impl Display
where
    I::Item: Display,
    I::IntoIter: Clone,
{
    let iter = list.into_iter();
    DisplayFn(move |f| {
        let mut iter = iter.clone().peekable();

        if iter.peek().is_none() {
            return f.write_str("<none>");
        };

        let mut prev_len = 0u32;

        while let Some(item) = iter.next() {
            let is_last = iter.peek().is_none();

            match (prev_len, is_last) {
                (0, _) => write!(f, "{item}")?,
                (1, true) => write!(f, "{conjunction} {item}")?,
                (2.., true) => write!(f, ", {conjunction} {item}")?,
                (1.., false) => write!(f, ", {item}")?,
            }

            prev_len += 1;
        }

        Ok(())
    })
}

#[non_exhaustive]
#[derive(Debug, Display, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MisplacedKind {
    #[display(
        fmt = "{} must be followed by {}",
        target,
        r#"display_conjunction_list(*post, "or")"#
    )]
    #[non_exhaustive]
    MustPrecede {
        target: ItemType,
        post: &'static [ItemType],
    },
    #[display(
        fmt = "{} must not be followed by {}",
        target,
        r#"display_conjunction_list(*post, "or")"#
    )]
    #[non_exhaustive]
    MustNotPrecede {
        target: ItemType,
        post: &'static [ItemType],
    },
    #[display(
        fmt = "{} must follow {}",
        target,
        r#"display_conjunction_list(*pre, "or")"#
    )]
    #[non_exhaustive]
    MustFollow {
        pre: &'static [ItemType],
        target: ItemType,
    },
    #[display(
        fmt = "{} must not follow {}",
        target,
        r#"display_conjunction_list(*pre, "or")"#
    )]
    #[non_exhaustive]
    MustNotFollow {
        pre: &'static [ItemType],
        target: ItemType,
    },
}

/// Represents an error that occurred while converting MinTyML.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "error-trait", derive(thiserror::Error))]
pub enum ConvertError<'src> {
    /// The conversion failed due to one or more syntax errors.
    #[cfg_attr(feature = "error-trait", error("{}", crate::utils::join_display(syntax_errors.iter().map(|x| x.display_with_src(src)), "; ")))]
    Syntax {
        syntax_errors: alloc::vec::Vec<SyntaxError>,
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
                src: src.into_owned().into(),
            },
            Self::Unknown => ConvertError::Unknown,
        }
    }
}

impl From<OutputError> for ConvertError<'_> {
    fn from(value: OutputError) -> Self {
        match value {
            OutputError::WriteError(fmt::Error) => Self::Unknown,
        }
    }
}
