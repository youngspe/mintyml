use alloc::{borrow::Cow, string::String};

use crate::document::TextSlice;

pub trait IntoTags<'cfg, M = (), _Bound = &'cfg Self>: 'cfg {
    fn tags(self) -> impl Iterator<Item = TextSlice<'cfg>> + Clone + 'cfg;
}

impl<'cfg, It: IntoIterator + 'cfg, M> IntoTags<'cfg, &'cfg [M]> for It
where
    It::Item: IntoTags<'cfg, M>,
    It::IntoIter: Clone + 'cfg,
{
    fn tags(self) -> impl Iterator<Item = TextSlice<'cfg>> + Clone + 'cfg {
        self.into_iter().flat_map(IntoTags::tags)
    }
}

impl<'cfg> IntoTags<'cfg> for TextSlice<'cfg> {
    fn tags(self) -> impl Iterator<Item = TextSlice<'cfg>> + Clone + 'cfg {
        core::iter::once(self)
    }
}

impl<'cfg> IntoTags<'cfg> for Cow<'cfg, str> {
    fn tags(self) -> impl Iterator<Item = TextSlice<'cfg>> + Clone + 'cfg {
        core::iter::once(self.into())
    }
}

impl<'cfg> IntoTags<'cfg> for String {
    fn tags(self) -> impl Iterator<Item = TextSlice<'cfg>> + Clone + 'cfg {
        core::iter::once(self.into())
    }
}

impl<'cfg> IntoTags<'cfg> for &'cfg str {
    fn tags(self) -> impl Iterator<Item = TextSlice<'cfg>> + Clone + 'cfg {
        core::iter::once(self.into())
    }
}
