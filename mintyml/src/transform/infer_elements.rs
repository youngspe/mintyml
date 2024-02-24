use core::mem;

use alloc::{borrow::Cow, string::String};

use crate::{
    ir::{Document, Element, ElementDelimiter, ElementKind, Node, SelectorElement, SpecialKind},
    utils::{default, to_lowercase},
    SpecialTagConfig,
};

#[non_exhaustive]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContentMode {
    #[default]
    Block,
    Inline,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElementInference<'lt> {
    pub block: &'lt str,
    pub line: &'lt str,
    pub inline: &'lt str,
    pub paragraph: Option<&'lt str>,
}

impl<'lt> ElementInference<'lt> {
    const DEFAULT: Self = Self {
        block: "div",
        line: "p",
        inline: "span",
        paragraph: None,
    };
}
impl<'lt> Default for ElementInference<'lt> {
    fn default() -> Self {
        ContentInference::DEFAULT.element
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentInference<'lt> {
    pub mode: ContentMode,
    pub first_element: Option<ElementInference<'lt>>,
    pub element: ElementInference<'lt>,
    pub is_raw: bool,
}

impl<'lt> ContentInference<'lt> {
    const DEFAULT: Self = Self {
        mode: ContentMode::Block,
        first_element: None,
        element: ElementInference::DEFAULT,
        is_raw: false,
    };
}

impl<'lt> Default for ContentInference<'lt> {
    fn default() -> Self {
        Self::DEFAULT
    }
}

fn get_inference<'cx>(tag: &str, ci: ContentInference<'cx>) -> ContentInference<'cx> {
    const SECTION: ContentInference = ContentInference {
        mode: ContentMode::Block,
        element: ElementInference {
            block: "div",
            line: "p",
            inline: "span",
            paragraph: Some("p"),
        },
        ..ContentInference::DEFAULT
    };
    const PARAGRAPH: ContentInference = ContentInference {
        mode: ContentMode::Inline,
        element: ElementInference {
            block: "span",
            line: "span",
            inline: "span",
            paragraph: None,
        },
        ..ContentInference::DEFAULT
    };

    match tag {
        "div" | "section" | "article" | "header" | "footer" | "main" | "hgroup" | "body"
        | "dialog" | "nav" => SECTION,
        "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "span" | "b" | "i" | "q" | "s" | "u" | "abbr"
        | "button" | "caption" | "cite" | "code" | "data" | "dd" | "dfn" | "dt" | "em"
        | "figcaption" | "kbd" | "legend" | "mark" | "meter" | "option" | "output" | "picture"
        | "pre" | "progress" | "samp" | "small" | "strong" | "sub" | "summary" | "sup"
        | "textarea" | "td" | "th" | "time" | "var" => PARAGRAPH,
        "ul" | "ol" | "menu" => ContentInference {
            first_element: None,
            element: ElementInference {
                block: "li",
                line: "li",
                inline: "li",
                paragraph: Some("li"),
                ..default()
            },
            ..ci
        },
        "head" => ContentInference {
            mode: ContentMode::Block,
            ..default()
        },
        "li" => match ci.mode {
            ContentMode::Inline => PARAGRAPH,
            _ => ContentInference {
                mode: ci.mode,
                ..SECTION
            },
        },
        "datalist" | "optgroup" | "select" => ContentInference {
            mode: ContentMode::Block,
            element: ElementInference {
                block: "option",
                line: "option",
                inline: "option",
                paragraph: Some("option"),
            },
            first_element: None,
            ..ci
        },
        "table" | "tbody" | "thead" | "tfoot" => ContentInference {
            mode: ContentMode::Block,
            element: ElementInference {
                block: "tr",
                line: "tr",
                inline: "tr",
                paragraph: Some("tr"),
            },
            ..default()
        },
        "tr" => ContentInference {
            element: ElementInference {
                block: "td",
                line: "td",
                inline: "td",
                paragraph: Some("td"),
            },
            ..SECTION
        },
        "colgroup" => ContentInference {
            element: ElementInference {
                block: "col",
                line: "col",
                inline: "col",
                ..default()
            },
            ..default()
        },
        "dl" => ContentInference {
            mode: ContentMode::Block,
            element: ElementInference {
                block: "dd",
                line: "dt",
                inline: "dt",
                paragraph: Some("dd"),
            },
            first_element: None,
            ..ci
        },
        "map" => ContentInference {
            mode: ContentMode::Block,
            element: ElementInference {
                block: "area",
                line: "area",
                inline: "area",
                ..default()
            },
            ..default()
        },
        "details" => ContentInference {
            mode: ContentMode::Block,
            first_element: Some(ElementInference {
                block: "summary",
                line: "summary",
                inline: "summary",
                ..default()
            }),
            ..match ci.mode {
                ContentMode::Inline => PARAGRAPH,
                _ => ci,
            }
        },
        "script" | "style" => ContentInference {
            is_raw: true,
            ..Default::default()
        },
        "label" => ContentInference {
            element: ElementInference {
                line: "input",
                ..default()
            },
            ..PARAGRAPH
        },
        _ => ContentInference {
            element: match ci.mode {
                ContentMode::Inline => PARAGRAPH.element,
                _ => SECTION.element,
            },
            first_element: None,
            ..ci
        },
    }
}

struct InferContext<'src, 'data, 'buf> {
    ci: ContentInference<'src>,
    special_tags: &'data SpecialTagConfig,
    element_kind: ElementKind,
    is_first_element: bool,
    string_buf: &'buf mut String,
}

impl<'src, 'data, 'buf> InferContext<'src, 'data, 'buf> {
    pub fn process_node(&mut self, node: &mut Node<'src>) {
        match node {
            Node::Element(e) => self.process_element(e),
            _ => {}
        }
    }
    pub fn process_element(&mut self, element: &mut Element<'src>) {
        let element_inf = match &self.ci.first_element {
            Some(inf) if self.is_first_element => inf,
            _ => &self.ci.element,
        };
        let ci;

        let tag = match (mem::take(&mut element.selector.element), &element.kind) {
            (SelectorElement::Name(s), _) => Some(s),
            (SelectorElement::Special(kind), _) => {
                let special_tags = &self.special_tags;
                let (custom, default) = match kind {
                    SpecialKind::Emphasis => (&special_tags.emphasis, "em"),
                    SpecialKind::Strong => (&special_tags.strong, "strong"),
                    SpecialKind::Underline => (&special_tags.underline, "u"),
                    SpecialKind::Strike => (&special_tags.strike, "s"),
                    SpecialKind::Quote => (&special_tags.quote, "q"),
                    SpecialKind::Code => (&special_tags.code, "code"),
                    SpecialKind::CodeBlockContainer => (&special_tags.code_block_container, "pre"),
                };

                Some(custom.as_ref().cloned().unwrap_or(default.into()))
            }
            (SelectorElement::Infer, ElementKind::Block) => Some(element_inf.block.into()),
            (SelectorElement::Infer, ElementKind::Inline(_)) => Some(element_inf.inline.into()),
            (SelectorElement::Infer, ElementKind::Line | ElementKind::LineBlock) => {
                Some(element_inf.line.into())
            }
            (SelectorElement::Infer, ElementKind::Paragraph) => match self.element_kind {
                ElementKind::Line
                | ElementKind::LineBlock
                | ElementKind::Inline(
                    None | Some(ElementDelimiter::Line | ElementDelimiter::LineBlock),
                ) => None,
                _ => element_inf.paragraph.map(Cow::Borrowed),
            },
        };

        if let Some(tag) = tag {
            let mode = match element.kind {
                ElementKind::Block => self.ci.mode,
                _ => ContentMode::Inline,
            };
            ci = get_inference(
                to_lowercase(&tag, &mut self.string_buf),
                ContentInference { mode, ..self.ci },
            );
            element.selector.element = SelectorElement::Name(tag);
            self.is_first_element = false;
        } else {
            ci = ContentInference {
                mode: ContentMode::Inline,
                ..self.ci
            };
        }

        element.is_raw = ci.is_raw;

        match (InferContext {
            ci,
            special_tags: self.special_tags,
            element_kind: element.kind,
            is_first_element: true,
            string_buf: &mut self.string_buf,
        }) {
            ref mut cx => {
                for node in &mut element.nodes {
                    cx.process_node(node);
                }
            }
        }
    }
}

pub fn infer_elements<'src, 'data>(
    doc: &mut Document<'src>,
    special_tags: &'data SpecialTagConfig,
) {
    match (InferContext::<'src, 'data, '_> {
        ci: ContentInference {
            mode: ContentMode::Block,
            first_element: None,
            element: ElementInference {
                block: "div",
                line: "p",
                inline: "span",
                paragraph: Some("p"),
            },
            is_raw: false,
        },
        special_tags,
        element_kind: ElementKind::Block,
        is_first_element: true,
        string_buf: &mut default(),
    }) {
        ref mut cx => {
            for node in &mut doc.nodes {
                cx.process_node(node);
            }
        }
    }
}
