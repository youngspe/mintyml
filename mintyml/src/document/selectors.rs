use alloc::vec::Vec;

use gramma::parse::{Location, LocationRange};

use crate::{
    ast::{self, AttributeAssignment},
    error::{ItemType, UnclosedDelimiterKind},
    utils::default,
};

use super::{BuildContext, BuildResult, TextSlice};

#[derive(Default)]
#[non_exhaustive]
pub struct Selector<'cfg> {
    pub range: LocationRange,
    pub tag: Tag<'cfg>,
    pub items: Vec<SelectorItem<'cfg>>,
}

impl<'cfg> Selector<'cfg> {
    pub fn uninferred(&self) -> bool {
        match self.tag {
            Tag::Explicit { .. } => false,
            Tag::Implicit { .. } | Tag::Wildcard { .. } => true,
        }
    }

    pub fn class_names(&self) -> impl Iterator<Item = &'_ TextSlice<'cfg>> {
        self.items.iter().filter_map(|item| match item {
            SelectorItem::Class { value, .. } => Some(value),
            _ => None,
        })
    }

    pub fn id(&self) -> Option<&TextSlice<'cfg>> {
        self.items.iter().find_map(|item| match item {
            SelectorItem::Id { value, .. } => Some(value),
            _ => None,
        })
    }

    pub fn attributes(
        &self,
    ) -> impl Iterator<Item = (&'_ TextSlice<'cfg>, Option<&'_ TextSlice<'cfg>>)> {
        self.items
            .iter()
            .flat_map(|item| match item {
                SelectorItem::Attributes { attributes, .. } => &attributes[..],
                _ => &[],
            })
            .map(|a| (&a.name, a.value.as_ref()))
    }
}

impl<'cfg> Selector<'cfg> {
    pub(crate) fn empty(location: Location) -> Self {
        Self {
            range: LocationRange {
                start: location,
                end: location,
            },
            ..default()
        }
    }

    pub(crate) fn with_tag(self, tag: impl Into<TextSlice<'cfg>>) -> Self {
        Self {
            tag: TextSlice::into(tag.into()),
            ..self
        }
    }
}

#[non_exhaustive]
pub enum Tag<'cfg> {
    #[non_exhaustive]
    Explicit { value: TextSlice<'cfg> },
    #[non_exhaustive]
    Implicit {},
    #[non_exhaustive]
    Wildcard { range: LocationRange },
}

impl<'cfg> Tag<'cfg> {
    pub fn name(&self) -> Option<&TextSlice<'cfg>> {
        match self {
            Self::Explicit { value } => Some(value),
            _ => None,
        }
    }
}

impl<'cfg> From<TextSlice<'cfg>> for Tag<'cfg> {
    fn from(value: TextSlice<'cfg>) -> Self {
        Self::Explicit { value }
    }
}

impl<'cfg> Default for Tag<'cfg> {
    fn default() -> Self {
        Self::Implicit {}
    }
}

#[non_exhaustive]
pub enum SelectorItem<'cfg> {
    #[non_exhaustive]
    Id {
        hash: LocationRange,
        value: TextSlice<'cfg>,
    },
    #[non_exhaustive]
    Class {
        dot: LocationRange,
        value: TextSlice<'cfg>,
    },
    #[non_exhaustive]
    Attributes {
        range: LocationRange,
        attributes: Vec<Attribute<'cfg>>,
    },
}

#[non_exhaustive]
pub struct Attribute<'cfg> {
    range: LocationRange,
    name: TextSlice<'cfg>,
    value: Option<TextSlice<'cfg>>,
}

impl<'cfg> BuildContext<'cfg> {
    fn build_attribute(
        &mut self,
        &ast::Attribute {
            start,
            ref name,
            ref assignment,
            end,
        }: &ast::Attribute,
    ) -> BuildResult<Attribute<'cfg>> {
        let value_range = match assignment {
            Some(AttributeAssignment { value, .. }) => match value {
                ast::AttributeValue::Unquoted { value } => Some(value.range),
                ast::AttributeValue::Quoted { value } => {
                    let mut range = value.range;
                    range.start += 1;
                    range.end -= 1;
                    Some(range)
                }
            },
            None => None,
        };
        let name = self.slice(name.range);

        let value = value_range.map(|r| self.slice(r));

        Ok(Attribute {
            range: LocationRange { start, end },
            name,
            value,
        })
    }

    fn build_attribute_list(
        &mut self,
        &ast::AttributeSelector {
            start,
            ref l_bracket,
            ref parts,
            ref r_bracket,
            end,
        }: &ast::AttributeSelector,
    ) -> BuildResult<SelectorItem<'cfg>> {
        if r_bracket.is_none() {
            self.unclosed(l_bracket.range, UnclosedDelimiterKind::AttributeList {})?;
        }
        Ok(SelectorItem::Attributes {
            range: LocationRange { start, end },
            attributes: Result::from_iter(parts.iter().map(|a| self.build_attribute(a)))?,
        })
    }

    fn build_tag(&mut self, ast: &Option<ast::ElementSelector>) -> BuildResult<Tag<'cfg>> {
        Ok(match ast {
            Some(ast::ElementSelector::Name { name }) => Tag::Explicit {
                value: self.slice(name.range),
            },
            Some(ast::ElementSelector::Star { star }) => Tag::Wildcard { range: star.range },
            None => Tag::Implicit {},
        })
    }

    fn build_class_like(
        &mut self,
        ast: &ast::ClassLike,
    ) -> BuildResult<Option<SelectorItem<'cfg>>> {
        Ok(match ast {
            ast::ClassLike::Class { value } => Some(SelectorItem::Class {
                dot: value.dot.range,
                value: self.slice(value.ident.range),
            }),
            ast::ClassLike::Id { value } => Some(SelectorItem::Id {
                hash: value.hash.range,
                value: self.slice(value.ident.range),
            }),
            &ast::ClassLike::Invalid { range } => {
                self.invalid(range, ItemType::Selector {})?;
                None
            }
        })
    }

    fn extend_class_like(
        &mut self,
        items: &mut Vec<SelectorItem<'cfg>>,
        ast: &Vec<ast::ClassLike>,
    ) -> BuildResult {
        for cl in ast {
            items.extend(self.build_class_like(cl)?);
        }
        Ok(())
    }

    pub fn build_selector(
        &mut self,
        &ast::Selector {
            start,
            first:
                ast::SelectorStart {
                    ref element,
                    class_like: ref first,
                },
            ref segments,
            end,
        }: &ast::Selector,
    ) -> BuildResult<Selector<'cfg>> {
        let range = LocationRange { start, end };
        let element = self.build_tag(element)?;

        let est_item_count = first.len()
            + segments
                .iter()
                .map(|s| 1 + s.class_like.len())
                .sum::<usize>();
        let mut items = Vec::with_capacity(est_item_count);
        self.extend_class_like(&mut items, first)?;

        for &ast::SelectorSegment {
            ref attributes,
            ref class_like,
        } in segments
        {
            items.push(self.build_attribute_list(attributes)?);
            self.extend_class_like(&mut items, class_like)?;
        }

        Ok(Selector {
            range,
            tag: element,
            items,
        })
    }
}
