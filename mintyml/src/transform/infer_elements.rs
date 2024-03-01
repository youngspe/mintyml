use core::mem;

use alloc::{borrow::Cow, string::String};

use crate::{
    document::{
        ContentMode, Document, Element, ElementDelimiter, ElementKind, Node, SelectorElement,
        SpecialKind,
    },
    utils::{default, to_lowercase},
    SpecialTagConfig,
};

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

    fn all(tag: &'lt str) -> Self {
        Self {
            block: tag,
            line: tag,
            inline: tag,
            paragraph: Some(tag),
        }
    }
    fn all_elements(tag: &'lt str) -> Self {
        Self {
            block: tag,
            line: tag,
            inline: tag,
            paragraph: None,
        }
    }
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

fn get_inference<'cx>(
    tag: &str,
    ci: ContentInference<'cx>,
    kind: ElementKind,
) -> ContentInference<'cx> {
    const DEFAULT: ContentInference = ContentInference::DEFAULT;
    const SECTION: ContentInference = ContentInference {
        mode: ContentMode::Block,
        element: ElementInference {
            block: "div",
            line: "p",
            inline: "span",
            paragraph: Some("p"),
        },
        ..DEFAULT
    };
    const PARAGRAPH: ContentInference = ContentInference {
        mode: ContentMode::Inline,
        element: ElementInference {
            block: "span",
            line: "span",
            inline: "span",
            paragraph: None,
        },
        ..DEFAULT
    };

    fn section(ci: ContentInference) -> ContentInference {
        ContentInference {
            mode: SECTION.mode,
            first_element: SECTION.first_element,
            element: SECTION.element,
            ..ci
        }
    }
    fn paragraph(ci: ContentInference) -> ContentInference {
        ContentInference {
            mode: PARAGRAPH.mode,
            first_element: PARAGRAPH.first_element,
            element: PARAGRAPH.element,
            ..ci
        }
    }

    fn transparent(ci: ContentInference) -> ContentInference {
        match ci.mode {
            ContentMode::Inline => paragraph(ci),
            mode => ContentInference {
                mode,
                ..section(ci)
            },
        }
    }

    fn non_content(ci: ContentInference) -> ContentInference {
        ContentInference {
            mode: ci.mode,
            first_element: DEFAULT.first_element,
            element: DEFAULT.element,
            ..ci
        }
    }

    match tag {
        "div" | "section" | "article" | "header" | "footer" | "main" | "hgroup" | "body"
        | "dialog" | "nav" | "aside" | "template" | "figure" | "blockquote" => section(ci),
        "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "span" | "b" | "i" | "q" | "s" | "u"
        | "button" | "caption" | "cite" | "code" | "data" | "dfn" | "dt" | "em" | "kbd"
        | "legend" | "mark" | "meter" | "option" | "output" | "pre" | "progress" | "samp"
        | "small" | "strong" | "sub" | "summary" | "sup" | "textarea" | "time" | "var" => {
            paragraph(ci)
        }
        "ul" | "ol" | "menu" => ContentInference {
            element: ElementInference::all("li"),
            ..non_content(ci)
        },
        "html" => ContentInference {
            first_element: Some(ElementInference {
                paragraph: Some("title"),
                ..ElementInference::all_elements("head")
            }),
            element: ElementInference::all_elements("body"),
            ..non_content(ci)
        },
        "head" => ContentInference {
            mode: ContentMode::Block,
            first_element: Some(ElementInference::all("title")),
            ..non_content(ci)
        },
        "datalist" | "optgroup" | "select" => ContentInference {
            mode: ContentMode::Block,
            element: ElementInference::all("option"),
            ..non_content(ci)
        },
        "table" | "tbody" | "thead" | "tfoot" => ContentInference {
            mode: ContentMode::Block,
            element: ElementInference::all("tr"),
            ..non_content(ci)
        },
        "tr" => ContentInference {
            mode: ContentMode::Block,
            element: ElementInference::all("td"),
            ..non_content(ci)
        },
        "colgroup" => ContentInference {
            element: ElementInference::all_elements("col"),
            ..non_content(ci)
        },
        "dl" => ContentInference {
            mode: ContentMode::Block,
            element: ElementInference {
                block: "dd",
                line: "dt",
                inline: "dt",
                paragraph: Some("dd"),
            },
            ..non_content(ci)
        },
        "map" => ContentInference {
            mode: ContentMode::Block,
            element: ElementInference::all_elements("area"),
            ..non_content(ci)
        },
        "details" => ContentInference {
            first_element: Some(ElementInference::all("summary")),
            ..section(ci)
        },
        "script" | "style" => ContentInference {
            is_raw: true,
            ..non_content(ci)
        },
        "label" => ContentInference {
            element: ElementInference {
                line: "input",
                ..PARAGRAPH.element
            },
            first_element: None,
            ..ci
        },
        "fieldset" => {
            let base = section(ci);

            ContentInference {
                mode: ContentMode::Block,
                first_element: Some(ElementInference {
                    paragraph: Some("legend"),
                    ..base.element
                }),
                ..base
            }
        }
        "picture" => ContentInference {
            mode: ContentMode::Block,
            ..non_content(ci)
        },
        "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input" | "link" | "meta"
        | "param" | "source" | "track" | "wbr" => non_content(ci),
        "td" | "th" | "li" | "dd" | "figcaption" => match kind {
            ElementKind::Line
            | ElementKind::LineBlock
            | ElementKind::Paragraph
            | ElementKind::Inline(
                Some(ElementDelimiter::Line | ElementDelimiter::LineBlock) | None,
            ) => paragraph(ci),
            ElementKind::Block | ElementKind::Inline(Some(ElementDelimiter::Block)) => section(ci),
        },
        "a" | _ => transparent(ci),
    }
}

struct InferContext<'src, 'data, 'buf> {
    ci: ContentInference<'src>,
    special_tags: &'data SpecialTagConfig<'src>,
    element_kind: ElementKind,
    is_first_element: bool,
    string_buf: &'buf mut String,
}

fn get_effective_kind(element: &Element) -> ElementKind {
    fn inner(mut element: &Element, orig_kind: ElementKind) -> ElementKind {
        loop {
            match *element.nodes {
                [Node::Element(Element {
                    kind: kind @ ElementKind::Block,
                    ..
                })] => return kind,
                [Node::Element(
                    ref e @ Element {
                        kind: ElementKind::Line,
                        ..
                    },
                )] => {
                    element = e;
                }
                _ => return orig_kind,
            }
        }
    }

    match element.kind {
        kind @ (ElementKind::Line | ElementKind::Inline(Some(ElementDelimiter::Line))) => {
            inner(element, kind)
        }
        ElementKind::Inline(Some(ElementDelimiter::Block)) => ElementKind::Block,
        kind => kind,
    }
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

        let tag = match mem::take(&mut element.selector.element) {
            SelectorElement::Name(s) => Some(s),
            SelectorElement::Special(kind) => {
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

            SelectorElement::Infer => match get_effective_kind(element) {
                ElementKind::Block => Some(element_inf.block.into()),
                ElementKind::Inline(_) => Some(element_inf.inline.into()),
                ElementKind::Line | ElementKind::LineBlock => Some(element_inf.line.into()),
                ElementKind::Paragraph => match self.element_kind {
                    ElementKind::Line
                    | ElementKind::LineBlock
                    | ElementKind::Inline(
                        None | Some(ElementDelimiter::Line | ElementDelimiter::LineBlock),
                    ) => None,
                    _ => element_inf.paragraph.map(Cow::Borrowed),
                },
            },
        };

        if let Some(tag) = tag {
            ci = get_inference(
                to_lowercase(&tag, &mut self.string_buf),
                self.ci,
                element.kind,
            );
            element.selector.element = SelectorElement::Name(tag);
            self.is_first_element = false;
        } else {
            ci = self.ci;
        }

        element.is_raw = ci.is_raw;
        element.mode = ci.mode;

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
    special_tags: &'data SpecialTagConfig<'src>,
) {
    match (InferContext::<'src, 'data, '_> {
        ci: get_inference("div", default(), ElementKind::Block),
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
