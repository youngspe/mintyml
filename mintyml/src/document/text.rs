use alloc::borrow::Cow;
use gramma::parse::LocationRange;

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

impl<'cfg> Default for TextSlice<'cfg> {
    fn default() -> Self {
        Self::Provided { value: default() }
    }
}

#[non_exhaustive]
pub struct Text<'cfg> {
    pub slice: TextSlice<'cfg>,
    pub unescape_in: bool,
    pub escape_out: bool,
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
    ) -> BuildResult<Node<'cfg>> {
        Ok(Node {
            range,
            node_type: NodeType::TextLike {
                value: TextLike::Text {
                    value: Text {
                        slice: self.escapable_slice(range, unescape_in)?,
                        unescape_in,
                        escape_out,
                    },
                },
            },
        })
    }

    fn build_verbatim_node(
        &mut self,
        ast::Verbatim { open, raw, tail }: &ast::Verbatim,
    ) -> BuildResult<Node> {
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
                    },
                },
            },
        })
    }

    pub fn build_inline_text(&mut self, text: &ast::InlineText) -> BuildResult<Node> {
        Ok(match text {
            ast::InlineText::Segment { value } => self.build_text_node(value.range, true, true)?,
            ast::InlineText::Verbatim { value } => self.build_verbatim_node(value)?,
            ast::InlineText::Comment { comment } => {
                let _ = comment;
                todo!()
            }
            ast::InlineText::Interpolation { interpolation } => {
                self.build_text_node(interpolation.range, false, false)?
            }
        })
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
    ) -> BuildResult<Node> {
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

    pub fn exact_space(&mut self, range: LocationRange) -> BuildResult<Node> {
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

    pub fn inline_space(&mut self, range: impl Into<Option<LocationRange>>) -> BuildResult<Node> {
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
