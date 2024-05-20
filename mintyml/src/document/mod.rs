mod elements;
mod selectors;
mod text;

use alloc::{vec, vec::Vec};
use core::mem;

use gramma::parse::{Location, LocationRange};

use crate::{
    ast,
    error::{
        Errors, InternalError, InternalResult, ItemType, MisplacedKind, SyntaxError,
        SyntaxErrorKind, UnclosedDelimiterKind,
    },
    escape::escape_errors,
    utils::default,
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
                ElementType::Special {
                    kind: SpecialKind::CodeBlockContainer,
                } => todo!(),
                ElementType::Special { .. } => ItemType::InlineElement {},
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

    fn add_line(
        &mut self,
        out_nodes: &mut Vec<Node<'cfg>>,
        line: &mut Vec<Node<'cfg>>,
        range: LocationRange,
        last_line_end: Location,
        form_paragraphs: bool,
    ) -> BuildResult {
        let first_visible = line.iter().find(|n| n.is_visible());
        let starts_with_element = first_visible
            .and_then(Node::as_element)
            .map(|e| matches!(e.element_type, ElementType::Standard { .. }))
            .unwrap_or(false);

        if !starts_with_element {
            self.append_to_previous_node_if_applicable(out_nodes, line, range, last_line_end)?;
        }

        if !line.is_empty() {
            let do_previous_nodes_exist = out_nodes.iter().any(Node::is_visible);

            if do_previous_nodes_exist {
                out_nodes.push(self.paragraph_end(last_line_end, range.start)?);
            }

            if starts_with_element || !form_paragraphs {
                out_nodes.extend(line.drain(..))
            } else {
                let paragraph = Element {
                    content: Content {
                        range,
                        nodes: line.drain(..).collect(),
                    },
                    ..Element::new(range, ElementType::Paragraph {})
                };
                out_nodes.push(paragraph.into())
            }
        }

        Ok(())
    }

    /// Called when a child combinator has just been read
    fn post_child_combinator(
        &mut self,
        prefix_range: LocationRange,
        nodes: &mut &[(Option<ast::Space>, ast::Node)],
        out_nodes: &mut Vec<Node<'cfg>>,
        selectors: &mut Vec<Selector<'cfg>>,
        last_range: LocationRange,
    ) -> BuildResult {
        let old_nodes = *nodes;
        if let Some((
            &(
                _,
                ast::Node {
                    start,
                    ref node_type,
                    end,
                },
            ),
            ref rest,
        )) = nodes.split_first()
        {
            let node_range = LocationRange { start, end };
            let prefix_range = LocationRange {
                end,
                ..prefix_range
            };
            *nodes = rest;
            match node_type {
                ast::NodeType::Selector { selector } => {
                    selectors.push(self.build_selector(selector)?);
                    return self.post_selector(
                        prefix_range,
                        nodes,
                        out_nodes,
                        selectors,
                        node_range,
                    );
                }
                ast::NodeType::Element {
                    element: ast::Element::Block { value },
                } => {
                    out_nodes.push(
                        Element {
                            element_type: ElementDelimiter::LineBlock {
                                combinator: last_range,
                                block: node_range,
                            }
                            .into(),
                            selectors: mem::take(selectors),
                            ..self.build_block(node_range, value, false)?
                        }
                        .into(),
                    );
                    return Ok(());
                }
                &ast::NodeType::Element {
                    element: ast::Element::Line { combinator },
                } => {
                    selectors.push(Selector::empty(start));
                    return self.post_child_combinator(
                        prefix_range,
                        nodes,
                        out_nodes,
                        selectors,
                        combinator,
                    );
                }
                _ => {}
            }
        }
        *nodes = old_nodes;
        let children = self.build_line(nodes, default())?;
        let range = if let Some(n) = children.last() {
            prefix_range.combine(n.range)
        } else {
            prefix_range
        };
        let content_range = LocationRange {
            start: prefix_range.end,
            ..range
        };

        out_nodes.push(
            Element {
                selectors: mem::take(selectors),
                content: Content {
                    range: content_range,
                    nodes: children,
                },
                ..Element::new(
                    range,
                    ElementDelimiter::Line {
                        combinator: last_range,
                    },
                )
            }
            .into(),
        );
        Ok(())
    }

    /// Called when a selector has just been read
    fn post_selector(
        &mut self,
        prefix_range: LocationRange,
        nodes: &mut &[(Option<ast::Space>, ast::Node)],
        out_nodes: &mut Vec<Node<'cfg>>,
        selectors: &mut Vec<Selector<'cfg>>,
        last_range: LocationRange,
    ) -> BuildResult {
        let old_nodes = *nodes;
        if let Some((
            &(
                _,
                ast::Node {
                    start,
                    ref node_type,
                    end,
                },
            ),
            ref rest,
        )) = nodes.split_first()
        {
            *nodes = rest;
            let prefix_range = LocationRange {
                end,
                ..prefix_range
            };
            match node_type {
                &ast::NodeType::Selector {
                    selector: ast::Selector { start, end, .. },
                } => self.misplaced(
                    LocationRange { start, end },
                    MisplacedKind::MustNotFollow {
                        pre: &[ItemType::Selector {}],
                        target: ItemType::Selector {},
                    },
                )?,
                ast::NodeType::Text {
                    text: ast::InlineText::Comment { comment },
                } => {
                    out_nodes.push(self.build_comment_node(comment)?);

                    return self.post_selector(
                        prefix_range,
                        nodes,
                        out_nodes,
                        selectors,
                        last_range,
                    );
                }
                &ast::NodeType::Element {
                    element: ast::Element::Line { combinator },
                } => {
                    return self.post_child_combinator(
                        prefix_range,
                        nodes,
                        out_nodes,
                        selectors,
                        combinator,
                    );
                }
                ast::NodeType::Element {
                    element: ast::Element::Block { value },
                } => {
                    let mut element = self.build_block(prefix_range, value, true)?;
                    element.selectors = mem::take(selectors);
                    out_nodes.push(element.into());
                    return Ok(());
                }
                _ => self.misplaced(
                    LocationRange { start, end },
                    MisplacedKind::MustPrecede {
                        target: ItemType::Selector {},
                        post: &[ItemType::Element {}],
                    },
                )?,
            }
        }
        *nodes = old_nodes;
        Ok(())
    }

    fn extend_line(
        &mut self,
        nodes: &mut &[(Option<ast::Space>, ast::Node)],
        out_nodes: &mut Vec<Node<'cfg>>,
    ) -> BuildResult {
        enum State<'cfg> {
            Initial,
            Inline { element_nodes: Vec<Node<'cfg>> },
            PostSelector { selectors: Vec<Selector<'cfg>> },
            PostChildCombinator { selectors: Vec<Selector<'cfg>> },
        }
        while let Some((
            &(
                ref space,
                ast::Node {
                    start,
                    ref node_type,
                    end,
                },
            ),
            ref rest,
        )) = nodes.split_first()
        {
            if let Some(space) = space {
                out_nodes.push(self.exact_space(space.range)?);
            }
            let node_range = LocationRange { start, end };
            *nodes = rest;
            match node_type {
                ast::NodeType::Text { text } => {
                    out_nodes.push(self.build_inline_text(text)?);
                }
                ast::NodeType::Selector { selector } => {
                    let mut selectors = vec![self.build_selector(selector)?];
                    self.post_selector(node_range, nodes, out_nodes, &mut selectors, node_range)?;
                }
                ast::NodeType::Element {
                    element: ast::Element::Line { .. },
                } => {
                    let mut selectors = vec![Selector::empty(start)];
                    return self.post_child_combinator(
                        node_range,
                        nodes,
                        out_nodes,
                        &mut selectors,
                        node_range,
                    );
                }
                ast::NodeType::Element { element } => {
                    self.build_element(node_range, element, out_nodes)?;
                }
            }
        }

        Ok(())
    }

    fn validate_line(&mut self, nodes: &mut Vec<Node<'cfg>>) -> BuildResult {
        let mut last_item_type = None::<ItemType>;
        let mut last_range = LocationRange::default();

        for node in nodes {
            let item_type = node.item_type();
            let range = node.range;

            if matches!(item_type, ItemType::Comment { .. } | ItemType::Space { .. }) {
                continue;
            }

            use ItemType::*;
            use MisplacedKind::*;

            if let Some(last) = last_item_type {
                let next = item_type;
                let pre = last.as_slice();
                let post = item_type.as_slice();
                match (last, &item_type) {
                    (Element {}, _) => {
                        self.misplaced(last_range, MustNotPrecede { target: last, post })?
                    }
                    (_, Element {}) => {
                        self.misplaced(range, MustNotFollow { pre, target: next })?
                    }
                    _ => {}
                }
            }

            last_range = range;
            last_item_type = Some(item_type);
        }

        Ok(())
    }

    fn build_line(
        &mut self,
        nodes: &mut &[(Option<ast::Space>, ast::Node)],
        mut out_nodes: Vec<Node<'cfg>>,
    ) -> BuildResult<Vec<Node<'cfg>>> {
        out_nodes.clear();
        self.extend_line(nodes, &mut out_nodes)?;
        self.validate_line(&mut out_nodes)?;
        Ok(out_nodes)
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
