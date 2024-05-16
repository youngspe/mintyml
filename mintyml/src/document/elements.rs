use core::mem;

use alloc::{vec, vec::Vec};

use derive_more::Display;
use gramma::parse::LocationRange;

use crate::{ast, error::UnclosedDelimiterKind, inference::engine::ChildInference, utils::default};

use super::{BuildContext, BuildResult, Content, Node, NodeType, Selector};

#[non_exhaustive]
#[derive(Clone)]
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
    Inline {},
    #[non_exhaustive]
    Special { kind: SpecialKind },
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
    pub(crate) inference_method: ChildInference<'cfg>,
    pub(super) format_inline: bool,
    pub(super) is_raw: bool,
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
            inference_method: ChildInference::default(),
            format_inline: false,
            is_raw: false,
        }
    }

    pub fn is_raw(&self) -> bool {
        self.is_raw
    }

    /// If `selectors` contains an uninferred tag at index >= 1, split the element
    /// into two nested elements so that the uninferred tag is at index 0 of the child element.
    pub fn split_uninferred(&mut self) {
        if let Some(uninferred_selector_index) = self
            .selectors
            .iter()
            .skip(1)
            .position(|s| s.uninferred())
            .map(|i| i + 1)
        {
            let new_selectors = self.selectors.split_off(uninferred_selector_index);
            let new_element = Element {
                range: self.range,
                selectors: new_selectors,
                content: Content {
                    range: self.content.range,
                    nodes: mem::take(&mut self.content.nodes),
                },
                element_type: self.element_type.clone(),
                inference_method: ChildInference::default(),
                format_inline: self.format_inline,
                is_raw: self.is_raw,
            };
            self.content.nodes = vec![new_element.into()];
            self.format_inline = true;
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
            )
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

impl<'cfg> From<Element<'cfg>> for Node<'cfg> {
    fn from(value: Element<'cfg>) -> Self {
        Node {
            range: value.range,
            node_type: NodeType::Element { value },
        }
    }
}

impl<'cfg> BuildContext<'cfg> {
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

        if !is_unclosed {
            self.unclosed(open, UnclosedDelimiterKind::SpecialInline { kind })?;
        }

        Ok(Element {
            content: self.build_content(content)?,
            ..Element::new(range, kind)
        })
    }

    fn build_inline(
        &mut self,
        range: LocationRange,
        ast::Inline { open, inner, close }: &ast::Inline,
    ) -> BuildResult<Element<'cfg>> {
        if close.is_none() {
            self.unclosed(open.range, UnclosedDelimiterKind::Inline {})?;
        }
        Ok(Element {
            content: self.build_content(inner)?,
            ..Element::new(range, ElementType::Inline {})
        })
    }

    pub fn build_block(
        &mut self,
        range: LocationRange,
        ast::Block {
            l_brace,
            content,
            r_brace,
        }: &ast::Block,
    ) -> BuildResult<Element<'cfg>> {
        if r_brace.is_none() {
            self.unclosed(l_brace.range, UnclosedDelimiterKind::Block {})?;
        }
        Ok(Element {
            content: self.build_content(content)?,
            ..Element::new(range, ElementDelimiter::Block { block: range })
        })
    }

    pub fn build_element(
        &mut self,
        range: LocationRange,
        ast: &ast::Element,
    ) -> BuildResult<Element<'cfg>> {
        Ok(match ast {
            &ast::Element::Line { combinator } => Element {
                content: Content {
                    range: LocationRange {
                        start: range.end,
                        ..range
                    },
                    nodes: default(),
                },
                ..Element::new(range, ElementDelimiter::Line { combinator })
            },
            ast::Element::Block { value } => self.build_block(range, value)?,
            ast::Element::MultilineCode { value } => {
                let _ = value;
                todo!()
            }
            ast::Element::Inline { value } => self.build_inline(range, value)?,
            ast::Element::InlineSpecial { value } => self.build_inline_special(range, value)?,
        })
    }
}
