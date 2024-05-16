mod elements;
mod selectors;
mod text;

use alloc::{vec, vec::Vec};
use core::mem;

use gramma::parse::{Location, LocationRange};

use crate::{
    ast,
    error::{ItemType, MisplacedKind, SyntaxError, SyntaxErrorKind, UnclosedDelimiterKind},
    escape::escape_errors,
    utils::default,
    OutputConfig,
};

pub use elements::*;
pub use selectors::*;
pub use text::*;

/// Indicates some error occurred while building the tree.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
struct BuildError {}

type BuildResult<T = ()> = Result<T, BuildError>;

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
                ElementType::Inline {} => ItemType::InlineElement {},
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
struct BuildContext<'cfg> {
    /// The MinTyML source string.
    pub src: &'cfg str,
    /// All syntax errors found while building so far.
    pub errors: Vec<SyntaxError>,
    /// If true, exit at the first error.
    pub fail_fast: bool,
}

impl<'cfg> BuildContext<'cfg> {
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
            self.record_errors(escape_errors(range.slice(self.src), range.start))?;
        }
        Ok(self.slice(range))
    }

    /// Adds the given errors to the context.
    /// Returns `Err(_)` if `errors` contained at least one value.
    fn record_errors<E: Into<SyntaxError>>(
        &mut self,
        errors: impl IntoIterator<Item = E>,
    ) -> BuildResult<()> {
        let pre_len = self.errors.len();
        self.errors.extend(errors.into_iter().map(Into::into));
        if self.fail_fast && self.errors.len() > pre_len {
            Err(default())
        } else {
            Ok(())
        }
    }

    fn unclosed(
        &mut self,
        opening: LocationRange,
        delimiter: UnclosedDelimiterKind,
    ) -> BuildResult {
        self.record_errors([SyntaxError {
            range: opening,
            kind: SyntaxErrorKind::Unclosed { delimiter },
        }])
    }

    fn invalid(&mut self, range: LocationRange, item: ItemType) -> BuildResult {
        self.record_errors([SyntaxError {
            range,
            kind: SyntaxErrorKind::InvalidItem { item },
        }])
    }

    fn misplaced(&mut self, range: LocationRange, kind: MisplacedKind) -> BuildResult {
        self.record_errors([SyntaxError {
            range,
            kind: SyntaxErrorKind::MisplacedItem { kind },
        }])
    }
}

impl<'cfg> BuildContext<'cfg> {
    pub fn build_content(
        &mut self,
        &ast::Content {
            start,
            ref lines,
            end,
        }: &ast::Content,
    ) -> BuildResult<Content<'cfg>> {
        let mut out_nodes = Vec::<Node<'cfg>>::new();
        let mut node_buf = Vec::new();
        let range = LocationRange { start, end };
        let mut consecutive_line = false;
        let mut last_line_end = start;

        for &ast::Line {
            start,
            ref nodes,
            end,
        } in lines
        {
            if nodes.is_empty() {
                if consecutive_line {
                    out_nodes.push(Node {
                        range: LocationRange { start, end },
                        node_type: NodeType::TextLike {
                            value: TextLike::Space {
                                value: Space::ParagraphEnd {},
                            },
                        },
                    });
                }
                consecutive_line = false;
            } else {
                node_buf = self.build_line(&mut &nodes[..], node_buf)?;
                let mut appended = false;

                let first_visible = node_buf.iter().find(|n| n.is_visible());
                let starts_with_element = first_visible
                    .and_then(Node::as_element)
                    .map(|e| matches!(e.element_type, ElementType::Standard { .. }))
                    .unwrap_or(false);

                // Determine if this line should be added to the last element:
                if consecutive_line && !starts_with_element {
                    if let Some((last_node_range, last_element)) = out_nodes
                        .iter_mut()
                        .rfind(|n| n.is_visible())
                        .and_then(|n| match n.node_type {
                            NodeType::Element { ref mut value } => Some((&mut n.range, value)),
                            _ => None,
                        })
                        .filter(|(_, e)| {
                            matches!(
                                e.element_type,
                                ElementType::Standard {
                                    delimiter: ElementDelimiter::Line { .. },
                                } | ElementType::Paragraph { .. }
                            )
                        })
                    {
                        appended = true;
                        let line_end_range = LocationRange {
                            start: last_line_end,
                            end: start,
                        };
                        last_element.content.nodes.push(Node {
                            range: line_end_range,
                            node_type: NodeType::TextLike {
                                value: Space::LineEnd {}.into(),
                            },
                        });
                        last_element.content.nodes.extend(node_buf.drain(..));
                        last_element.range.end = end;
                        last_node_range.end = end;
                    }
                }
                consecutive_line = true;
                last_line_end = end;
                if !appended {
                    if starts_with_element {
                        out_nodes.extend(node_buf.drain(..))
                    } else {
                        out_nodes.push(
                            Element {
                                content: Content {
                                    range,
                                    nodes: node_buf.drain(..).collect(),
                                },
                                ..Element::new(range, ElementType::Paragraph {})
                            }
                            .into(),
                        )
                    }
                }
            }
        }

        Ok(Content {
            range,
            nodes: out_nodes,
        })
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
                            ..self.build_block(node_range, value)?
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
                    let mut element = self.build_block(prefix_range, value)?;
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
                    out_nodes.push(self.build_element(node_range, element)?.into());
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
        config: &OutputConfig<'cfg>,
    ) -> (Option<Self>, Vec<SyntaxError>) {
        let mut cx = BuildContext {
            src,
            errors: default(),
            fail_fast: config.fail_fast.unwrap_or(false),
        };
        let content = cx.build_content(&ast.content);
        let out = content.map(|content| Self {
            range: LocationRange {
                start: Location { position: 0 },
                end: Location {
                    position: src.len(),
                },
            },
            content,
        });
        (out.ok(), cx.errors)
    }

    pub(crate) fn parse(
        src: &'cfg str,
        config: &OutputConfig<'cfg>,
    ) -> (Option<Self>, Vec<SyntaxError>) {
        match ast::parse(src) {
            Ok(ast) => Self::from_ast(src, &ast, config),
            Err(e) => return (None, vec![e.into()]),
        }
    }
}
