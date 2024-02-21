mod utils;

use core::{
    fmt::{self, Write},
    mem,
};

use alloc::{
    borrow::{Cow, ToOwned},
    string::String,
};

use crate::{
    escape::{unescape_parts, UnescapePart},
    ir::{
        Document, Element, ElementDelimiter, ElementKind, Node, Selector, SelectorElement, Space,
        SpecialKind, Text,
    },
    utils::default,
};

use self::utils::trim_multiline;

#[non_exhaustive]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContentMode {
    #[default]
    Block,
    Inline,
}

#[non_exhaustive]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentInference<'lt> {
    pub mode: ContentMode,
    pub block: &'lt str,
    pub line: &'lt str,
    pub inline: &'lt str,
    pub paragraph: Option<&'lt str>,
    pub is_raw: bool,
}

impl<'lt> ContentInference<'lt> {
    const DEFAULT: ContentInference<'static> = ContentInference {
        mode: ContentMode::Block,
        block: "div",
        line: "p",
        inline: "span",
        paragraph: None,
        is_raw: false,
    };
}

fn get_inference<'cx>(tag: &str, ci: ContentInference<'cx>) -> ContentInference<'cx> {
    const SECTION: ContentInference = ContentInference {
        mode: ContentMode::Block,
        block: "div",
        line: "p",
        inline: "span",
        paragraph: Some("p"),
        ..ContentInference::DEFAULT
    };
    const PARAGRAPH: ContentInference = ContentInference {
        mode: ContentMode::Inline,
        block: "span",
        line: "span",
        inline: "span",
        paragraph: None,
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
            block: "li",
            line: "li",
            inline: "li",
            paragraph: Some("li"),
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
            block: "option",
            line: "option",
            inline: "option",
            paragraph: Some("option"),
            ..ci
        },
        "table" | "tbody" | "thead" | "tfoot" => ContentInference {
            mode: ContentMode::Block,
            block: "tr",
            line: "tr",
            inline: "tr",
            paragraph: Some("tr"),
            ..default()
        },
        "tr" => ContentInference {
            block: "td",
            line: "td",
            inline: "td",
            paragraph: Some("td"),
            ..SECTION
        },
        "colgroup" => ContentInference {
            block: "col",
            line: "col",
            inline: "col",
            ..default()
        },
        "dl" => ContentInference {
            mode: ContentMode::Block,
            block: "dd",
            line: "dt",
            inline: "dt",
            paragraph: Some("dd"),
            ..ci
        },
        "map" => ContentInference {
            mode: ContentMode::Block,
            block: "area",
            line: "area",
            inline: "area",
            ..default()
        },
        "details" => match ci.mode {
            ContentMode::Inline => ContentInference {
                mode: ContentMode::Block,
                block: "summary",
                line: "summary",
                inline: "summary",
                ..PARAGRAPH
            },
            _ => ContentInference {
                mode: ContentMode::Block,
                block: "summary",
                line: "summary",
                inline: "summary",
                ..ci
            },
        },
        "script" | "style" => ContentInference {
            is_raw: true,
            ..Default::default()
        },
        "label" => ContentInference {
            line: "input",
            ..PARAGRAPH
        },
        _ => ci,
    }
}

fn is_void(tag: &str) -> bool {
    match tag {
        "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input" | "link" | "meta"
        | "param" | "source" | "track" | "wbr" => true,
        _ => false,
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct TagInfo<'cx> {
    pub ci: ContentInference<'cx>,
    pub is_void: bool,
}

#[non_exhaustive]
#[derive(Debug, Default, Clone)]
pub struct OutputConfig {
    pub indent: Option<Cow<'static, str>>,
    pub xml: Option<bool>,
    pub special_tags: SpecialTagConfig,
}

#[non_exhaustive]
#[derive(Debug, Default, Clone)]
pub struct SpecialTagConfig {
    pub emphasis: Option<Cow<'static, str>>,
    pub strong: Option<Cow<'static, str>>,
    pub underline: Option<Cow<'static, str>>,
    pub strike: Option<Cow<'static, str>>,
    pub quote: Option<Cow<'static, str>>,
    pub code: Option<Cow<'static, str>>,
    pub code_block_container: Option<Cow<'static, str>>,
}

impl OutputConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }

    pub fn indent(self, indent: impl Into<Cow<'static, str>>) -> Self {
        self.update(|c| c.indent = Some(indent.into()))
    }

    pub fn xml(self, xml: bool) -> Self {
        self.update(|c| c.xml = Some(xml))
    }

    pub fn emphasis_tag(self, tag: impl Into<Cow<'static, str>>) -> Self {
        self.update(|c| c.special_tags.emphasis = Some(tag.into()))
    }

    pub fn strong_tag(self, tag: impl Into<Cow<'static, str>>) -> Self {
        self.update(|c| c.special_tags.strong = Some(tag.into()))
    }

    pub fn underline_tag(self, tag: impl Into<Cow<'static, str>>) -> Self {
        self.update(|c| c.special_tags.underline = Some(tag.into()))
    }

    pub fn strike_tag(self, tag: impl Into<Cow<'static, str>>) -> Self {
        self.update(|c| c.special_tags.strike = Some(tag.into()))
    }

    pub fn quote_tag(self, tag: impl Into<Cow<'static, str>>) -> Self {
        self.update(|c| c.special_tags.quote = Some(tag.into()))
    }

    pub fn code_tag(self, tag: impl Into<Cow<'static, str>>) -> Self {
        self.update(|c| c.special_tags.code = Some(tag.into()))
    }

    pub fn code_block_container_tag(self, tag: impl Into<Cow<'static, str>>) -> Self {
        self.update(|c| c.special_tags.code_block_container = Some(tag.into()))
    }
}

struct OutputContext<'cx, Out> {
    string_buf: String,
    out: &'cx mut Out,
    config: OutputConfig,
    indent_level: usize,
    ci: ContentInference<'cx>,
    element_kind: ElementKind,
    first_line: bool,
    follows_space: bool,
}

trait Escape {
    fn get_escape(&self, ch: char) -> Option<EscapeKind>;
    fn write_escape(&self, esc: EscapeKind, out: &mut impl Write) -> fmt::Result;
}

enum EscapeKind {
    Number(u32),
    Special(&'static str),
}

#[derive(Default)]
struct HtmlEscape<const QUOTE: bool> {}

impl<const QUOTE: bool> Escape for HtmlEscape<QUOTE> {
    fn get_escape(&self, ch: char) -> Option<EscapeKind> {
        match ch {
            '&' => Some(EscapeKind::Special("&amp;")),
            '<' => Some(EscapeKind::Special("&lt;")),
            '>' => Some(EscapeKind::Special("&gt;")),
            '\t' => Some(EscapeKind::Special("&Tab;")),
            '\n' => Some(EscapeKind::Special("&NewLine;")),
            '"' if QUOTE => Some(EscapeKind::Special("&quot;")),
            '\u{0}'..='\u{1f}' | '\u{7f}'..='\u{9f}' => Some(EscapeKind::Number(ch as u32)),
            _ => None,
        }
    }

    fn write_escape(&self, esc: EscapeKind, out: &mut impl Write) -> fmt::Result {
        match esc {
            EscapeKind::Number(num) => write!(out, "&#{num};"),
            EscapeKind::Special(s) => out.write_str(s),
        }
    }
}

#[derive(Default)]
struct XmlEscape<const QUOTE: bool> {}

impl<const QUOTE: bool> Escape for XmlEscape<QUOTE> {
    fn get_escape(&self, ch: char) -> Option<EscapeKind> {
        match ch {
            '&' => Some(EscapeKind::Special("&amp;")),
            '<' => Some(EscapeKind::Special("&lt;")),
            '>' => Some(EscapeKind::Special("&gt;")),
            '"' if QUOTE => Some(EscapeKind::Special("&quot;")),
            '\u{0}'..='\u{1f}' | '\u{7f}'..='\u{9f}' => Some(EscapeKind::Number(ch as u32)),
            _ => None,
        }
    }

    fn write_escape(&self, esc: EscapeKind, out: &mut impl Write) -> fmt::Result {
        match esc {
            EscapeKind::Number(num) => write!(out, "&#{num};"),
            EscapeKind::Special(s) => out.write_str(s),
        }
    }
}

struct EscapeWriter<W, E> {
    inner: W,
    escape: E,
}

impl<W, E> EscapeWriter<W, E> {
    fn new(inner: W) -> Self
    where
        E: Default,
    {
        Self {
            inner,
            escape: default(),
        }
    }
}

impl<W: Write, E: Escape> Write for EscapeWriter<W, E> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let last_match = s
            .char_indices()
            .filter_map(|(idx, ch)| Some((idx, ch, self.escape.get_escape(ch)?)))
            .try_fold(0, |last, (idx, ch, esc)| {
                Ok({
                    let slice = s.get(last..idx).unwrap_or_default();
                    if !slice.is_empty() {
                        self.inner.write_str(slice)?;
                    }
                    self.escape.write_escape(esc, &mut self.inner)?;
                    idx + ch.len_utf8()
                })
            })?;

        let rest = s.get(last_match..).unwrap_or_default();
        if !rest.is_empty() {
            self.inner.write_str(rest)?;
        }
        Ok(())
    }

    fn write_char(&mut self, ch: char) -> fmt::Result {
        match self.escape.get_escape(ch) {
            Some(esc) => self.escape.write_escape(esc, &mut self.inner),
            None => self.inner.write_char(ch),
        }
    }
}

fn write_unescaped(src: &str, mut out: impl Write) -> fmt::Result {
    unescape_parts(src, None).try_for_each(|part| match part.map_err(|_| default())? {
        UnescapePart::Slice(s) => out.write_str(s),
        UnescapePart::Char(c) => out.write_char(c),
    })
}

impl<'cx, Out> OutputContext<'cx, Out>
where
    Out: Write,
{
    fn is_xml(&self) -> bool {
        self.config.xml == Some(true)
    }

    fn write_escape_unescape(&mut self, src: &str, quote: bool) -> OutputResult {
        if self.is_xml() {
            if quote {
                write_unescaped(src, EscapeWriter::<_, XmlEscape<true>>::new(&mut *self.out))
            } else {
                write_unescaped(
                    src,
                    EscapeWriter::<_, XmlEscape<false>>::new(&mut *self.out),
                )
            }
        } else {
            if quote {
                write_unescaped(
                    src,
                    EscapeWriter::<_, HtmlEscape<true>>::new(&mut *self.out),
                )
            } else {
                write_unescaped(
                    src,
                    EscapeWriter::<_, HtmlEscape<false>>::new(&mut *self.out),
                )
            }
        }
        .map_err(Into::into)
    }
    fn write_escape(&mut self, src: &str, quote: bool) -> OutputResult {
        if self.is_xml() {
            if quote {
                EscapeWriter::<_, XmlEscape<true>>::new(&mut *self.out).write_str(src)
            } else {
                EscapeWriter::<_, XmlEscape<false>>::new(&mut *self.out).write_str(src)
            }
        } else {
            if quote {
                EscapeWriter::<_, HtmlEscape<true>>::new(&mut *self.out).write_str(src)
            } else {
                EscapeWriter::<_, HtmlEscape<false>>::new(&mut *self.out).write_str(src)
            }
        }
        .map_err(Into::into)
    }

    fn write_unescape(&mut self, src: &str) -> OutputResult {
        write_unescaped(src, &mut *self.out).map_err(Into::into)
    }

    fn lowercase_str(&mut self, src: &str) -> &str {
        src.clone_into(&mut self.string_buf);
        self.string_buf.make_ascii_lowercase();
        &self.string_buf
    }

    fn get_info(&mut self, tag: &str, ci: ContentInference<'cx>) -> (&str, TagInfo<'cx>) {
        tag.clone_into(&mut self.string_buf);
        self.lowercase_str(tag);

        (
            &self.string_buf,
            TagInfo {
                ci: get_inference(tag, ci),
                is_void: !self.is_xml() && is_void(&self.string_buf),
            },
        )
    }

    fn _line(&mut self) -> OutputResult {
        if let Some(indent) = self.config.indent.as_deref() {
            if !self.first_line {
                self.out.write_char('\n')?;
            }
            self.first_line = false;
            for _ in 0..self.indent_level {
                self.out.write_str(indent)?;
            }
            self.follows_space = true;
        }

        Ok(())
    }
    fn line(&mut self) -> OutputResult {
        if !self.follows_space && self.ci.mode == ContentMode::Block {
            self._line()
        } else {
            Ok(())
        }
    }

    fn space(&mut self, space: Space) -> OutputResult {
        if !self.follows_space {
            match space {
                Space::LineEnd | Space::ParagraphEnd
                    if self.ci.mode == ContentMode::Block && self.config.indent.is_some() =>
                {
                    self._line()?
                }
                _ => self.out.write_str(" ")?,
            }
            self.follows_space = true;
        }
        Ok(())
    }

    fn indent<T>(&mut self, f: impl FnOnce(&mut Self) -> OutputResult<T>) -> OutputResult<T> {
        if self.ci.mode == ContentMode::Block {
            if self.config.indent.is_some() {
                self.indent_level += 1;
                let out = f(self);
                self.indent_level -= 1;
                out
            } else {
                f(self)
            }
        } else {
            f(self)
        }
    }

    fn in_content<T>(
        &mut self,
        mut ci: ContentInference<'cx>,
        mut element_kind: ElementKind,
        f: impl FnOnce(&mut Self) -> OutputResult<T>,
    ) -> OutputResult<T> {
        mem::swap(&mut ci, &mut self.ci);
        mem::swap(&mut element_kind, &mut self.element_kind);
        let out = f(self);
        self.ci = ci;
        self.element_kind = element_kind;
        out
    }

    fn write_open_tag<'attr>(
        &mut self,
        tag: &str,
        selector: &Selector,
        self_close: bool,
    ) -> OutputResult {
        let Selector {
            class_names,
            id,
            attributes,
            ..
        } = selector;
        write!(self.out, "<{tag}")?;

        if let Some(id) = id {
            self.out.write_str(" id=\"")?;
            self.write_escape_unescape(id, true)?;
            self.out.write_char('"')?;
        }

        if let Some((first, rest)) = class_names.split_first() {
            self.out.write_str(" class=\"")?;
            self.write_escape_unescape(first, true)?;

            for class in rest {
                self.out.write_char(' ')?;
                self.write_escape_unescape(class, true)?;
            }

            self.out.write_char('"')?;
        }

        for attr in attributes {
            self.out.write_char(' ')?;
            self.write_unescape(&attr.name)?;

            if let Some(value) = attr.value.as_ref() {
                self.out.write_str("=\"")?;
                self.write_escape_unescape(&value, true)?;
                self.out.write_char('"')?;
            }
        }

        if self_close {
            self.out.write_char('/')?;
        }
        self.out.write_char('>')?;
        self.follows_space = false;
        Ok(())
    }

    fn write_close_tag(&mut self, tag: &str) -> OutputResult {
        write!(self.out, "</{tag}>")?;
        self.follows_space = false;
        Ok(())
    }

    fn process_element(&mut self, element: &Element) -> OutputResult {
        let Some(tag) = (match (&element.selector.element, &element.kind) {
            (SelectorElement::Name(s), _) => Some(match *s {
                Cow::Borrowed(s) => Cow::Borrowed(s),
                Cow::Owned(ref s) => Cow::Borrowed(s as &str),
            }),
            (SelectorElement::Special(kind), _) => {
                let special_tags = &self.config.special_tags;
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
            (SelectorElement::Infer, ElementKind::Block) => Some(self.ci.block.into()),
            (SelectorElement::Infer, ElementKind::Inline(_)) => Some(self.ci.inline.into()),
            (SelectorElement::Infer, ElementKind::Line | ElementKind::LineBlock) => {
                Some(self.ci.line.into())
            }
            (SelectorElement::Infer, ElementKind::Paragraph) => match self.element_kind {
                ElementKind::Line
                | ElementKind::LineBlock
                | ElementKind::Inline(
                    None | Some(ElementDelimiter::Line | ElementDelimiter::LineBlock),
                ) => None,
                _ => self.ci.paragraph.map(Cow::Borrowed),
            },
        }) else {
            return self.in_content(
                ContentInference {
                    mode: ContentMode::Inline,
                    ..self.ci
                },
                element.kind,
                |this| {
                    element
                        .nodes
                        .iter()
                        .try_for_each(|node| this.process_node(node))
                },
            );
        };

        let mode = match element.kind {
            ElementKind::Block => self.ci.mode,
            _ => ContentMode::Inline,
        };
        let (_, tag_info) = self.get_info(&tag, ContentInference { mode, ..self.ci });

        if element.nodes.len() == 0 && self.is_xml() {
            self.write_open_tag(&tag, &element.selector, true)
        } else {
            self.write_open_tag(&tag, &element.selector, false)?;
            self.in_content(tag_info.ci, element.kind, |this| {
                if !element.nodes.is_empty() {
                    this.indent(|this| {
                        this.line()?;
                        element
                            .nodes
                            .iter()
                            .try_for_each(|node| this.process_node(node))
                    })?;
                    this.line()?;
                }
                if !tag_info.is_void {
                    this.write_close_tag(&tag)?;
                }
                Ok(())
            })
        }
    }

    fn process_node(&mut self, node: &Node) -> OutputResult {
        let out = match node {
            Node::Element(e) => self.process_element(e)?,
            Node::Text(text) if text.value.is_empty() => {}
            Node::Text(text) => {
                let escape = text.escape;
                let is_raw = self.ci.is_raw;
                let mut write = |value| match (escape, is_raw) {
                    (true, true) => self.write_unescape(value),
                    (true, false) => self.write_escape_unescape(value, false),
                    (false, true) => self.out.write_str(value).map_err(Into::into),
                    (false, false) => self.write_escape(value, false),
                };

                let mut last_line = &*text.value;

                if text.multiline {
                    for line in trim_multiline(&text.value) {
                        write(line)?;
                        last_line = &line
                    }
                } else {
                    write(&text.value)?;
                }

                self.follows_space = last_line.ends_with([' ', '\t']);
            }
            Node::Comment(s) => {
                self.out.write_str("<!--")?;
                self.write_escape(s, false)?;
                self.out.write_str("-->")?;
            }
            Node::Space(space) => {
                self.space(*space)?;
                self.follows_space = true
            }
        };

        Ok(out)
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub enum OutputError {
    WriteError(fmt::Error),
}

impl From<fmt::Error> for OutputError {
    fn from(value: fmt::Error) -> Self {
        OutputError::WriteError(value)
    }
}

pub type OutputResult<T = ()> = Result<T, OutputError>;

pub fn output_html_to(
    document: &Document,
    out: &mut impl Write,
    config: OutputConfig,
) -> OutputResult {
    let mut cx = OutputContext {
        string_buf: default(),
        out,
        config,
        indent_level: 0,
        ci: ContentInference {
            mode: ContentMode::Block,
            block: "div",
            line: "p",
            inline: "span",
            paragraph: Some("p"),
            is_raw: false,
        },
        element_kind: ElementKind::Block,
        first_line: true,
        follows_space: true,
    };
    document
        .nodes
        .iter()
        .try_for_each(|node| cx.process_node(node))
}

#[test]
fn output_demo() {
    let src = r#"
section {
    h1#foo.bar[
        x
    ].baz> <(foo)>

    div> { 1 }

    Hello, world!
    Click <(a[x=1]> here )> to get<!this is a comment!> started.

    div {
        a>1
    }

    > {
        This paragraph contains <(em> emphasized)>,
        <(strong> strong)>, and <(u> underlined)> text.
    }
}
section {
    line 1
    line 2
    >new paragraph
    >new paragraph
    same paragraph
    >new paragraph
}
section#list-section {
    Following is a list:

    div> foo

    ul {
        Item 1

        Item 2

        Item
        3

        #item4> Item 4

        {
            Item 5
        }

        > {
            Item 6
        }
    }
}
section {
    Following is a table:

    table {
        {
            th> Foo
            th> Bar
        }
        {
            a

            b
        }
        <( c )> <( d )>
    }
}
    "#;
    let mut out = String::new();
    output_html_to(
        &Document::parse(src).unwrap(),
        &mut out,
        OutputConfig {
            indent: Some("  ".into()),
            ..default()
        },
    )
    .unwrap();
    #[cfg(feature = "std")]
    {
        ::std::println!("{out}");
    }
}
