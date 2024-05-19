use alloc::{borrow::Cow, vec};
use gramma::parse::LocationRange;

use crate::{
    document::{Attribute, Document, Element, NodeType},
    error::{Errors, InternalResult},
    OutputConfig,
};

use self::apply_special_tags::apply_special_tags;

mod apply_special_tags;
// TODO: restore this
// mod complete_page;
// mod metadata;

struct TransformError;
type TransformResult<T = ()> = Result<T, TransformError>;

/// If `lang` contains a value, assign it to the `lang` attribute of each top-level element.
fn apply_lang<'src>(document: &mut Document<'src>, lang: &Option<Cow<'src, str>>) {
    if let Some(ref lang) = lang {
        for node in &mut document.content.nodes {
            if let NodeType::Element {
                value: Element { selectors, .. },
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
    config: &OutputConfig<'cfg>,
    errors: &mut Errors,
) -> InternalResult<Document<'cfg>> {
    document = apply_special_tags(document, config, errors)?;

    // TODO: restore this:
    // if config.complete_page.unwrap_or(false) {
    //     transform::complete_page::complete_page(&mut document, &config);
    // }

    // transform::infer_elements::infer_elements(&mut document, &config.special_tags);
    // transform::apply_lang(&mut document, &config.lang);

    // if let Some(ref metadata) = config.metadata {
    //     transform::metadata::add_metadata(&mut document, metadata);
    // }

    apply_lang(&mut document, &config.lang);
    Ok(document)
}
