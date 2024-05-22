use alloc::{vec, vec::Vec};
use core::mem;

use gramma::parse::{Location, LocationRange};

use crate::{
    ast,
    error::{ItemType, MisplacedKind},
    utils::default,
};

use super::{
    BuildContext, BuildResult, Content, Element, ElementDelimiter, ElementType, MultilineKind,
    Node, Selector,
};

impl<'cfg> BuildContext<'_, 'cfg> {
    pub fn add_line(
        &mut self,
        out_nodes: &mut Vec<Node<'cfg>>,
        line: &mut Vec<Node<'cfg>>,
        range: LocationRange,
        last_line_end: Location,
        form_paragraphs: bool,
    ) -> BuildResult {
        let first_visible = line.iter().find(|n| n.is_visible());
        let contains_visible_nodes = first_visible.is_some();

        let starts_with_element = first_visible
            .and_then(Node::as_element)
            .map(|e| {
                matches!(
                    e.element_type,
                    ElementType::Standard { .. }
                        | ElementType::Multiline {
                            kind: MultilineKind::Code { .. }
                        }
                )
            })
            .unwrap_or(false);

        if !starts_with_element {
            self.append_to_previous_node_if_applicable(out_nodes, line, range, last_line_end)?;
        }

        if !line.is_empty() {
            let do_previous_nodes_exist = out_nodes.iter().any(Node::is_visible);

            if do_previous_nodes_exist {
                out_nodes.push(self.paragraph_end(last_line_end, range.start)?);
            }

            let should_wrap_in_paragraph =
                form_paragraphs && !starts_with_element && contains_visible_nodes;

            if should_wrap_in_paragraph {
                let paragraph = Element {
                    content: Content {
                        range,
                        nodes: line.drain(..).collect(),
                    },
                    ..Element::new(range, ElementType::Paragraph {})
                };
                out_nodes.push(paragraph.into())
            } else {
                out_nodes.extend(line.drain(..))
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
                ast::NodeType::Multiline { multiline } => {
                    self.build_multiline(node_range, multiline, out_nodes)?;
                }
            }
        }

        Ok(())
    }

    pub fn validate_line(&mut self, nodes: &mut Vec<Node<'cfg>>) -> BuildResult {
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
                    (Element {} | Multiline {}, _) => {
                        self.misplaced(last_range, MustNotPrecede { target: last, post })?
                    }
                    (_, Element {} | Multiline {}) => {
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

    pub fn build_line(
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
