use alloc::vec;
use core::mem;
use gramma::parse::LocationRange;

use crate::{
    document::{Attribute, Document, Element, Node, Selector, SelectorElement, Space},
    OutputConfig,
};

/// Tags that generally belong in a `<head>` element.
const METADATA_TAGS: &[&str] = &["base", "link", "meta", "title", "style"];

fn extract_element_with_tag_in<'node, 'src, I: Iterator + Clone>(
    node: &'node mut Node<'src>,
    tags: impl IntoIterator<IntoIter = I>,
) -> Option<&'node mut Element<'src>>
where
    I::Item: AsRef<str>,
{
    let tags = tags.into_iter();
    match &mut node.node_type {
        crate::document::NodeType::Element(e) => match e {
            Element {
                selector:
                    Selector {
                        element: SelectorElement::Name(tag),
                        ..
                    },
                ..
            } if tags.clone().any(|t| tag.eq_ignore_ascii_case(t.as_ref())) => Some(e),
            _ => None,
        },
        _ => None,
    }
}

fn has_tag_in<'node, 'src, I: Iterator + Clone>(
    node: &'node mut Node<'src>,
    tags: impl IntoIterator<IntoIter = I>,
) -> bool
where
    I::Item: AsRef<str>,
{
    extract_element_with_tag_in(node, tags).is_some()
}

/// Transforms `doc` so that its nodes are wrapped in `<html>` tags with a `<head>` and `<body>`
pub fn complete_page<'src>(doc: &mut Document<'src>, config: &OutputConfig<'src>) {
    if let Some(root) = doc
        .nodes
        .iter_mut()
        .find_map(|n| extract_element_with_tag_in(n, ["html"]))
    {
        // There's already an <html> tag so no need to restructure.
        if let Some(lang) = config.lang.as_ref() {
            if !root
                .selector
                .attributes
                .iter()
                .any(|attr| attr.name == "lang")
            {
                root.selector.attributes.push(Attribute {
                    name: "lang".into(),
                    value: Some(lang.clone()),
                })
            }
        }

        return;
    }

    let mut root = Element::with_tag("html");

    if let Some(lang) = config.lang.as_ref() {
        root.selector.attributes.push(Attribute {
            name: "lang".into(),
            value: Some(lang.clone()),
        })
    }

    if doc.nodes.iter_mut().any(|n| has_tag_in(n, ["body"])) {
        // There's already a body tag so just wrap it all in <html> and call it good.
        root.nodes = mem::take(&mut doc.nodes);
    } else {
        let mut head = Element::with_tag("head");
        let mut body = Element::with_tag("body");

        body.nodes = mem::take(&mut doc.nodes)
            .into_iter()
            .filter_map(|mut n| {
                if let Some(e) = extract_element_with_tag_in(&mut n, ["head"]) {
                    head.nodes.extend(mem::take(&mut e.nodes));
                    None
                } else if has_tag_in(&mut n, METADATA_TAGS) {
                    head.nodes.push(n);
                    None
                } else {
                    Some(n)
                }
            })
            .collect();

        let body_range = body
            .nodes
            .first()
            .zip(body.nodes.last())
            .map_or(doc.range, |(first, last)| first.range.combine(last.range));

        root.nodes = vec![
            Node {
                range: LocationRange::INVALID,
                node_type: head.into(),
            },
            Node {
                range: LocationRange::INVALID,
                node_type: Space::ParagraphEnd.into(),
            },
            Node {
                range: body_range,
                node_type: body.into(),
            },
        ];
    }

    doc.nodes = vec![Node {
        range: doc.range,
        node_type: root.into(),
    }];
}
