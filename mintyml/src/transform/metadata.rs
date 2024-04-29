use alloc::string::ToString;
use core::mem;
use gramma::parse::LocationRange;

use crate::{
    config::MetadataConfig,
    document::{Attribute, Document, Element, Node, NodeType, Selector, SelectorElement, Text},
    utils::default,
};

const XMLNS_URI: &str = "tag:youngspe.github.io,2024:mintyml/metadata";

mod attr {
    pub const XMLNS: &str = "xmlns:mty";
    pub const START: &str = "mty:start";
    pub const END: &str = "mty:end";
    pub const CONTENT_START: &str = "mty:ct-start";
    pub const CONTENT_END: &str = "mty:ct-end";
    pub const VERBATIM: &str = "mty:verbatim";
    pub const RAW: &str = "mty:raw";
}
mod tag {
    pub const COMMENT: &str = "mty:comment";
    pub const TEXT: &str = "mty:text";
}
mod literal {
    pub const TRUE: &str = "true";
    pub const FALSE: &str = "false";
}

fn bool_string(value: bool) -> &'static str {
    if value {
        literal::TRUE
    } else {
        literal::FALSE
    }
}

fn add_boolean_attr(target: &mut Selector, name: &'static str, value: bool) {
    if value {
        target.attributes.push(Attribute {
            name: name.into(),
            value: Some(bool_string(value).into()),
        })
    }
}

pub fn add_metadata(target: &mut Document, options: &MetadataConfig) {
    fn handle_element(range: LocationRange, element: &mut Element, root: bool) {
        if root {
            element.selector.attributes.push(Attribute {
                name: attr::XMLNS.into(),
                value: Some(XMLNS_URI.into()),
            });
        }

        element.selector.attributes.extend([
            Attribute {
                name: attr::START.into(),
                value: Some(range.start.position.to_string().into()),
            },
            Attribute {
                name: attr::END.into(),
                value: Some(range.end.position.to_string().into()),
            },
        ]);

        if let (Some(start), Some(end)) = (
            element
                .content_range
                .or_else(|| element.nodes.first().map(|n| n.range))
                .map(|r| r.start),
            element
                .content_range
                .or_else(|| element.nodes.last().map(|n| n.range))
                .map(|r| r.end),
        ) {
            element.selector.attributes.extend([
                Attribute {
                    name: attr::CONTENT_START.into(),
                    value: Some(start.position.to_string().into()),
                },
                Attribute {
                    name: attr::CONTENT_END.into(),
                    value: Some(end.position.to_string().into()),
                },
            ])
        }
    }
    fn inner(
        &mut Node {
            range,
            ref mut node_type,
        }: &mut Node,
        options: &MetadataConfig,
        root: bool,
    ) {
        match node_type {
            NodeType::Element(element) => {
                handle_element(range, element, root);
                for node in &mut element.nodes {
                    inner(node, options, false);
                }
            }
            NodeType::Text(text) if options.elements => {
                let text = mem::take(text);
                let mut element = Element {
                    selector: SelectorElement::Name(tag::TEXT.into()).into(),
                    is_raw: text.raw,
                    ..default()
                };

                add_boolean_attr(&mut element.selector, attr::VERBATIM, !text.escape);
                add_boolean_attr(&mut element.selector, attr::RAW, text.raw);
                element.nodes.push(Node {
                    range: text.range,
                    node_type: NodeType::Text(text),
                });
                handle_element(range, &mut element, root);
                *node_type = NodeType::Element(element);
            }
            NodeType::Comment(comment) if options.elements => {
                let comment = mem::take(comment);
                let mut element = Element {
                    selector: SelectorElement::Name(tag::COMMENT.into()).into(),
                    ..default()
                };
                element.nodes.push(Node {
                    range: comment.range,
                    node_type: NodeType::Text(Text {
                        value: comment.value,
                        range: comment.range,
                        ..default()
                    }),
                });
                handle_element(range, &mut element, root);
                *node_type = NodeType::Element(element);
            }
            NodeType::Text(..)
            | NodeType::Comment(..)
            | NodeType::Space(..)
            | NodeType::Unknown => {}
        }
    }

    for node in &mut target.nodes {
        inner(node, options, true)
    }
}
