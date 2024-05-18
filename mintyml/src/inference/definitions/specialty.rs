use crate::inference::engine::{when::*, Infer, MethodDefinition, TagDefinition};

use super::{common_methods, PhrasingInfer, StandardInfer};

#[non_exhaustive]
#[derive(Debug)]
pub struct RootInfer {}

impl<'cfg> Infer<'cfg> for RootInfer {
    fn with_tags(&self, definition: impl TagDefinition<'cfg>) -> impl TagDefinition<'cfg> {
        definition
            .when(first() & paragraph(), "title")
            .when(first(), "head")
            .default("body")
    }

    fn with_methods(&self, definition: impl MethodDefinition<'cfg>) -> impl MethodDefinition<'cfg> {
        definition
            .append(common_methods())
            .when(line() | paragraph(), &PhrasingInfer {})
            .default(&StandardInfer {})
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct ListInfer {}

impl<'cfg> Infer<'cfg> for ListInfer {
    fn with_tags(&self, definition: impl TagDefinition<'cfg>) -> impl TagDefinition<'cfg> {
        definition.default("li")
    }

    fn with_methods(&self, definition: impl MethodDefinition<'cfg>) -> impl MethodDefinition<'cfg> {
        definition
            .append(common_methods())
            .when(line() | paragraph(), &PhrasingInfer {})
            .default(&StandardInfer {})
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct DescriptionListInfer {}

impl<'cfg> Infer<'cfg> for DescriptionListInfer {
    fn with_tags(&self, definition: impl TagDefinition<'cfg>) -> impl TagDefinition<'cfg> {
        definition.when(line(), "dt").default("dd")
    }

    fn with_methods(&self, definition: impl MethodDefinition<'cfg>) -> impl MethodDefinition<'cfg> {
        definition
            .append(common_methods())
            .when(line() | paragraph(), &PhrasingInfer {})
            .default(&StandardInfer {})
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct OptGroupInfer {}

impl<'cfg> Infer<'cfg> for OptGroupInfer {
    fn with_tags(&self, definition: impl TagDefinition<'cfg>) -> impl TagDefinition<'cfg> {
        definition.default("option")
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct SelectInfer {}

impl<'cfg> Infer<'cfg> for SelectInfer {
    fn with_tags(&self, definition: impl TagDefinition<'cfg>) -> impl TagDefinition<'cfg> {
        definition.when(block(), "optgroup").default("option")
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct MapInfer {}

impl<'cfg> Infer<'cfg> for MapInfer {
    fn with_tags(&self, definition: impl TagDefinition<'cfg>) -> impl TagDefinition<'cfg> {
        definition.default("area")
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct DetailsInfer {}

impl<'cfg> Infer<'cfg> for DetailsInfer {
    fn with_tags(&self, definition: impl TagDefinition<'cfg>) -> impl TagDefinition<'cfg> {
        definition
            .when(first(), "summary")
            .apply_from(&StandardInfer {})
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct LabelInfer {}

impl<'cfg> Infer<'cfg> for LabelInfer {
    fn with_tags(&self, definition: impl TagDefinition<'cfg>) -> impl TagDefinition<'cfg> {
        definition
            .when(line(), "input")
            .apply_from(&StandardInfer {})
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct FieldSetInfer {}

impl<'cfg> Infer<'cfg> for FieldSetInfer {
    fn with_tags(&self, definition: impl TagDefinition<'cfg>) -> impl TagDefinition<'cfg> {
        definition
            .when(first() & paragraph(), "legend")
            .apply_from(&StandardInfer {})
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct PictureInfer {}

impl<'cfg> Infer<'cfg> for PictureInfer {
    fn with_tags(&self, definition: impl TagDefinition<'cfg>) -> impl TagDefinition<'cfg> {
        definition
            .when(line() & last(), "img")
            .when(line(), "source")
    }
}
