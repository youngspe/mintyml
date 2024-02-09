// pub trait ContentInference {

// }

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
    ir::{Document, Element, InferElement, Node, Selector, SelectorElement},
    utils::default,
};

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
}

fn get_inference<'cx>(tag: &str, ci: ContentInference<'cx>) -> ContentInference<'cx> {
    const SECTION: ContentInference = ContentInference {
        mode: ContentMode::Block,
        block: "div",
        line: "p",
        inline: "span",
        paragraph: Some("p"),
    };
    const PARAGRAPH: ContentInference = ContentInference {
        mode: ContentMode::Inline,
        block: "span",
        line: "span",
        inline: "span",
        paragraph: None,
    };

    match tag {
        "div" | "section" | "article" | "header" | "footer" | "main" | "hgroup" | "body"
        | "dialog" | "nav" => SECTION,
        "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "span" | "b" | "i" | "q" | "s" | "u" | "abbr"
        | "button" | "caption" | "cite" | "code" | "data" | "dd" | "details" | "dfn" | "dt"
        | "em" | "figcaption" | "head" | "kbd" | "label" | "legend" | "mark" | "meter"
        | "option" | "output" | "picture" | "pre" | "progress" | "samp" | "small" | "strong"
        | "sub" | "summary" | "sup" | "textarea" | "td" | "th" | "time" | "var" => PARAGRAPH,
        "ul" | "ol" | "menu" => ContentInference {
            block: "li",
            line: "li",
            paragraph: Some("li"),
            ..ci
        },
        "li" => match ci.mode {
            ContentMode::Inline => PARAGRAPH,
            _ => ContentInference {
                mode: ci.mode,
                ..SECTION
            },
        },
        "datalist" | "optgroup" | "select" => ContentInference {
            block: "option",
            line: "option",
            paragraph: Some("option"),
            ..ci
        },
        "table" | "tbody" | "thead" | "tfoot" => ContentInference {
            mode: ContentMode::Block,
            block: "tr",
            line: "tr",
            inline: "td",
            paragraph: Some("tr"),
        },
        "tr" => ContentInference {
            block: "td",
            line: "td",
            paragraph: Some("td"),
            ..SECTION
        },
        "colgroup" => ContentInference {
            block: "col",
            line: "col",
            inline: "col",
            ..PARAGRAPH
        },
        "dl" => ContentInference {
            block: "dd",
            line: "dd",
            paragraph: Some("dd"),
            ..ci
        },
        "map" => ContentInference {
            block: "area",
            line: "area",
            inline: "area",
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
}

struct OutputContext<'cx, Out> {
    string_buf: String,
    out: &'cx mut Out,
    config: OutputConfig,
    indent_level: usize,
    ci: ContentInference<'cx>,
    first_line: bool,
}

struct HtmlEscape<W, const QUOTE: bool> {
    inner: W,
}

enum EscapeKind {
    Number(u32),
    Special(&'static str),
}

impl<W: Write, const QUOTE: bool> HtmlEscape<W, QUOTE> {
    fn new(inner: W) -> Self {
        Self { inner }
    }

    fn get_escape(ch: char) -> Option<EscapeKind> {
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

    fn write_escape(&mut self, esc: EscapeKind) -> fmt::Result {
        match esc {
            EscapeKind::Number(num) => write!(self.inner, "&#{num};"),
            EscapeKind::Special(s) => self.inner.write_str(s),
        }
    }
}

impl<W: Write, const QUOTE: bool> Write for HtmlEscape<W, QUOTE> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let last_match = s
            .char_indices()
            .filter_map(|(idx, ch)| Some((idx, ch, Self::get_escape(ch)?)))
            .try_fold(0, |last, (idx, ch, esc)| {
                Ok({
                    let slice = s.get(last..idx).unwrap_or_default();
                    if !slice.is_empty() {
                        self.inner.write_str(slice)?;
                    }
                    self.write_escape(esc)?;
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
        match Self::get_escape(ch) {
            Some(esc) => self.write_escape(esc),
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
    fn write_escape_unescape(&mut self, src: &str, quote: bool) -> OutputResult {
        if quote {
            write_unescaped(src, HtmlEscape::<_, true>::new(&mut *self.out))?;
        } else {
            write_unescaped(src, HtmlEscape::<_, false>::new(&mut *self.out))?;
        }
        Ok(())
    }
    fn write_escape(&mut self, src: &str, quote: bool) -> OutputResult {
        if quote {
            HtmlEscape::<_, true>::new(&mut *self.out).write_str(src)?;
        } else {
            HtmlEscape::<_, false>::new(&mut *self.out).write_str(src)?;
        }
        Ok(())
    }

    fn write_unescape(&mut self, src: &str) -> OutputResult {
        write_unescaped(src, &mut *self.out)?;
        Ok(())
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
                is_void: is_void(&self.string_buf),
            },
        )
    }

    fn _line<T>(&mut self, f: impl FnOnce(&mut Self) -> OutputResult<T>) -> OutputResult<T> {
        match self.config.indent.as_deref() {
            Some(indent) => {
                if !self.first_line {
                    self.out.write_char('\n')?;
                }
                self.first_line = false;
                for _ in 0..self.indent_level {
                    self.out.write_str(indent)?;
                }
                f(self)
            }
            _ => f(self),
        }
    }
    fn line<T>(&mut self, f: impl FnOnce(&mut Self) -> OutputResult<T>) -> OutputResult<T> {
        if self.ci.mode == ContentMode::Block {
            self._line(f)
        } else {
            f(self)
        }
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
            // self._line(f)
            f(self)
        }
    }

    fn in_content<T>(
        &mut self,
        mut ci: ContentInference<'cx>,
        f: impl FnOnce(&mut Self) -> OutputResult<T>,
    ) -> OutputResult<T> {
        mem::swap(&mut ci, &mut self.ci);
        let out = f(self);
        mem::swap(&mut ci, &mut self.ci);
        out
    }

    fn write_open_tag<'attr>(&mut self, tag: &str, selector: &Selector) -> OutputResult {
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

        self.out.write_char('>')?;
        Ok(())
    }

    fn write_close_tag(&mut self, tag: &str) -> OutputResult {
        write!(self.out, "</{tag}>")?;
        Ok(())
    }

    fn process_element(&mut self, element: &Element) -> OutputResult {
        let Some(tag) = (match &element.selector.element {
            SelectorElement::Name(s) => Some(&**s),
            SelectorElement::Infer(InferElement::Block) => Some(self.ci.block),
            SelectorElement::Infer(InferElement::Inline) => Some(self.ci.inline),
            SelectorElement::Infer(InferElement::Line) => Some(self.ci.line),
            SelectorElement::Infer(InferElement::Paragraph) => self.ci.paragraph,
        }) else {
            return self.in_content(
                ContentInference {
                    mode: ContentMode::Inline,
                    ..self.ci
                },
                |this| {
                    element
                        .nodes
                        .iter()
                        .try_for_each(|node| this.process_node(node))
                },
            );
        };

        let (_, tag_info) = self.get_info(tag, self.ci);
        self.line(|this| this.write_open_tag(tag, &element.selector))?;
        self.in_content(tag_info.ci, |this| {
            this.indent(|this| {
                element
                    .nodes
                    .iter()
                    .try_for_each(|node| this.process_node(node))
            })?;
            if !tag_info.is_void {
                this.line(|this| this.write_close_tag(tag))?;
            }
            Ok(())
        })
    }

    fn process_node(&mut self, node: &Node) -> OutputResult {
        match node {
            Node::Element(e) => self.process_element(e),
            Node::Text(s) if s.is_empty() => Ok(()),
            Node::Text(s) => self.write_escape_unescape(s, false),
            Node::Comment(s) => {
                self.out.write_str("<!--")?;
                self.write_escape(s, false)?;
                self.out.write_str("-->")?;
                Ok(())
            }
        }
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
        },
        first_line: true,
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



            Hello, world!
            Click <(a[x=1]>here)> to get<!this is a comment!> started.

            div {
                a> 1
            }
        }
        section {
            line 1
            line 2
            > new paragraph
            > new paragraph
            same paragraph
            > new paragraph
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
        use rs_typed_parser::ast::WithSource;
        ::std::println!("{out}");
    }
}
