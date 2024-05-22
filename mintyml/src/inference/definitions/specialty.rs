use crate::inference::engine::{define_tags, when::*, Infer, TagDefinition};

use super::StandardInfer;

#[non_exhaustive]
#[derive(Debug)]
pub struct RootInfer {}

impl<'cfg> Infer<'cfg> for RootInfer {
    fn define_tags(&self) -> impl TagDefinition<'cfg> {
        define_tags()
            .when(first() & paragraph(), "title")
            .when(first(), "head")
            .default("body")
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct ListInfer {}

impl<'cfg> Infer<'cfg> for ListInfer {
    fn define_tags(&self) -> impl TagDefinition<'cfg> {
        define_tags().default("li")
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct DescriptionListInfer {}

impl<'cfg> Infer<'cfg> for DescriptionListInfer {
    fn define_tags(&self) -> impl TagDefinition<'cfg> {
        define_tags().when(line(), "dt").default("dd")
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct OptGroupInfer {}

impl<'cfg> Infer<'cfg> for OptGroupInfer {
    fn define_tags(&self) -> impl TagDefinition<'cfg> {
        define_tags().default("option")
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct SelectInfer {}

impl<'cfg> Infer<'cfg> for SelectInfer {
    fn define_tags(&self) -> impl TagDefinition<'cfg> {
        define_tags().when(block(), "optgroup").default("option")
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct MapInfer {}

impl<'cfg> Infer<'cfg> for MapInfer {
    fn define_tags(&self) -> impl TagDefinition<'cfg> {
        define_tags().default("area")
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct DetailsInfer {}

impl<'cfg> Infer<'cfg> for DetailsInfer {
    fn define_tags(&self) -> impl TagDefinition<'cfg> {
        define_tags()
            .when(first(), "summary")
            .apply_from(&StandardInfer {})
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct LabelInfer {}

impl<'cfg> Infer<'cfg> for LabelInfer {
    fn define_tags(&self) -> impl TagDefinition<'cfg> {
        define_tags()
            .when(line(), "input")
            .apply_from(&StandardInfer {})
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct FieldSetInfer {}

impl<'cfg> Infer<'cfg> for FieldSetInfer {
    fn define_tags(&self) -> impl TagDefinition<'cfg> {
        define_tags()
            .when(first() & paragraph(), "legend")
            .apply_from(&StandardInfer {})
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct PictureInfer {}

impl<'cfg> Infer<'cfg> for PictureInfer {
    fn define_tags(&self) -> impl TagDefinition<'cfg> {
        define_tags()
            .when(line() & last(), "img")
            .when(line(), "source")
    }
}
