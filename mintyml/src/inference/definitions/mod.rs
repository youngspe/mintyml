mod specialty;
mod table;

use super::engine::{define_methods, define_tags, when::*, Infer, MethodDefinition, TagDefinition};

#[rustfmt::skip]
fn contains_phrasing(tag: &str) -> bool {
    matches!(tag,
        | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "span" | "b" | "i"| "q" | "s" | "u"
        | "button" | "caption" | "cite" | "code" | "data" | "dfn" | "dt" | "em" | "kbd" | "legend"
        | "mark" | "meter" | "option" | "output" | "pre" | "progress" | "samp" | "small" | "strong"
        | "sub" | "summary" | "sup" | "textarea" | "time" | "var"
    )
}

#[rustfmt::skip]
fn contains_blocks(tag: &str) -> bool {
    matches!(tag,
        | "div" | "section" | "article" | "header" | "footer" | "main" | "hgroup" | "body"
        | "dialog" | "nav" | "aside" | "template" | "figure" | "blockquote"
    )
}

#[rustfmt::skip]
fn is_void(tag: &str) -> bool {
    matches!(tag,
        | "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input" | "link" | "meta"
        | "param" | "source" | "track" | "wbr"
    )
}

fn common_methods<'cfg>() -> impl MethodDefinition<'cfg> {
    define_methods()
        .when(
            tag_where(contains_phrasing) | tag_where(is_void),
            &PhrasingInfer {},
        )
        .when(tag("html"), &specialty::RootInfer {})
        .when(tag_where(contains_blocks), &StandardInfer {})
        .when(tag_in(["ul", "ol", "menu"]), &specialty::ListInfer {})
        .when(
            tag_in(["table", "thead", "tbody", "tfoot"]),
            &table::TableInfer {},
        )
        .when(tag_in(["tr"]), &table::RowInfer {})
        .when(tag_in(["colgroup"]), &table::ColGroupInfer {})
        .when(tag("dl"), &specialty::DescriptionListInfer {})
        .when(
            tag_in(["optgroup", "datalist"]),
            &specialty::OptGroupInfer {},
        )
        .when(tag("select"), &specialty::SelectInfer {})
        .when(tag("map"), &specialty::MapInfer {})
        .when(tag("details"), &specialty::DetailsInfer {})
        .when(tag("label"), &specialty::LabelInfer {})
        .when(tag("fieldset"), &specialty::FieldSetInfer {})
        .when(tag("picture"), &specialty::PictureInfer {})
        .when(tag_in(["style", "script"]), &RawInfer {})
}

#[non_exhaustive]
#[derive(Debug)]
pub struct StandardInfer {}

impl<'cfg> Infer<'cfg> for StandardInfer {
    fn define_tags(&self) -> impl TagDefinition<'cfg> {
        define_tags().when(line() | paragraph(), "p").default("div")
    }

    fn define_methods(&self) -> impl MethodDefinition<'cfg> {
        define_methods()
            .apply(common_methods())
            .when(line() | paragraph(), &PhrasingInfer {})
            .default(&StandardInfer {})
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct PhrasingInfer {}

impl<'cfg> Infer<'cfg> for PhrasingInfer {
    fn define_tags(&self) -> impl TagDefinition<'cfg> {
        define_tags().when(paragraph(), "").default("span")
    }

    fn define_methods(&self) -> impl MethodDefinition<'cfg> {
        define_methods()
            .apply(common_methods())
            .when(
                tag_where(contains_phrasing) | tag_where(contains_blocks),
                &PhrasingInfer {},
            )
            .when(block(), &StandardInfer {})
            .default(&PhrasingInfer {})
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct RawInfer {}

impl<'cfg> Infer<'cfg> for RawInfer {
    fn define_tags(&self) -> impl TagDefinition<'cfg> {
        StandardInfer {}.define_tags()
    }
    fn root_is_raw(&self) -> bool {
        true
    }
}
