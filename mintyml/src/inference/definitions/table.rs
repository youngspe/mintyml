use crate::inference::engine::{define_tags, when::*, Infer, TagDefinition};

#[non_exhaustive]
#[derive(Debug)]
pub struct TableInfer {}

impl<'cfg> Infer<'cfg> for TableInfer {
    fn define_tags(&self) -> impl TagDefinition<'cfg> {
        define_tags().default("tr")
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct RowInfer {}

impl<'cfg> Infer<'cfg> for RowInfer {
    fn define_tags(&self) -> impl TagDefinition<'cfg> {
        define_tags()
            .when(child_of(child_of(tag("thead"))), "th")
            .default("td")
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct ColGroupInfer {}

impl<'cfg> Infer<'cfg> for ColGroupInfer {
    fn define_tags(&self) -> impl TagDefinition<'cfg> {
        define_tags().default("col")
    }
}
