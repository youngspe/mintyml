mod into_tags;

use core::mem;

use alloc::{vec, vec::Vec};

use derive_more::Display;
use gramma::parse::LocationRange;

use crate::{ast, error::UnclosedDelimiterKind, utils::default};

use super::{BuildContext, BuildResult, Content, Node, NodeType, Selector, TextSlice};

pub use self::into_tags::IntoTags;

#[non_exhaustive]
#[derive(Clone, Copy)]
pub enum ElementDelimiter {
    #[non_exhaustive]
    Line { combinator: LocationRange },
    #[non_exhaustive]
    Block { block: LocationRange },
    #[non_exhaustive]
    LineBlock {
        combinator: LocationRange,
        block: LocationRange,
    },
}

#[non_exhaustive]
#[derive(Clone)]
pub enum ElementType {
    #[non_exhaustive]
    Paragraph {},
    #[non_exhaustive]
    Standard { delimiter: ElementDelimiter },
    #[non_exhaustive]
    Inline { delimiter: Option<ElementDelimiter> },
    #[non_exhaustive]
    Special { kind: SpecialKind },
    #[non_exhaustive]
    Multiline { kind: MultilineKind },
    #[non_exhaustive]
    Unknown {},
}

impl From<SpecialKind> for ElementType {
    fn from(kind: SpecialKind) -> Self {
        Self::Special { kind }
    }
}

impl From<ElementDelimiter> for ElementType {
    fn from(delimiter: ElementDelimiter) -> Self {
        Self::Standard { delimiter }
    }
}

#[non_exhaustive]
pub struct Element<'cfg> {
    pub range: LocationRange,
    pub selectors: Vec<Selector<'cfg>>,
    pub content: Content<'cfg>,
    pub element_type: ElementType,
    pub(crate) format_inline: bool,
    pub(crate) is_raw: bool,
}

impl<'cfg> Element<'cfg> {
    pub fn new(range: LocationRange, element_type: impl Into<ElementType>) -> Self {
        Self {
            range,
            element_type: element_type.into(),
            selectors: Vec::new(),
            content: Content {
                range,
                nodes: default(),
            },
            format_inline: false,
            is_raw: false,
        }
    }

    pub fn is_raw(&self) -> bool {
        self.is_raw
    }

    fn split_at(&mut self, selector_index: usize) {
        let new_selectors = self.selectors.split_off(selector_index);
        let selector_start = new_selectors[0].range.start;

        let new_range = self.content.range.combine(LocationRange {
            start: selector_start,
            end: self.range.end,
        });

        let new_element = Element {
            range: new_range,
            selectors: new_selectors,
            content: Content {
                range: self.content.range,
                nodes: mem::take(&mut self.content.nodes),
            },
            element_type: self.element_type.clone(),
            format_inline: self.format_inline,
            is_raw: self.is_raw,
        };
        self.content.range = self.content.range.combine(new_range);
        self.content.nodes = vec![new_element.into()];
        self.format_inline = true;
    }

    pub fn split_first(&mut self) -> bool {
        if self.selectors.len() > 1 {
            self.split_at(1);
            true
        } else {
            false
        }
    }

    pub(crate) fn format_inline(&self) -> bool {
        self.format_inline
            || matches!(
                self.element_type,
                ElementType::Standard {
                    delimiter: ElementDelimiter::Line { .. }
                } | ElementType::Inline { .. }
                    | ElementType::Special { .. }
                    | ElementType::Paragraph { .. }
                    | ElementType::Multiline { .. }
            )
    }

    pub fn apply_tags(&mut self, tags: impl IntoIterator<Item = TextSlice<'cfg>>) {
        let mut tags = tags.into_iter().filter(|t| !t.is_empty());
        let Some(first) = tags.next() else {
            if !self.selectors.is_empty() {
                self.selectors.remove(0);
            }
            return;
        };

        let selector_location;

        if let Some(selector) = self.selectors.first_mut() {
            selector_location = selector.range.end;
            selector.tag = first.into();
        } else {
            selector_location = self.range.start;
            self.selectors
                .push(Selector::empty(selector_location).with_tag(first))
        }

        self.selectors.splice(
            1..1,
            tags.map(|t| Selector::empty(selector_location).with_tag(t)),
        );
    }
    pub fn with_tag<M>(mut self, tags: impl IntoTags<'cfg, M>) -> Self {
        self.apply_tags(tags.tags());
        self
    }
}

/// The type of a special element.
#[non_exhaustive]
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpecialKind {
    /// `em>`
    #[display(fmt = "emphasis")]
    Emphasis,
    /// `strong>`
    #[display(fmt = "strong")]
    Strong,
    /// `u>`
    #[display(fmt = "underline")]
    Underline,
    /// `s>`
    #[display(fmt = "strike")]
    Strike,
    /// `q>`
    #[display(fmt = "quote")]
    Quote,
    /// `code>`
    #[display(fmt = "code")]
    Code,
    /// `pre>code>`
    #[display(fmt = "code block")]
    CodeBlockContainer,
}

/// The type of a multiline text block.
#[non_exhaustive]
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MultilineKind {
    /// Double quotes
    #[non_exhaustive]
    Escaped {},
    /// Single quotes
    #[non_exhaustive]
    Unescaped {},
    /// Backticks
    #[non_exhaustive]
    Code {},
}

impl<'cfg> From<Element<'cfg>> for Node<'cfg> {
    fn from(value: Element<'cfg>) -> Self {
        Node {
            range: value.range,
            node_type: NodeType::Element { element: value },
        }
    }
}

impl<'cfg> BuildContext<'_, 'cfg> {
    fn build_inline_special(
        &mut self,
        range: LocationRange,
        ast: &ast::InlineSpecial,
    ) -> BuildResult<Element<'cfg>> {
        use ast::InlineSpecial::*;

        let (open, content, is_unclosed) = match ast {
            Emphasis { open, inner, close } => (open.range, inner, close.is_none()),
            Strong { open, inner, close } => (open.range, inner, close.is_none()),
            Underline { open, inner, close } => (open.range, inner, close.is_none()),
            Strike { open, inner, close } => (open.range, inner, close.is_none()),
            Quote { open, inner, close } => (open.range, inner, close.is_none()),
            Code { code } => {
                return Ok(Element {
                    content: {
                        let mut range = code.range;
                        // shave off the first and last 2 chars ("<`", "`>")
                        range.start += 2;
                        range.end -= 2;
                        self.build_text_node(range, false, true, false, false)?
                            .into()
                    },
                    ..Element::new(range, SpecialKind::Code)
                });
            }
        };

        let kind = match ast {
            Emphasis { .. } => SpecialKind::Emphasis,
            Strong { .. } => SpecialKind::Strong,
            Underline { .. } => SpecialKind::Underline,
            Strike { .. } => SpecialKind::Strike,
            Quote { .. } => SpecialKind::Quote,
            Code { .. } => SpecialKind::Code,
        };

        if is_unclosed {
            self.unclosed(open, UnclosedDelimiterKind::SpecialInline { kind })?;
        }

        Ok(Element {
            content: self.build_content(content, false)?,
            ..Element::new(range, kind)
        })
    }

    fn build_inline(
        &mut self,
        range: LocationRange,
        ast::Inline { open, inner, close }: &ast::Inline,
        out_nodes: &mut Vec<Node<'cfg>>,
    ) -> BuildResult {
        let inner_range = LocationRange {
            start: open.range.end,
            end: close.as_ref().map(|c| c.range.start).unwrap_or(range.end),
        };
        if close.is_none() {
            self.unclosed(open.range, UnclosedDelimiterKind::Inline {})?;
        }

        let mut line = self.build_line(&mut &inner[..], default())?;
        self.validate_line(&mut line)?;

        let first_visible_node = line.iter_mut().find(|n| n.is_visible());

        if let Some(node) = first_visible_node {
            if let Some(element) = node.as_element_mut() {
                if let ElementType::Standard { delimiter } = element.element_type {
                    element.element_type = ElementType::Inline {
                        delimiter: Some(delimiter),
                    };

                    element.range = range;
                    node.range = range;
                    out_nodes.extend(line);

                    return Ok(());
                }
            }
        }

        out_nodes.push(
            Element {
                content: Content {
                    range: inner_range,
                    nodes: line,
                },
                ..Element::new(range, ElementType::Inline { delimiter: None })
            }
            .into(),
        );

        Ok(())
    }

    pub fn build_block(
        &mut self,
        range: LocationRange,
        ast::Block {
            l_brace,
            content,
            r_brace,
        }: &ast::Block,
        paragraphs: bool,
    ) -> BuildResult<Element<'cfg>> {
        if r_brace.is_none() {
            self.unclosed(l_brace.range, UnclosedDelimiterKind::Block {})?;
        }
        Ok(Element {
            content: self.build_content(content, paragraphs)?,
            ..Element::new(range, ElementDelimiter::Block { block: range })
        })
    }

    pub fn build_element(
        &mut self,
        range: LocationRange,
        ast: &ast::Element,
        out_nodes: &mut Vec<Node<'cfg>>,
    ) -> BuildResult {
        Ok(match ast {
            &ast::Element::Line { combinator } => out_nodes.push(
                Element {
                    content: Content {
                        range: LocationRange {
                            start: range.end,
                            ..range
                        },
                        nodes: default(),
                    },
                    ..Element::new(range, ElementDelimiter::Line { combinator })
                }
                .into(),
            ),
            ast::Element::Block { value } => {
                out_nodes.push(self.build_block(range, value, true)?.into())
            }
            ast::Element::Inline { value } => self.build_inline(range, value, out_nodes)?,
            ast::Element::InlineSpecial { value } => {
                out_nodes.push(self.build_inline_special(range, value)?.into())
            }
        })
    }

    pub fn build_multiline(
        &mut self,
        range: LocationRange,
        ast: &ast::Multiline,
        out_nodes: &mut Vec<Node<'cfg>>,
    ) -> BuildResult {
        use ast::Multiline::*;

        let (Escaped { start, end, .. } | Unescaped { start, end, .. } | Code { start, end, .. }) =
            *ast;

        let kind = match ast {
            Escaped { .. } => MultilineKind::Escaped {},
            Unescaped { .. } => MultilineKind::Unescaped {},
            Code { .. } => MultilineKind::Code {},
        };

        let unescape_in = match kind {
            MultilineKind::Escaped {} => true,
            MultilineKind::Unescaped {} | MultilineKind::Code {} => false,
        };

        let inner_range = LocationRange { start, end };

        out_nodes.push(
            Element {
                content: Content {
                    range: inner_range,
                    nodes: vec![self.build_text_node(
                        inner_range,
                        unescape_in,
                        true,
                        false,
                        true,
                    )?],
                },
                ..Element::new(range, ElementType::Multiline { kind })
            }
            .into(),
        );

        Ok(())
    }
}
