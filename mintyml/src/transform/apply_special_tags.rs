use alloc::borrow::Cow;

use crate::{
    document::{
        Content, Document, Element, ElementType, MultilineKind, Node, NodeType, SpecialKind,
    },
    error::{Errors, InternalResult},
    OutputConfig,
};

pub fn apply_special_tags<'cfg>(
    mut document: Document<'cfg>,
    config: &OutputConfig<'cfg>,
    errors: &mut Errors,
) -> InternalResult<Document<'cfg>> {
    document.content = TransformContext { config, errors }.transform_content(document.content)?;
    Ok(document)
}

struct TransformContext<'cx, 'cfg> {
    config: &'cx OutputConfig<'cfg>,
    errors: &'cx mut Errors,
}

impl<'cfg> TransformContext<'_, 'cfg> {
    fn tag_name(&self, kind: &SpecialKind) -> InternalResult<Cow<'cfg, str>> {
        use SpecialKind::*;
        let cfg = &self.config.special_tags;
        Ok(match kind {
            Emphasis => cfg.emphasis.clone().unwrap_or("em".into()),
            Strong => cfg.strong.clone().unwrap_or("strong".into()),
            Underline => cfg.underline.clone().unwrap_or("u".into()),
            Strike => cfg.strike.clone().unwrap_or("s".into()),
            Quote => cfg.quote.clone().unwrap_or("q".into()),
            Code => cfg.code.clone().unwrap_or("code".into()),
            CodeBlockContainer => cfg.emphasis.clone().unwrap_or("pre".into()),
        })
    }

    fn transform_content(&mut self, mut content: Content<'cfg>) -> InternalResult<Content<'cfg>> {
        content.nodes = content
            .nodes
            .into_iter()
            .map(|n| self.transform_node(n))
            .collect::<InternalResult<_>>()?;
        Ok(content)
    }

    fn transform_node(&mut self, mut node: Node<'cfg>) -> InternalResult<Node<'cfg>> {
        node.node_type = match node.node_type {
            NodeType::Element { value } => NodeType::Element {
                value: self.transform_element(value)?,
            },
            node_type @ NodeType::TextLike { .. } => node_type,
        };
        Ok(node)
    }

    fn transform_element(&mut self, mut element: Element<'cfg>) -> InternalResult<Element<'cfg>> {
        match element.element_type {
            ElementType::Special { ref kind } => {
                let tag_name = self.tag_name(kind)?;
                element.apply_tags([tag_name.into()]);
            }
            ElementType::Multiline {
                kind: MultilineKind::Code { .. },
            } => {
                let outer_tag = self.tag_name(&SpecialKind::CodeBlockContainer)?;
                let inner_tag = self.tag_name(&SpecialKind::Code)?;
                element.apply_tags([outer_tag.into(), inner_tag.into()]);
            }
            _ => (),
        }

        element.content = self.transform_content(element.content)?;
        Ok(element)
    }
}
