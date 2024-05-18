use crate::inference::engine::{when::*, Infer, TagDefinition};

#[non_exhaustive]
#[derive(Debug)]
pub struct TableInfer {}

impl<'cfg> Infer<'cfg> for TableInfer {
    fn with_tags(&self, definition: impl TagDefinition<'cfg>) -> impl TagDefinition<'cfg> {
        definition.default("tr")
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct RowInfer {}

impl<'cfg> Infer<'cfg> for RowInfer {
    fn with_tags(&self, definition: impl TagDefinition<'cfg>) -> impl TagDefinition<'cfg> {
        definition
            .when(child_of(child_of(tag("thead"))), "th")
            .default("td")
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct ColGroupInfer {}

impl<'cfg> Infer<'cfg> for ColGroupInfer {
    fn with_tags(&self, definition: impl TagDefinition<'cfg>) -> impl TagDefinition<'cfg> {
        definition.default("col")
    }
}
