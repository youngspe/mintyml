use alloc::vec;
use core::mem;

use gramma::parse::LocationRange;

use crate::{
    document::{Document, Element, ElementType, Node, NodeType, Space},
    error::InternalResult,
};

/// Tags that generally belong in a `<head>` element.
const METADATA_TAGS: &[&str] = &["base", "link", "meta", "title", "style"];

/// If `node` is an element whose tag is contained in `tags`, returns a reference to the [Element] value.
fn extract_element_with_tag_in<'node, 'src, I: Iterator + Clone>(
    src: &str,
    node: &'node mut Node<'src>,
    tags: impl IntoIterator<IntoIter = I>,
) -> Option<&'node mut Element<'src>>
where
    I::Item: AsRef<str>,
{
    let tags = tags.into_iter();
    let e = node.as_element_mut()?;

    let tag = e.selectors.first()?.tag.name()?;

    if !tags
        .clone()
        .any(|t| tag.as_str(src).eq_ignore_ascii_case(t.as_ref()))
    {
        return None;
    }

    Some(e)
}

/// Determines whether `node` is an element whose tag is contained in `tags`.
fn has_tag_in<'node, 'src, I: Iterator + Clone>(
    src: &str,
    node: &'node mut Node<'src>,
    tags: impl IntoIterator<IntoIter = I>,
) -> bool
where
    I::Item: AsRef<str>,
{
    extract_element_with_tag_in(src, node, tags).is_some()
}

/// Transforms `doc` so that its nodes are wrapped in `<html>` tags with a `<head>` and `<body>`
pub fn complete_page<'cfg>(mut doc: Document<'cfg>, src: &str) -> InternalResult<Document<'cfg>> {
    if doc
        .content
        .nodes
        .iter_mut()
        .any(|n| has_tag_in(src, n, ["html"]))
    {
        // There's already an <html> tag so no need to restructure.
        return Ok(doc);
    }

    let mut root = Element::new(LocationRange::INVALID, ElementType::Unknown {}).with_tag("html");
    root.content.range = doc.content.range;

    if doc
        .content
        .nodes
        .iter_mut()
        .any(|n| has_tag_in(src, n, ["body"]))
    {
        // There's already a body tag so just wrap it all in <html> and call it good.
        root.content.nodes = doc.content.nodes
    } else {
        let mut head =
            Element::new(LocationRange::INVALID, ElementType::Unknown {}).with_tag("head");
        let mut body =
            Element::new(LocationRange::INVALID, ElementType::Unknown {}).with_tag("body");

        body.content.nodes = doc
            .content
            .nodes
            .into_iter()
            .filter_map(|mut n| {
                if let Some(e) = extract_element_with_tag_in(src, &mut n, ["head"]) {
                    head.content.nodes.extend(mem::take(&mut e.content.nodes));
                    None
                } else if has_tag_in(src, &mut n, METADATA_TAGS) {
                    head.content.nodes.push(n);
                    None
                } else {
                    Some(n)
                }
            })
            .collect();

        root.content.nodes = vec![
            head.into(),
            Node {
                range: LocationRange::INVALID,
                node_type: NodeType::TextLike {
                    text_like: Space::ParagraphEnd {}.into(),
                },
            },
            body.into(),
        ];
    }

    doc.content.nodes = vec![root.into()];
    Ok(doc)
}
