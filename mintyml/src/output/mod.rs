mod utils;

use core::{
    fmt::{self, Write},
    mem,
};

use alloc::string::String;

use crate::{
    document::{Comment, Document, Element, Node, NodeType, Selector, Space, TextLike, TextSlice},
    escape::{unescape_parts, UnescapePart},
    utils::{default, to_lowercase},
    OutputConfig,
};

use self::utils::trim_multiline;

fn is_void(tag: &str) -> bool {
    match tag {
        "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input" | "link" | "meta"
        | "param" | "source" | "track" | "wbr" => true,
        _ => false,
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct TagInfo {
    pub is_void: bool,
    pub is_root: bool,
}

struct OutputContext<'cx, 'cfg, Out> {
    src: &'cfg str,
    string_buf: String,
    out: &'cx mut Out,
    config: &'cx OutputConfig<'cfg>,
    indent_level: u32,
    element: Option<&'cx Element<'cfg>>,
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

impl<'cx, 'cfg, Out> OutputContext<'cx, 'cfg, Out>
where
    Out: Write,
{
    fn is_xml(&self) -> bool {
        self.config.xml == Some(true)
    }

    fn format_inline(&self) -> bool {
        self.element.map(|e| e.format_inline()).unwrap_or(false)
    }

    fn is_raw(&self) -> bool {
        self.element.map(|e| e.is_raw()).unwrap_or(false)
    }

    fn slice<'s>(&self, s: &'s TextSlice<'cfg>) -> &'s str {
        s.as_str(self.src)
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

    /// Conform to the requirements of both HTML and XML comments.
    fn write_comment_body(&mut self, src: &str) -> OutputResult {
        // HTML comments must not start with ">" or "->"
        if src.starts_with(">") || src.starts_with("->") {
            self.out.write_char(' ')?;
        }

        let mut slice = src;

        // XML comments must not contain "--", and HTML comments must not contain
        // "<!--", "-->", or "--!>".
        while let Some(i) = slice.find("--") {
            let chunk;
            (chunk, slice) = slice.split_at(i + 1);
            self.out.write_str(chunk)?;
            self.out.write_char(' ')?;
        }

        self.out.write_str(slice)?;

        // XML comments must not end with "-", and HTML comments must not end
        // with "<!-"
        if slice.ends_with("-") {
            self.out.write_char(' ')?;
        }
        Ok(())
    }

    fn write_unescape(&mut self, src: &str) -> OutputResult {
        write_unescaped(src, &mut *self.out).map_err(Into::into)
    }

    fn get_info(&mut self, tag: &str) -> TagInfo {
        let is_root = tag.eq_ignore_ascii_case("html");
        let is_void = !self.is_xml() && is_void(to_lowercase(tag, &mut self.string_buf));

        TagInfo { is_void, is_root }
    }

    fn _line(&mut self) -> OutputResult {
        if let Some(indent) = self.config.indent.as_deref() {
            self.out.write_char('\n')?;
            for _ in 0..self.indent_level {
                self.out.write_str(indent)?;
            }
            self.follows_space = true;
        }

        Ok(())
    }

    fn line(&mut self) -> OutputResult {
        if self.follows_space || self.format_inline() {
            Ok(())
        } else {
            self._line()
        }
    }

    fn space(&mut self, space: &Space) -> OutputResult {
        if !self.follows_space {
            match space {
                Space::LineEnd { .. } | Space::ParagraphEnd { .. }
                    if !self.format_inline() && self.config.indent.is_some() =>
                {
                    self._line()?
                }
                _ => self.out.write_str(" ")?,
            }
            self.follows_space = true;
        }
        Ok(())
    }

    fn indent<T>(
        &mut self,
        by: u32,
        f: impl FnOnce(&mut Self) -> OutputResult<T>,
    ) -> OutputResult<T> {
        if !self.format_inline() {
            if self.config.indent.is_some() {
                self.indent_level += by;
                let out = f(self);
                self.indent_level -= by;
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
        element: &'cx Element<'cfg>,
        f: impl FnOnce(&mut Self) -> OutputResult<T>,
    ) -> OutputResult<T> {
        let mut element = Some(element);
        mem::swap(&mut element, &mut self.element);
        let out = f(self);
        self.element = element;
        out
    }

    fn write_open_tag(
        &mut self,
        tag: &str,
        selector: &Selector<'cfg>,
        self_close: bool,
    ) -> OutputResult {
        write!(self.out, "<{tag}")?;

        if let Some(id) = selector.id() {
            self.out.write_str(" id=\"")?;
            self.write_escape_unescape(self.slice(id), true)?;
            self.out.write_char('"')?;
        }

        let mut class_names = selector.class_names();
        if let Some(first) = class_names.next() {
            self.out.write_str(" class=\"")?;
            self.write_escape_unescape(self.slice(first), true)?;

            for class in class_names {
                self.out.write_char(' ')?;
                self.write_escape_unescape(self.slice(class), true)?;
            }

            self.out.write_char('"')?;
        }

        for (name, value) in selector.attributes() {
            self.out.write_char(' ')?;
            self.write_unescape(self.slice(name))?;

            if let Some(value) = value {
                self.out.write_str("=\"")?;
                self.write_escape_unescape(self.slice(value), true)?;
                self.out.write_char('"')?;
            } else if self.is_xml() {
                self.out.write_str("=\"\"")?;
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

    fn process_element(&mut self, element: &'cx Element<'cfg>) -> OutputResult {
        let get_valid_tags =
            |e: &'cx Element<'cfg>| e.selectors.iter().filter_map(|s| Some((s.tag.name()?, s)));

        let mut opening_tags = get_valid_tags(element).peekable();

        if opening_tags.peek().is_none() {
            return self.in_content(element, |this| {
                element
                    .content
                    .nodes
                    .iter()
                    .try_for_each(|node| this.process_node(node))
            });
        }

        let self_close_last = element.content.nodes.is_empty() && self.is_xml();
        let mut last_tag_info = TagInfo::default();

        while let Some((tag, selector)) = opening_tags.next() {
            let tag = self.slice(tag);
            let is_last = opening_tags.peek().is_none();
            if is_last {
                last_tag_info = self.get_info(tag);
            }
            let self_closing = is_last && self_close_last;
            self.write_open_tag(tag, &selector, self_closing)?;
        }

        self.in_content(element, |this| {
            if !element.content.nodes.is_empty() {
                this.indent(if last_tag_info.is_root { 0 } else { 1 }, |this| {
                    this.line()?;
                    element
                        .content
                        .nodes
                        .iter()
                        .try_for_each(|node| this.process_node(node))
                })?;
                this.line()?;
            }
            for (tag, _) in get_valid_tags(element)
                .rev()
                .skip(if self_close_last { 1 } else { 0 })
            {
                let tag = this.slice(tag);
                let tag_info = this.get_info(tag);
                if !this.is_xml() && !tag_info.is_void {
                    this.write_close_tag(tag)?;
                }
            }
            Ok(())
        })
    }

    fn process_node(&mut self, node: &'cx Node<'cfg>) -> OutputResult {
        let out = match &node.node_type {
            NodeType::Element { value } => self.process_element(value)?,
            NodeType::TextLike { value } => match value {
                TextLike::Text { value: text } if text.slice.is_empty() => {}
                TextLike::Text { value: text } => {
                    let is_raw = !self.is_xml() && (text.escape_out || self.is_raw());
                    let slice = self.slice(&text.slice);

                    let mut write = |value| match (text.unescape_in, is_raw) {
                        (true, true) => self.write_unescape(value),
                        (true, false) => self.write_escape_unescape(value, false),
                        (false, true) => self.out.write_str(value).map_err(Into::into),
                        (false, false) => self.write_escape(value, false),
                    };

                    let mut last_line = slice;

                    if text.multiline {
                        for line in trim_multiline(slice) {
                            write(line)?;
                            last_line = line
                        }
                    } else {
                        write(slice)?;
                    }

                    self.follows_space = last_line.ends_with([' ', '\t']);
                }
                TextLike::Comment {
                    value: Comment::Tag { value },
                } => {
                    self.out.write_str("<!--")?;
                    self.write_comment_body(self.slice(value))?;
                    self.out.write_str("-->")?;
                    self.follows_space = false;
                }
                TextLike::Space { value: space } => {
                    self.space(space)?;
                    self.follows_space = true
                }
            },
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

pub fn output_html_to<'cx, 'cfg>(
    src: &'cfg str,
    document: &'cx Document<'cfg>,
    out: &'cx mut impl Write,
    config: &'cx OutputConfig<'cfg>,
) -> OutputResult {
    match (OutputContext::<'cx, 'cfg> {
        src,
        string_buf: default(),
        out,
        config,
        indent_level: 0,
        element: None,
        follows_space: true,
    }) {
        mut cx => {
            if cx.config.complete_page.unwrap_or(false) {
                cx.out.write_str("<!DOCTYPE html>\n")?;
            }

            document
                .content
                .nodes
                .iter()
                .try_for_each(|node| cx.process_node(node))?;

            cx.line()?;

            Ok(())
        }
    }
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
    let config = OutputConfig {
        indent: Some("  ".into()),
        ..default()
    };
    let (document, errors) = Document::parse(src, &config);
    let document = document.ok_or_else(|| errors).unwrap();

    output_html_to(src, &document, &mut out, &config).unwrap();
    #[cfg(feature = "std")]
    {
        ::std::println!("{out}");
    }
}
