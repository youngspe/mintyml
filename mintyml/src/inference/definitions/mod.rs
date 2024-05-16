use super::engine::{rule, rules, when::*, DefineRules, Infer};

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

#[non_exhaustive]
#[derive(Debug)]
pub struct StandardInfer {}

impl<'cfg> Infer<'cfg> for StandardInfer {
    fn define_rules(&self) -> impl DefineRules<'cfg> {
        rules()
            .add(
                PhrasingInfer {}
                    .define_rules()
                    .into_rules()
                    .when(child_of(tag_where(contains_phrasing)))
                    .inference_method(&PhrasingInfer {}),
            )
            .add(
                rules()
                    .when(tag_where(contains_blocks))
                    .inference_method(&BlockInfer {}),
            )
            .add(BlockInfer {}.define_rules().into_rules())
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct PhrasingInfer {}

impl<'cfg> Infer<'cfg> for PhrasingInfer {
    fn define_rules(&self) -> impl DefineRules<'cfg> {
        rules()
            .inherit_inference()
            .add(rule(paragraph(), |t| t.dissolve()))
            .add(rule(line() | block(), |t| t.tag("span")))
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct BlockInfer {}

impl<'cfg> Infer<'cfg> for BlockInfer {
    fn define_rules(&self) -> impl DefineRules<'cfg> {
        rules()
            .add(
                rules()
                    .when(tag_where(contains_phrasing))
                    .inference_method(&PhrasingInfer {}),
            )
            .inherit_inference()
            .add(rule(line() | paragraph(), |t| {
                t.tag("p").inference_method(&PhrasingInfer {})
            }))
            .add(rule(block(), |t| t.tag("div")))
    }
}
