use alloc::{borrow::Cow, vec};
use gramma::parse::LocationRange;

use crate::{
    document::{Attribute, Document, Element, NodeType},
    error::{Errors, InternalResult},
    OutputConfig,
};

use self::apply_special_tags::apply_special_tags;

mod apply_special_tags;
mod complete_page;
mod metadata;

/// If `lang` contains a value, assign it to the `lang` attribute of each top-level element.
fn apply_lang<'src>(document: &mut Document<'src>, lang: &Option<Cow<'src, str>>) {
    if let Some(ref lang) = lang {
        for node in &mut document.content.nodes {
            if let NodeType::Element {
                element: Element { selectors, .. },
            } = &mut node.node_type
            {
                if let Some(selector) = selectors.iter_mut().find(|s| !s.uninferred()) {
                    let range = LocationRange {
                        start: selector.range.end,
                        end: selector.range.end,
                    };
                    selector
                        .items
                        .push(crate::document::SelectorItem::Attributes {
                            range,
                            attributes: vec![Attribute {
                                range,
                                name: "lang".into(),
                                value: Some(lang.clone().into()),
                            }],
                        })
                }
            }
        }
    }
}

pub fn transform_document<'cfg>(
    mut document: Document<'cfg>,
    src: &'cfg str,
    config: &OutputConfig<'cfg>,
    errors: &mut Errors,
) -> InternalResult<Document<'cfg>> {
    document = apply_special_tags(document, config, errors)?;

    if config.complete_page.unwrap_or(false) {
        document = complete_page::complete_page(document, src)?;
    }

    crate::inference::engine::infer(src, &mut document.content);

    if let Some(ref metadata) = config.metadata {
        document = metadata::add_metadata(document, metadata)?;
    }

    apply_lang(&mut document, &config.lang);
    Ok(document)
}
