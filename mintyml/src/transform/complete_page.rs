use core::mem;

use crate::ir::{Document, Element, Node, Selector, SelectorElement};

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
    match node {
        crate::ir::Node::Element(e) => match e {
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
pub fn complete_page<'src>(doc: &mut Document<'src>) {
    if doc.nodes.iter_mut().any(|n| has_tag_in(n, ["html"])) {
        // There's already an <html> tag so we're done.
        return;
    }

    let mut root = Element::with_tag("html");

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

        root.nodes = vec![];
    }

    doc.nodes = vec![root.into()];
}
