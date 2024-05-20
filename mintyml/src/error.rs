use core::fmt::{self, Display};

use alloc::{borrow::Cow, vec::Vec};

use derive_more::Display;
use gramma::{parse::LocationRange, ParseError};

use crate::{
    document::SpecialKind,
    escape::EscapeError,
    output::OutputError,
    utils::{default, join_display, DisplayFn},
    OutputConfig, Src,
};

/// Represents a syntax error in the MinTyML source.
#[non_exhaustive]
#[derive(Default, Display, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[display(
    fmt = "{kind} at character {}..<{}",
    "range.start.position",
    "range.end.position"
)]
#[cfg_attr(feature = "error-trait", derive(derive_more::Error))]
pub struct SyntaxError {
    /// The [LocationRange] encapsulating the syntax error.
    pub range: LocationRange,
    pub kind: SyntaxErrorKind,
}

impl fmt::Debug for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SyntaxError")?;
        f.debug_set().entry(&format_args!("{self}")).finish()?;
        Ok(())
    }
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

                write!(f, " at character {}", self.range.start.position)?;
                if self.range.end > self.range.start {
                    write!(f, "..<{}", self.range.end.position)?;
                }
                Ok(())
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
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ItemType {
    #[non_exhaustive]
    #[display(fmt = "selector")]
    Selector {},
    #[non_exhaustive]
    #[display(fmt = "element")]
    Element {},
    #[non_exhaustive]
    #[display(fmt = "paragraph")]
    Paragraph {},
    #[non_exhaustive]
    #[display(fmt = "text")]
    Text {},
    #[non_exhaustive]
    #[display(fmt = "inline element")]
    InlineElement {},
    #[non_exhaustive]
    #[display(fmt = "comment")]
    Comment {},
    #[non_exhaustive]
    #[display(fmt = "space")]
    Space {},
    #[non_exhaustive]
    #[display(fmt = "<unknown>")]
    Unknown {},
}

impl ItemType {
    pub fn as_slice(&self) -> &'static [Self] {
        match self {
            ItemType::Selector {} => &[ItemType::Selector {}],
            ItemType::Element {} => &[ItemType::Element {}],
            ItemType::Paragraph {} => &[ItemType::Paragraph {}],
            ItemType::Text {} => &[ItemType::Text {}],
            ItemType::InlineElement {} => &[ItemType::InlineElement {}],
            ItemType::Comment {} => &[ItemType::Comment {}],
            ItemType::Space {} => &[ItemType::Space {}],
            ItemType::Unknown {} => &[ItemType::Unknown {}],
        }
    }
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
#[derive(Display, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "error-trait", derive(derive_more::Error))]
pub enum ConvertError<'src> {
    /// The conversion failed due to one or more syntax errors.
    #[display(
        fmt = "{}",
        r#"crate::utils::join_display(syntax_errors.iter().map(|x| x.display_with_src(src)), "; ")"#,
    )]
    Syntax {
        syntax_errors: Vec<SyntaxError>,
        src: Src<'src>,
    },
    /// The conversion failed due to one or more semantic errors.
    #[display(
        fmt = "{}",
        r#"crate::utils::join_display(semantic_errors.iter().map(|x| x.display_with_src(src)), "; ")"#,
    )]
    Semantic {
        semantic_errors: Vec<SemanticError>,
        src: Src<'src>,
    },
    /// The conversion failed for some other reason.
    Unknown,
}

impl<'src> fmt::Debug for ConvertError<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ConvertError")?;
        f.debug_set().entry(&format_args!("{self}")).finish()?;
        Ok(())
    }
}
impl<'src> ConvertError<'src> {
    /// Copies all borrowed data so the error can outlive the source str.
    pub fn to_static(self) -> ConvertError<'static> {
        match self {
            ConvertError::Syntax { syntax_errors, src } => ConvertError::Syntax {
                syntax_errors,
                src: src.into_owned().into(),
            },
            ConvertError::Semantic {
                semantic_errors,
                src,
            } => ConvertError::Semantic {
                semantic_errors,
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

/// Represents an invalid document structure in otherwise syntactically-correct MinTyML source.
#[non_exhaustive]
#[derive(Debug, Display, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[display(
    fmt = "{kind} at character {}..<{}",
    "range.start.position",
    "range.end.position"
)]
#[cfg_attr(feature = "error-trait", derive(derive_more::Error))]
pub struct SemanticError {
    /// The [LocationRange] of the source that best illustrates the cause of the error.
    pub range: LocationRange,
    pub kind: SemanticErrorKind,
}

/// Indicates what caused a semantic error.
#[derive(Debug, Display, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SemanticErrorKind {
    #[default]
    Unknown,
}

impl SemanticError {
    pub(crate) fn display_with_src<'data>(
        &'data self,
        _src: &'data str,
    ) -> impl fmt::Display + 'data {
        DisplayFn(move |f| match self.kind {
            ref kind => write!(f, "{kind}"),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct InternalError;
pub(crate) type InternalResult<T = ()> = Result<T, InternalError>;

#[derive(Debug)]
pub(crate) struct Errors {
    fail_fast: bool,
    syntax_errors: Vec<SyntaxError>,
    semantic_errors: Vec<SemanticError>,
    unknown_error: bool,
}

impl Errors {
    pub fn new(config: &OutputConfig) -> Self {
        Self {
            fail_fast: config.fail_fast.unwrap_or(false),
            syntax_errors: default(),
            semantic_errors: default(),
            unknown_error: false,
        }
    }

    pub fn count(&self) -> usize {
        self.syntax_errors.len() + self.semantic_errors.len() + self.unknown_error as usize
    }

    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    pub fn to_convert_error<'src>(
        self,
        src: impl Into<Cow<'src, str>>,
    ) -> Result<(), ConvertError<'src>> {
        let src = src.into();

        let Self {
            syntax_errors,
            semantic_errors,
            unknown_error,
            ..
        } = self;

        Err(
            match (syntax_errors.len(), semantic_errors.len(), unknown_error) {
                (0, 0, false) => return Ok(()),
                (1.., _, _) => ConvertError::Syntax { syntax_errors, src },
                (_, 1.., _) => ConvertError::Semantic {
                    semantic_errors,
                    src,
                },
                (_, _, true) => ConvertError::Unknown,
            },
        )
    }

    pub fn syntax<E>(&mut self, errors: impl IntoIterator<Item = E>) -> InternalResult
    where
        E: Into<SyntaxError>,
    {
        let old_len = self.syntax_errors.len();
        self.syntax_errors
            .extend(errors.into_iter().map(Into::into));
        if self.fail_fast && old_len < self.syntax_errors.len() {
            return Err(InternalError);
        }
        Ok(())
    }

    pub fn semantic<E>(&mut self, errors: impl IntoIterator<Item = E>) -> InternalResult
    where
        E: Into<SemanticError>,
    {
        let old_len = self.semantic_errors.len();
        self.semantic_errors
            .extend(errors.into_iter().map(Into::into));
        if self.fail_fast && old_len < self.semantic_errors.len() {
            return Err(InternalError);
        }
        Ok(())
    }

    pub fn unknown(&mut self) -> InternalResult {
        self.unknown_error = true;
        if self.fail_fast {
            return Err(InternalError);
        }
        Ok(())
    }
}
