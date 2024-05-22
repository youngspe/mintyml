mod elements;
mod line;
mod selectors;
mod text;

use alloc::{vec, vec::Vec};

use gramma::parse::{Location, LocationRange};

use crate::{
    ast,
    error::{
        Errors, InternalError, InternalResult, ItemType, MisplacedKind, SyntaxError,
        SyntaxErrorKind, UnclosedDelimiterKind,
    },
    escape::escape_errors,
};

pub use elements::*;
pub use selectors::*;
pub use text::*;

type BuildResult<T = ()> = InternalResult<T>;

#[non_exhaustive]
pub struct Node<'cfg> {
    pub range: LocationRange,
    pub node_type: NodeType<'cfg>,
}

impl<'cfg> Node<'cfg> {
    pub fn item_type(&self) -> ItemType {
        match self.node_type {
            NodeType::Element { ref value } => match value.element_type {
                ElementType::Paragraph {} => ItemType::Paragraph {},
                ElementType::Standard { .. } => ItemType::Element {},
                ElementType::Inline { .. } => ItemType::InlineElement {},
                ElementType::Special { .. } => ItemType::InlineElement {},
                ElementType::Multiline { .. } => ItemType::Multiline {},
            },
            NodeType::TextLike { ref value } => match value {
                TextLike::Text { .. } => ItemType::Text {},
                TextLike::Comment { .. } => ItemType::Comment {},
                TextLike::Space { .. } => ItemType::Space {},
            },
        }
    }

    pub fn as_element(&self) -> Option<&Element<'cfg>> {
        match &self.node_type {
            NodeType::Element { value } => Some(value),
            NodeType::TextLike { .. } => None,
        }
    }

    pub fn as_element_mut(&mut self) -> Option<&mut Element<'cfg>> {
        match &mut self.node_type {
            NodeType::Element { value } => Some(value),
            NodeType::TextLike { .. } => None,
        }
    }

    /// i.e. is not space or comment
    pub fn is_visible(&self) -> bool {
        !matches!(
            self.node_type,
            NodeType::TextLike {
                value: TextLike::Comment { .. } | TextLike::Space { .. },
            }
        )
    }
}

#[non_exhaustive]
pub enum NodeType<'cfg> {
    #[non_exhaustive]
    Element { value: Element<'cfg> },
    #[non_exhaustive]
    TextLike { value: TextLike<'cfg> },
}

pub struct Content<'cfg> {
    pub range: LocationRange,
    pub nodes: Vec<Node<'cfg>>,
}

impl<'cfg> From<Node<'cfg>> for Content<'cfg> {
    fn from(value: Node<'cfg>) -> Self {
        Self {
            range: value.range,
            nodes: vec![value],
        }
    }
}

/// An object that holds relevant state and resources for building a document.
#[derive(Debug)]
struct BuildContext<'cx, 'cfg> {
    /// The MinTyML source string.
    pub src: &'cfg str,
    /// All syntax errors found while building so far.
    pub errors: &'cx mut Errors,
}

impl<'cfg> BuildContext<'_, 'cfg> {
    /// Extracts a slice of the source.
    fn slice(&self, range: LocationRange) -> TextSlice<'cfg> {
        TextSlice::FromSource { range }
    }

    /// Extracts a slice of the source, validating any escape sequences within.
    fn escapable_slice(
        &mut self,
        range: LocationRange,
        escape: bool,
    ) -> BuildResult<TextSlice<'cfg>> {
        if escape {
            self.errors
                .syntax(escape_errors(range.slice(self.src), range.start))?;
        }
        Ok(self.slice(range))
    }
    fn unclosed(
        &mut self,
        opening: LocationRange,
        delimiter: UnclosedDelimiterKind,
    ) -> BuildResult {
        self.errors.syntax([SyntaxError {
            range: opening,
            kind: SyntaxErrorKind::Unclosed { delimiter },
        }])
    }

    fn invalid(&mut self, range: LocationRange, item: ItemType) -> BuildResult {
        self.errors.syntax([SyntaxError {
            range,
            kind: SyntaxErrorKind::InvalidItem { item },
        }])
    }

    fn misplaced(&mut self, range: LocationRange, kind: MisplacedKind) -> BuildResult {
        self.errors.syntax([SyntaxError {
            range,
            kind: SyntaxErrorKind::MisplacedItem { kind },
        }])
    }
}

impl<'cfg> BuildContext<'_, 'cfg> {
    pub fn build_content(
        &mut self,
        &ast::Content {
            start,
            ref lines,
            end,
        }: &ast::Content,
        form_paragraphs: bool,
    ) -> BuildResult<Content<'cfg>> {
        let mut out_nodes = Vec::<Node<'cfg>>::new();
        let mut node_buf = Vec::new();
        let range = LocationRange { start, end };
        let mut last_line_end = start;

        for &ast::Line {
            start,
            ref nodes,
            end,
        } in lines
        {
            if nodes.is_empty() {
                out_nodes.push(self.paragraph_end(last_line_end, end)?);
            } else {
                let mut nodes = &nodes[..];
                node_buf = self.build_line(&mut nodes, node_buf)?;

                self.add_line(
                    &mut out_nodes,
                    &mut node_buf,
                    LocationRange { start, end },
                    last_line_end,
                    form_paragraphs,
                )?;
            }
            last_line_end = end;
        }

        Ok(Content {
            range,
            nodes: out_nodes,
        })
    }

    fn append_to_previous_node_if_applicable(
        &mut self,
        out_nodes: &mut Vec<Node<'cfg>>,
        line: &mut Vec<Node<'cfg>>,
        range: LocationRange,
        last_line_end: Location,
    ) -> BuildResult {
        // IF the following conditions are met:

        // - The last node exists
        let Some(last_node) = out_nodes.last_mut() else {
            return Ok(());
        };

        // - The last node is an element
        let NodeType::Element {
            value: ref mut last_element,
        } = last_node.node_type
        else {
            return Ok(());
        };

        // - The last node is either a line element or a paragraph
        let (ElementType::Standard {
            delimiter: ElementDelimiter::Line { .. },
        }
        | ElementType::Paragraph { .. }) = last_element.element_type
        else {
            return Ok(());
        };

        // THEN add all the nodes in the current line to the previous node, preceded by a LineEnd node:

        let line_end = self.line_end(last_line_end, range.start)?;

        last_element
            .content
            .nodes
            .extend([line_end].into_iter().chain(line.drain(..)));

        // Make sure we update the ranges for the previous node to include the current line:
        last_element.range.end = range.end;
        last_node.range.end = range.end;

        Ok(())
    }
}

#[non_exhaustive]
pub struct Document<'cfg> {
    pub range: LocationRange,
    pub content: Content<'cfg>,
}

impl<'cfg> Document<'cfg> {
    /// Converts an abstract syntax tree to a document.
    pub(crate) fn from_ast(
        src: &'cfg str,
        ast: &ast::Document,
        errors: &mut Errors,
    ) -> InternalResult<Self> {
        let mut cx = BuildContext { src, errors };
        let content = cx.build_content(&ast.content, true)?;

        Ok(Self {
            range: LocationRange {
                start: Location { position: 0 },
                end: Location {
                    position: src.len(),
                },
            },
            content,
        })
    }

    pub(crate) fn parse(src: &'cfg str, errors: &mut Errors) -> InternalResult<Self> {
        match ast::parse(src) {
            Ok(ast) => Self::from_ast(src, &ast, errors),
            Err(e) => {
                errors.syntax([e])?;
                return Err(InternalError);
            }
        }
    }
}
