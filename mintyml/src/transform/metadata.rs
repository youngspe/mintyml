use alloc::{string::ToString, vec, vec::Vec};
use core::{marker::PhantomData, mem};
use gramma::parse::{Location, LocationRange};

use crate::{
    config::MetadataConfig,
    document::{
        Attribute, Comment, Document, Element, ElementType, Node, NodeType, Selector, SelectorItem,
        Text, TextLike, TextSlice,
    },
    error::InternalResult,
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
    pub const MULTILINE: &str = "mty:multiline";
}
mod tag {
    pub const COMMENT: &str = "mty:comment";
    pub const TEXT: &str = "mty:text";
    pub const ELEMENT: &str = "mty::element";
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
struct AttributeFactory<'cfg> {
    out: Vec<Attribute<'cfg>>,
    location: Location,
}

impl<'cfg> AttributeFactory<'cfg> {
    fn range(&self) -> LocationRange {
        LocationRange {
            start: self.location,
            end: self.location,
        }
    }
    fn add_some(
        &mut self,
        name: &'cfg str,
        value: Option<impl Into<TextSlice<'cfg>>>,
    ) -> InternalResult<&mut Self> {
        self.out.push(Attribute {
            range: self.range(),
            name: name.into(),
            value: value.map(Into::into),
        });
        Ok(self)
    }
    fn add(
        &mut self,
        name: &'cfg str,
        value: impl Into<TextSlice<'cfg>>,
    ) -> InternalResult<&mut Self> {
        self.add_some(name, Some(value))
    }

    fn add_location(
        &mut self,
        name: &'cfg str,
        value: impl Into<Option<Location>>,
    ) -> InternalResult<&mut Self> {
        self.add_some(name, value.into().map(|v| v.position.to_string()))
    }

    fn add_range(
        &mut self,
        start_name: &'cfg str,
        end_name: &'cfg str,
        value: impl Into<Option<LocationRange>>,
    ) -> InternalResult<&mut Self> {
        match value.into() {
            Some(value) if value != LocationRange::INVALID => self
                .add_location(start_name, value.start)?
                .add_location(end_name, value.end),
            _ => Ok(self),
        }
    }

    fn add_bool(
        &mut self,
        name: &'cfg str,
        value: impl Into<Option<bool>>,
    ) -> InternalResult<&mut Self> {
        self.add_some(name, value.into().map(bool_string))
    }

    fn build_selector_item(self) -> InternalResult<Option<SelectorItem<'cfg>>> {
        if self.out.is_empty() {
            return Ok(None);
        }

        Ok(Some(SelectorItem::Attributes {
            range: self.range(),
            attributes: self.out,
        }))
    }

    fn finish(self, target: &mut Element<'cfg>) -> InternalResult {
        let location = self.location;
        let Some(item) = self.build_selector_item()? else {
            return Ok(());
        };

        match target.selectors.first_mut() {
            Some(first) => {
                first.items.push(item);
            }
            None => {
                target.selectors.push(Selector {
                    items: vec![item],
                    ..Selector::empty(location).with_tag(tag::ELEMENT)
                });
            }
        }

        Ok(())
    }
}

#[derive(Default)]
struct TransformContext<'cx, 'cfg> {
    _lt: PhantomData<(&'cx (), &'cfg ())>,
}

impl<'cx, 'cfg> TransformContext<'cx, 'cfg> {
    fn attrs(&self, location: Location) -> AttributeFactory<'cfg> {
        AttributeFactory {
            out: default(),
            location,
        }
    }

    fn handle_element(
        &mut self,
        range: LocationRange,
        element: &mut Element<'cfg>,
        root: bool,
    ) -> InternalResult {
        let mut attrs = self.attrs(range.start);
        if root {
            attrs.add(attr::XMLNS.into(), XMLNS_URI)?;
        }

        attrs.add_range(attr::START, attr::END, range)?.add_range(
            attr::CONTENT_START,
            attr::CONTENT_END,
            element.content.range,
        )?;

        attrs.finish(element)?;
        Ok(())
    }
    fn process_node(
        &mut self,
        mut node: Node<'cfg>,
        options: &MetadataConfig,
        root: bool,
    ) -> InternalResult<Node<'cfg>> {
        let range = node.range;
        match node.node_type {
            NodeType::Element { ref mut element } => {
                if !element.selectors.is_empty() {
                    self.handle_element(range, element, root)?;
                }
                element.content.nodes = mem::take(&mut element.content.nodes)
                    .into_iter()
                    .map(|n| self.process_node(n, options, false))
                    .collect::<Result<_, _>>()?;
            }
            NodeType::TextLike {
                text_like: TextLike::Text { ref text },
            } if options.elements && !text.raw => {
                let mut element = Element::new(range, ElementType::Unknown {}).with_tag(tag::TEXT);
                let mut attrs = self.attrs(range.start);

                attrs
                    .add_bool(attr::VERBATIM, !text.unescape_in)?
                    .add_bool(attr::RAW, !text.escape_out)?
                    .add_bool(attr::MULTILINE, text.multiline.then_some(text.multiline))?;

                attrs.finish(&mut element)?;
                element.content.nodes.push(node);
                self.handle_element(range, &mut element, root)?;
                node = element.into();
            }
            NodeType::TextLike {
                text_like: TextLike::Comment { comment },
            } if options.elements => {
                let Comment::Tag { slice } = comment;
                let mut element =
                    Element::new(range, ElementType::Unknown {}).with_tag(tag::COMMENT);

                element.content.nodes.push(Node {
                    range,
                    node_type: NodeType::TextLike {
                        text_like: TextLike::Text {
                            text: Text {
                                slice,
                                escape_out: true,
                                ..Text::default()
                            },
                        },
                    },
                });
                self.handle_element(range, &mut element, root)?;
                node = element.into();
            }
            NodeType::TextLike {
                text_like: TextLike::Comment { .. } | TextLike::Space { .. } | TextLike::Text { .. },
            } => {}
        }
        Ok(node)
    }
}

pub fn add_metadata<'cfg>(
    mut target: Document<'cfg>,
    options: &MetadataConfig,
) -> InternalResult<Document<'cfg>> {
    let mut cx = TransformContext { _lt: PhantomData };

    target.content.nodes = target
        .content
        .nodes
        .into_iter()
        .map(|n| cx.process_node(n, options, true))
        .collect::<Result<_, _>>()?;

    Ok(target)
}
