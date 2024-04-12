use crate::document::{Attribute, Document, Element, NodeType, Src};

pub mod complete_page;
pub mod infer_elements;

/// If `lang` contains a value, assign it to the `lang` attribute of each top-level element.
pub fn apply_lang<'src>(document: &mut Document<'src>, lang: &Option<Src<'src>>) {
    if let Some(ref lang) = lang {
        for node in &mut document.nodes {
            if let NodeType::Element(Element { selector, .. }) = &mut node.node_type {
                selector.attributes.push(Attribute {
                    name: "lang".into(),
                    value: Some(lang.clone()),
                })
            }
        }
    }
}
