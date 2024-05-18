use alloc::{borrow::Cow, string::String};
use gramma::parse::{Location, LocationRange};

use crate::{ast, utils::default};

use super::{BuildContext, BuildResult, Node, NodeType};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextSlice<'cfg> {
    #[non_exhaustive]
    FromSource { range: LocationRange },
    #[non_exhaustive]
    Provided { value: Cow<'cfg, str> },
}

impl<'cfg> TextSlice<'cfg> {
    pub fn as_str<'src>(&'src self, src: &'src str) -> &'src str {
        match self {
            TextSlice::FromSource { range } => range.slice(src),
            TextSlice::Provided { value } => &*value,
        }
    }
}

impl<'cfg> From<Cow<'cfg, str>> for TextSlice<'cfg> {
    fn from(value: Cow<'cfg, str>) -> Self {
        Self::Provided { value }
    }
}

impl<'cfg> From<&'cfg str> for TextSlice<'cfg> {
    fn from(value: &'cfg str) -> Self {
        Self::Provided {
            value: value.into(),
        }
    }
}

impl<'cfg> From<String> for TextSlice<'_> {
    fn from(value: String) -> Self {
        Self::Provided {
            value: value.into(),
        }
    }
}

impl From<LocationRange> for TextSlice<'_> {
    fn from(range: LocationRange) -> Self {
        Self::FromSource { range }
    }
}

impl<'cfg> From<Space<'cfg>> for NodeType<'cfg> {
    fn from(value: Space<'cfg>) -> Self {
        NodeType::TextLike {
            value: TextLike::Space { value },
        }
    }
}

impl<'cfg> TextSlice<'cfg> {
    pub fn is_empty(&self) -> bool {
        match self {
            TextSlice::FromSource { range } => range.end <= range.start,
            TextSlice::Provided { value } => value.is_empty(),
        }
    }
}

impl<'cfg> Default for TextSlice<'cfg> {
    fn default() -> Self {
        Self::Provided { value: default() }
    }
}

#[non_exhaustive]
pub struct Text<'cfg> {
    pub slice: TextSlice<'cfg>,
    pub multiline: bool,
    pub unescape_in: bool,
    pub escape_out: bool,
    pub raw: bool,
}

#[non_exhaustive]
pub enum TextLike<'cfg> {
    #[non_exhaustive]
    Text { value: Text<'cfg> },
    #[non_exhaustive]
    Comment { value: Comment<'cfg> },
    #[non_exhaustive]
    Space { value: Space<'cfg> },
}

impl<'cfg> From<Space<'cfg>> for TextLike<'cfg> {
    fn from(value: Space<'cfg>) -> Self {
        Self::Space { value }
    }
}

/// Represents some kind of whitespace that should be considered when converting to HTML.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Space<'cfg> {
    /// Whitespace between elements on the same line.
    #[non_exhaustive]
    Inline {},
    /// Whitespace between lines of a paragraph.
    #[non_exhaustive]
    LineEnd {},
    /// Whitespace at the end of a paragraph.
    #[non_exhaustive]
    ParagraphEnd {},
    /// A specific string of whitespace.
    #[non_exhaustive]
    Exact { slice: TextSlice<'cfg> },
}

#[non_exhaustive]
pub enum Comment<'cfg> {
    #[non_exhaustive]
    Tag { value: TextSlice<'cfg> },
}

impl<'cfg> BuildContext<'cfg> {
    pub fn build_text_node(
        &mut self,
        range: LocationRange,
        unescape_in: bool,
        escape_out: bool,
        raw: bool,
        multiline: bool,
    ) -> BuildResult<Node<'cfg>> {
        Ok(Node {
            range,
            node_type: NodeType::TextLike {
                value: TextLike::Text {
                    value: Text {
                        slice: self.escapable_slice(range, unescape_in)?,
                        unescape_in,
                        escape_out,
                        raw,
                        multiline,
                    },
                },
            },
        })
    }

    fn build_verbatim_node(
        &mut self,
        ast::Verbatim { open, raw, tail }: &ast::Verbatim,
    ) -> BuildResult<Node<'cfg>> {
        let (tail_range, hash_count) = match tail {
            ast::VerbatimTail::Verbatim0 { value } => (value.range, 0),
            ast::VerbatimTail::Verbatim1 { value } => (value.range, 1),
            ast::VerbatimTail::Verbatim2 { value } => (value.range, 2),
        };

        // Difference between tail_range and the range containing content:
        //   open  |  raw  |                        tail                               |
        //   <[    |  raw  |      ##     |  [  |  <content>  |  ]  |     ##     |  ]>  |
        //         |       | hash_count  +  1  |             |  1  + hash_count +  2   |
        //                 |    inset_start    |             |        inset_end        |
        let inset_start = hash_count + 1;
        let inset_end = hash_count + 3;

        let mut content_range = tail_range;
        content_range.start += inset_start;
        content_range.end -= inset_end;

        let outer_range = open.range.combine(tail_range);

        Ok(Node {
            range: outer_range,
            node_type: NodeType::TextLike {
                value: TextLike::Text {
                    value: Text {
                        slice: self.slice(content_range),
                        unescape_in: false,
                        escape_out: raw.is_none(),
                        multiline: false,
                        raw: false,
                    },
                },
            },
        })
    }

    pub fn build_inline_text(&mut self, text: &ast::InlineText) -> BuildResult<Node<'cfg>> {
        match text {
            ast::InlineText::Segment { value } => {
                self.build_text_node(value.range, true, true, false, false)
            }
            ast::InlineText::Verbatim { value } => self.build_verbatim_node(value),
            ast::InlineText::Comment { comment } => self.build_comment_node(comment),
            ast::InlineText::Interpolation { interpolation } => {
                self.build_text_node(interpolation.range, false, false, true, false)
            }
        }
    }

    pub fn build_comment_node(
        &mut self,
        &ast::Comment {
            start,
            ref open,
            inner,
            ref close,
            end,
        }: &ast::Comment,
    ) -> BuildResult<Node<'cfg>> {
        let range = LocationRange { start, end };

        if close.is_none() {
            self.unclosed(open.range, crate::error::UnclosedDelimiterKind::Comment {})?;
        }

        Ok(Node {
            range,
            node_type: NodeType::TextLike {
                value: TextLike::Comment {
                    value: Comment::Tag {
                        value: self.slice(inner),
                    },
                },
            },
        })
    }

    pub fn exact_space(&mut self, range: LocationRange) -> BuildResult<Node<'cfg>> {
        Ok(Node {
            range,
            node_type: NodeType::TextLike {
                value: TextLike::Space {
                    value: Space::Exact {
                        slice: self.slice(range),
                    },
                },
            },
        })
    }

    pub fn line_end(
        &mut self,
        prev_end: Location,
        next_start: Location,
    ) -> BuildResult<Node<'cfg>> {
        Ok(Node {
            range: LocationRange {
                start: prev_end,
                end: next_start,
            },
            node_type: Space::LineEnd {}.into(),
        })
    }

    pub fn paragraph_end(
        &mut self,
        prev_end: Location,
        next_start: Location,
    ) -> BuildResult<Node<'cfg>> {
        Ok(Node {
            range: LocationRange {
                start: prev_end,
                end: next_start,
            },
            node_type: Space::ParagraphEnd {}.into(),
        })
    }

    pub fn inline_space(
        &mut self,
        range: impl Into<Option<LocationRange>>,
    ) -> BuildResult<Node<'cfg>> {
        let range = range.into();
        Ok(Node {
            range: range.unwrap_or(LocationRange::INVALID),
            node_type: NodeType::TextLike {
                value: TextLike::Space {
                    value: Space::Inline {},
                },
            },
        })
    }
}
