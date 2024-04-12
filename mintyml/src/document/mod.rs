use core::fmt;

use alloc::{
    borrow::{Cow, ToOwned},
    vec,
    vec::Vec,
};
use gramma::{parse::LocationRange, parse_tree, ParseError};

use crate::{
    ast,
    escape::{escape_errors, EscapeError},
    utils::{default, intersperse_with, join_display, try_extend, DisplayFn},
};

/// A value that can be made to outlive the `'static` lifetime, e.g. by copying all borrowed data.
pub trait ToStatic {
    type Static: 'static;
    /// Consumes `self` and returns an equivalent value that contains no borrowed data.
    fn to_static(self) -> Self::Static;
}

impl<T> ToStatic for Cow<'_, T>
where
    T: ?Sized + ToOwned + 'static,
{
    type Static = Cow<'static, T>;

    fn to_static(self) -> Self::Static {
        match self {
            Cow::Borrowed(x) => Cow::Owned(x.to_owned()),
            Cow::Owned(x) => Cow::Owned(x),
        }
    }
}

impl<T> ToStatic for Vec<T>
where
    T: ToStatic,
{
    type Static = Vec<T::Static>;

    fn to_static(self) -> Self::Static {
        self.into_iter().map(T::to_static).collect()
    }
}

impl<T> ToStatic for Option<T>
where
    T: ToStatic,
{
    type Static = Option<T::Static>;

    fn to_static(self) -> Self::Static {
        self.map(T::to_static)
    }
}

/// Represents a fully parsed MinTyML document.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct Document<'src> {
    pub range: LocationRange,
    pub nodes: Vec<Node<'src>>,
}

impl<'src> ToStatic for Document<'src> {
    type Static = Document<'static>;
    fn to_static(self) -> Self::Static {
        Document {
            range: self.range,
            nodes: self.nodes.to_static(),
        }
    }
}

/// Represents some kind of whitespace that should be considered when converting to HTML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Space {
    /// Whitespace between elements on the same line.
    Inline,
    /// Whitespace between lines of a paragraph.
    LineEnd,
    /// Whitespace at the end of a paragraph.
    ParagraphEnd,
}

/// Represents plain text data.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct Text<'src> {
    pub value: Src<'src>,
    pub escape: bool,
    pub multiline: bool,
    pub raw: bool,
}

/// Represents a MinTyML node, which roughly corresponds to an HTML element.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct Node<'src> {
    pub range: LocationRange,
    pub node_type: NodeType<'src>,
}

/// The internal data of a MinTyML node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum NodeType<'src> {
    Element(Element<'src>),
    Text(Text<'src>),
    Comment(Src<'src>),
    Space(Space),
}

impl<'src> From<Text<'src>> for NodeType<'src> {
    fn from(v: Text<'src>) -> Self {
        Self::Text(v)
    }
}

impl<'src> From<Space> for NodeType<'src> {
    fn from(v: Space) -> Self {
        Self::Space(v)
    }
}

impl<'src> From<Element<'src>> for NodeType<'src> {
    fn from(v: Element<'src>) -> Self {
        Self::Element(v)
    }
}

impl ToStatic for Node<'_> {
    type Static = Node<'static>;

    fn to_static(self) -> Self::Static {
        Node {
            range: self.range,
            node_type: self.node_type.to_static(),
        }
    }
}

impl<'src> ToStatic for NodeType<'src> {
    type Static = NodeType<'static>;

    fn to_static(self) -> Self::Static {
        match self {
            NodeType::Element(x) => NodeType::Element(x.to_static()),
            NodeType::Text(Text {
                value,
                escape,
                multiline,
                raw,
            }) => NodeType::Text(Text {
                value: value.to_static(),
                escape,
                multiline,
                raw,
            }),
            NodeType::Comment(x) => NodeType::Comment(x.to_static()),
            NodeType::Space(x) => NodeType::Space(x),
        }
    }
}

/// Represents the context in which child elements are defined.
/// Used as a hint for element type inference as well as line breaks when producing "pretty" HTML.
#[non_exhaustive]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContentMode {
    /// The element is defined with block syntax within a block.
    #[default]
    Block,
    /// The element is defined as a line, line-block, or inline element, or within inline content.
    Inline,
}

/// A MinTyML node that produces an HTML element.
#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct Element<'src> {
    pub selector: Selector<'src>,
    /// Child nodes of the element.
    pub nodes: Vec<Node<'src>>,
    pub kind: ElementKind,
    /// If true, escape sequences within this element should be converted as-is.
    pub is_raw: bool,
    pub mode: ContentMode,
}

impl<'src> Element<'src> {
    pub fn with_tag(tag: impl Into<Src<'src>>) -> Self {
        Self {
            selector: SelectorElement::Name(tag.into()).into(),
            ..default()
        }
    }
}

impl<'src> ToStatic for Element<'src> {
    type Static = Element<'static>;
    fn to_static(self) -> Self::Static {
        Element {
            selector: self.selector.to_static(),
            nodes: self.nodes.to_static(),
            kind: self.kind,
            is_raw: self.is_raw,
            mode: self.mode,
        }
    }
}

/// Defines the type and attributes of an element.
#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct Selector<'src> {
    /// The element's type (or _tag_).
    pub element: SelectorElement<'src>,
    /// The classes applied to the element.
    pub class_names: Vec<Src<'src>>,
    /// The ID of the element.
    pub id: Option<Src<'src>>,
    /// All other attributes of the element.
    pub attributes: Vec<Attribute<'src>>,
}

impl<'src> ToStatic for Selector<'src> {
    type Static = Selector<'static>;
    fn to_static(self) -> Self::Static {
        Selector {
            element: self.element.to_static(),
            class_names: self.class_names.to_static(),
            id: self.id.to_static(),
            attributes: self.attributes.to_static(),
        }
    }
}

impl<'src> From<SelectorElement<'src>> for Selector<'src> {
    fn from(element: SelectorElement<'src>) -> Self {
        Selector {
            element,
            class_names: default(),
            id: default(),
            attributes: default(),
        }
    }
}

pub(crate) type Src<'src> = Cow<'src, str>;

/// Represents an HTML attribute of an element.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct Attribute<'src> {
    pub name: Src<'src>,
    pub value: Option<Src<'src>>,
}

impl<'src> ToStatic for Attribute<'src> {
    type Static = Attribute<'static>;

    fn to_static(self) -> Self::Static {
        Attribute {
            name: self.name.to_static(),
            value: self.value.to_static(),
        }
    }
}

/// A the part of a selector that defines the element type.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum SelectorElement<'src> {
    /// The type is unspecified and should be inferred.
    #[default]
    Infer,
    /// The name of the type.
    Name(Src<'src>),
    /// The type of a special of element.
    Special(SpecialKind),
}

impl<'src> ToStatic for SelectorElement<'src> {
    type Static = SelectorElement<'static>;
    fn to_static(self) -> Self::Static {
        match self {
            SelectorElement::Infer => SelectorElement::Infer,
            SelectorElement::Name(x) => SelectorElement::Name(x.to_static()),
            SelectorElement::Special(x) => SelectorElement::Special(x),
        }
    }
}

/// Describes the syntax used to define an element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ElementDelimiter {
    /// Defined using line (`>`) syntax, or a paragraph.
    Line,
    /// Defined using line-block (`> {}`)syntax.
    LineBlock,
    /// Defined using block (`{}`) syntax.
    Block,
}

/// Describes how an element or paragraph is represented.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ElementKind {
    /// An explicit single-line element.
    Line,
    /// A multi-line element with the semantics of a line element.
    LineBlock,
    #[default]
    /// A multi-line element.
    Block,
    /// An element defined within a line element or paragraph
    Inline(
        /// The syntax used to define the element inside the inline delimiters
        /// e.g. `<(b> text)>`, `<(b> { text })>`, `<(b { text })>`, `<(text)>`
        Option<ElementDelimiter>
    ),
    /// An element implicitly defined by a group of consecutive lines of text.
    /// Depending on inference, it may correspond to a text node rather than an HTML element.
    Paragraph,
}

/// Indicates some error occurred while building the tree.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
struct BuildError {}

/// Represents a syntax error in the MinTyML source.
#[non_exhaustive]
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "std", derive(thiserror::Error), error("{kind:?} at character {}", range.start.position))]
pub struct SyntaxError {
    /// The [LocationRange] encapsulating the syntax error.
    pub range: LocationRange,
    pub kind: SyntaxErrorKind,
}

impl SyntaxError {
    pub(crate) fn display_with_src<'data>(
        &'data self,
        src: &'data str,
    ) -> impl fmt::Display + 'data {
        DisplayFn(move |f| {
            let mut inner = |sample| {
                match &self.kind {
                    SyntaxErrorKind::Unknown => f.write_str("Unknown"),
                    SyntaxErrorKind::InvalidEscape { .. } => {
                        write!(f, "Invalid escape sequence {sample}.")
                    }
                    SyntaxErrorKind::ParseFailed { expected } => {
                        write!(
                            f,
                            "Unexpected {sample}. Expected {}",
                            join_display(expected.iter().map(|t| t.name()), " | ")
                        )
                    }
                }?;

                write!(f, " at character {}", self.range.start.position)
            };

            if self.range.start.position >= src.len() {
                inner(format_args!("end-of-file"))
            } else {
                inner(format_args!(
                    "{:?}",
                    src.get(self.range.start.position..self.range.end.position)
                        .unwrap_or_default()
                ))
            }
        })
    }
}

/// Indicates what caused a syntax error.
#[non_exhaustive]
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SyntaxErrorKind {
    /// An unknown error occurred.
    #[default]
    Unknown,
    /// An invalid escape sequence was found.
    #[non_exhaustive]
    InvalidEscape {},
    /// The document could not be parsed into an abstract syntax tree.
    #[non_exhaustive]
    ParseFailed {
        expected: Vec<gramma::error::ExpectedParse>,
    },
}

impl From<EscapeError> for SyntaxError {
    fn from(value: EscapeError) -> Self {
        Self {
            range: value.range,
            kind: SyntaxErrorKind::InvalidEscape {},
        }
    }
}

impl From<ParseError<'_>> for SyntaxError {
    fn from(value: ParseError<'_>) -> Self {
        let start = value.location;
        let end = start + value.actual.len();
        Self {
            range: LocationRange { start, end },
            kind: SyntaxErrorKind::ParseFailed {
                expected: value.expected,
            },
        }
    }
}

/// An object that holds relevant state and resources for building a document.
#[derive(Debug)]
struct BuildContext<'src> {
    /// The MinTyML source string.
    pub src: &'src str,
    /// All syntax errors found while building so far.
    pub errors: Vec<SyntaxError>,
}

impl<'src> BuildContext<'src> {
    /// Extracts a slice of the source.
    fn slice(&self, range: LocationRange) -> Src<'src> {
        slice_str(self.src, range)
    }

    /// Extracts a slice of the source, validating any escape sequences within.
    fn escapable_slice(&mut self, range: LocationRange) -> BuildResult<Src<'src>> {
        let slice = self.slice(range);
        self.record_errors(escape_errors(&slice, range.start))
            .map(|()| slice)
    }

    /// Adds the given errors to the context.
    /// Returns `Err(_)` if `errors` contained at least one value.
    fn record_errors<E: Into<SyntaxError>>(
        &mut self,
        errors: impl IntoIterator<Item = E>,
    ) -> BuildResult<()> {
        let pre_len = self.errors.len();
        self.errors.extend(errors.into_iter().map(Into::into));
        if self.errors.len() == pre_len {
            Ok(())
        } else {
            Err(default())
        }
    }
}

/// Extracts a slice of a string given a [LocationRange].
fn slice_str<'src>(src: &'src str, LocationRange { start, end }: LocationRange) -> Src<'src> {
    src.get(start.position..end.position)
        .unwrap_or_default()
        .into()
}

type BuildResult<T> = Result<T, BuildError>;

impl<'src> Document<'src> {
    fn build_from_ast(cx: &mut BuildContext<'src>, ast: &ast::Document) -> BuildResult<Self> {
        Ok(Document {
            range: LocationRange {
                start: ast.start,
                end: ast.end,
            },
            nodes: nodes_from_ast(
                cx,
                ast.nodes
                    .as_ref()
                    .map(|n| n.nodes.iter())
                    .unwrap_or_default(),
            )?,
        })
    }

    /// Converts an abstract syntax tree to a document.
    pub fn from_ast(src: &'src str, ast: &ast::Document) -> Result<Self, Vec<SyntaxError>> {
        let mut cx = BuildContext {
            src,
            errors: default(),
        };
        Self::build_from_ast(&mut cx, &ast).map_err(|_| cx.errors)
    }

    /// Parses a document from a MinTyML source string.
    pub fn parse(src: &'src str) -> Result<Self, Vec<SyntaxError>> {
        let ast = parse_tree::<ast::Document, 4>(src).map_err(|e| vec![e.into()])?;
        Self::from_ast(src, &ast)
    }
}

impl<'src> Selector<'src> {
    fn build_from_ast(
        cx: &mut BuildContext<'src>,
        ast::Selector { element, parts }: &ast::Selector,
    ) -> BuildResult<Self> {
        let element = match element {
            Some(ast::ElementSelector::Name { name }) => {
                SelectorElement::Name(cx.slice(name.range))
            }
            None | Some(ast::ElementSelector::Star { .. }) => SelectorElement::Infer,
        };

        parts.iter().try_fold(Self::from(element), |mut out, part| {
            match part {
                ast::SelectorPart::Attribute { value } => {
                    for attr in &value.parts {
                        let name = cx.escapable_slice(attr.name.range)?;

                        let value = attr
                            .assignment
                            .as_ref()
                            .map(|a| match &a.value {
                                ast::AttributeValue::Unquoted { value } => value.range,
                                ast::AttributeValue::Quoted { value } => {
                                    let mut range = value.range;

                                    const QUOTE_LEN: usize = {
                                        if '"'.len_utf8() != '\''.len_utf8() {
                                            panic!();
                                        }
                                        '"'.len_utf8()
                                    };

                                    range.start += QUOTE_LEN;
                                    range.end -= QUOTE_LEN;
                                    range
                                }
                            })
                            .map(|range| cx.escapable_slice(range))
                            .transpose()?;

                        out.attributes.push(Attribute { name, value });
                    }
                }
                ast::SelectorPart::ClassSelector { value } => {
                    out.class_names.push(cx.escapable_slice(value.ident.range)?);
                }
                ast::SelectorPart::IdSelector { value } => {
                    out.id = Some(cx.escapable_slice(value.ident.range)?);
                }
            }
            Ok(out)
        })
    }
}

/// Given a [ast::TextLine], push all nodes on the line to `out`.
fn build_text_line<'src>(
    cx: &mut BuildContext<'src>,
    line: &ast::TextLine,
    out: &mut Vec<Node<'src>>,
) -> BuildResult<()> {
    let first = [(&None, &line.part1)].into_iter();

    let rest = line.parts.iter().map(|(space, parts)| (space, parts));

    let nodes = first.chain(rest).flat_map(|(space, part)| {
        [
            space.as_ref().map(|space| {
                Ok(Node {
                    range: space.range,
                    node_type: Space::Inline.into(),
                })
            }),
            Some(build_text_line_part(part, cx)),
        ]
        .into_iter()
        .flatten()
    });

    try_extend(out, nodes)
}

/// Builds a node from the multiline text in a given range.
fn get_multiline_text<'src, const ESCAPE: bool>(
    cx: &mut BuildContext<'src>,
    range: LocationRange,
    escape: bool,
) -> BuildResult<Node<'src>> {
    let value = if escape {
        cx.escapable_slice(range)?
    } else {
        cx.slice(range)
    };
    Ok(Node {
        range,
        node_type: Text {
            value,
            escape,
            multiline: true,
            ..default()
        }
        .into(),
    })
}

/// Builds a node from a [ast::Multiline].
fn build_multiline<'src>(
    cx: &mut BuildContext<'src>,
    multiline: &ast::Multiline,
) -> BuildResult<Node<'src>> {
    match multiline {
        ast::Multiline::Escaped { value } => get_multiline_text::<true>(cx, value.range, true),
        ast::Multiline::Unescaped { value } => get_multiline_text::<false>(cx, value.range, false),
    }
}

/// Builds node contents from a [ast::MultilineCode].
fn build_multiline_code<'src>(
    cx: &mut BuildContext<'src>,
    multiline: &ast::MultilineCode,
) -> BuildResult<NodeType<'src>> {
    let inner = get_multiline_text::<false>(cx, multiline.range, false)?;

    let code = Element {
        kind: ElementKind::Paragraph,
        selector: SelectorElement::Special(SpecialKind::Code).into(),
        nodes: vec![inner],
        ..default()
    };

    let pre = Element {
        kind: ElementKind::Block,
        selector: SelectorElement::Special(SpecialKind::CodeBlockContainer).into(),
        nodes: vec![Node {
            range: multiline.range,
            node_type: code.into(),
        }],
        ..default()
    };

    Ok(pre.into())
}

fn build_paragraph_item<'src>(
    cx: &mut BuildContext<'src>,
    item: &ast::ParagraphItem,
    out: &mut Vec<Node<'src>>,
) -> BuildResult<()> {
    match item {
        ast::ParagraphItem::Line { line } => build_text_line(cx, line, out)?,
        ast::ParagraphItem::Multiline { multiline } => out.push(build_multiline(cx, multiline)?),
    }

    Ok(())
}

fn build_verbatim_text<'src>(
    v: &ast::Verbatim,
    cx: &mut BuildContext<'src>,
) -> BuildResult<NodeType<'src>> {
    let (mut range, trim_start, trim_end) = match &v.tail {
        ast::VerbatimTail::Verbatim0 { value, .. } => (value.range, 1, 3),
        ast::VerbatimTail::Verbatim1 { value, .. } => (value.range, 2, 4),
        ast::VerbatimTail::Verbatim2 { value, .. } => (value.range, 3, 5),
    };

    range.start += trim_start;
    range.end -= trim_end;

    Ok(Text {
        value: cx.slice(range),
        raw: v.raw.is_some(),
        ..default()
    }
    .into())
}

/// The type of a special element.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpecialKind {
    /// `em>`
    Emphasis,
    /// `strong>`
    Strong,
    /// `u>`
    Underline,
    /// `s>`
    Strike,
    /// `q>`
    Quote,
    /// `code>`
    Code,
    /// `pre>code>`
    CodeBlockContainer,
}

fn build_inline_special<'src>(
    special: &ast::InlineSpecial,
    cx: &mut BuildContext<'src>,
) -> BuildResult<Node<'src>> {
    use ast::InlineSpecial::*;

    let kind = match special {
        Emphasis { .. } => SpecialKind::Emphasis,
        Strong { .. } => SpecialKind::Strong,
        Underline { .. } => SpecialKind::Underline,
        Strike { .. } => SpecialKind::Strike,
        Quote { .. } => SpecialKind::Quote,
        Code { .. } => SpecialKind::Code,
    };
    let nodes = match special {
        Emphasis { inner, .. }
        | Strong { inner, .. }
        | Underline { inner, .. }
        | Strike { inner, .. }
        | Quote { inner, .. } => nodes_from_ast(cx, &inner.nodes)?,
        Code { code, .. } => {
            let mut range = code.range;
            range.start += 2;
            range.end -= 2;
            vec![Node {
                range: code.range,
                node_type: Text {
                    value: cx.slice(range),
                    ..default()
                }
                .into(),
            }]
        }
    };
    let (open, close) = match special {
        Emphasis { open, close, .. } => (open.range, close.range),
        Strong { open, close, .. } => (open.range, close.range),
        Underline { open, close, .. } => (open.range, close.range),
        Strike { open, close, .. } => (open.range, close.range),
        Quote { open, close, .. } => (open.range, close.range),
        Code { code } => (code.range, code.range),
    };

    let range = open.combine(close);

    Ok(Element {
        selector: Selector {
            element: SelectorElement::Special(kind),
            ..default()
        },
        nodes,
        kind: ElementKind::Inline(None),
        ..default()
    })
    .map(NodeType::Element)
    .map(|node_type| Node { range, node_type })
}

/// Determines the syntax used to define an element.
fn get_delimiter(body: &ast::ElementBody) -> ElementDelimiter {
    match body {
        ast::ElementBody::Block { .. } => ElementDelimiter::Block,
        ast::ElementBody::LineBlock { .. } => ElementDelimiter::LineBlock,
        ast::ElementBody::Line { .. } => ElementDelimiter::Line,
    }
}

/// Builds a node that represents a portion of a line.
///
/// This could be:
/// - plain text
/// - verbatim text
/// - an inline element
/// - a comment
fn build_text_line_part<'src>(
    part: &ast::TextLinePart,
    cx: &mut BuildContext<'src>,
) -> BuildResult<Node<'src>> {
    use ast::TextLinePart::*;
    match part {
        NonParagraph { node } => Ok(Node {
            range: LocationRange {
                start: node.start,
                end: node.end,
            },
            node_type: build_non_paragraph_node(cx, &node.node_type)?,
        }),
        TextSegment { text } => Ok(Node {
            range: text.range,
            node_type: Text {
                value: cx.escapable_slice(text.range)?,
                escape: true,
                ..default()
            }
            .into(),
        }),
        Inline {
            inline: ast::Inline {
                inner: Some(node), ..
            },
        } => Ok(Node {
            range: LocationRange {
                start: node.start,
                end: node.end,
            },
            node_type: match &node.node_type {
                ast::NodeType::NonParagraph { node } => {
                    build_non_paragraph_node(cx, &node.node_type)?
                }
                ast::NodeType::MultilineCode { multiline } => build_multiline_code(cx, multiline)?,
                ast::NodeType::Element {
                    element: ast::Element::WithSelector { selector, body },
                } => Element {
                    selector: Selector::build_from_ast(cx, selector)?,
                    nodes: build_element_body(cx, body)?,
                    kind: ElementKind::Inline(Some(get_delimiter(body))),
                    ..default()
                }
                .into(),
                ast::NodeType::Element {
                    element: ast::Element::Body { body },
                } => Element {
                    selector: default(),
                    nodes: build_element_body(cx, body)?,
                    kind: ElementKind::Inline(Some(get_delimiter(body))),
                    ..default()
                }
                .into(),
                ast::NodeType::Paragraph { paragraph } => Element {
                    kind: ElementKind::Inline(None),
                    ..build_paragraph(cx, paragraph)?
                }
                .into(),
            },
        }),
        Inline {
            inline:
                ast::Inline {
                    inner: None,
                    open,
                    close,
                    ..
                },
        } => Ok(Node {
            range: open.range.combine(close.range),
            node_type: Element {
                kind: ElementKind::Inline(None),
                ..default()
            }
            .into(),
        }),
        InlineSpecial { inline_special } => build_inline_special(inline_special, cx),
    }
}

fn build_paragraph<'src>(
    cx: &mut BuildContext<'src>,
    paragraph: &ast::Paragraph,
) -> BuildResult<Element<'src>> {
    let mut nodes = Vec::new();
    build_paragraph_item(cx, &paragraph.line1, &mut nodes)?;

    for (space, line) in &paragraph.lines {
        nodes.push(Node {
            range: space.range,
            node_type: NodeType::Space(Space::LineEnd),
        });
        build_paragraph_item(cx, line, &mut nodes)?;
    }

    Ok(Element {
        selector: default(),
        nodes,
        kind: ElementKind::Paragraph,
        ..default()
    })
}

fn build_element_body<'src>(
    cx: &mut BuildContext<'src>,
    ast: &ast::ElementBody,
) -> BuildResult<Vec<Node<'src>>> {
    match ast {
        ast::ElementBody::Block { block } | ast::ElementBody::LineBlock { block, .. } => block
            .nodes
            .as_ref()
            .map(|n| nodes_from_ast(cx, &n.nodes))
            .unwrap_or(Ok(default())),
        ast::ElementBody::Line { body: None, .. } => Ok(default()),
        ast::ElementBody::Line {
            body: Some(body), ..
        } => Ok(vec![Node {
            range: LocationRange {
                start: body.start,
                end: body.end,
            },
            node_type: match &body.node_type {
                ast::NodeType::NonParagraph { node } => {
                    build_non_paragraph_node(cx, &node.node_type)?
                }
                ast::NodeType::MultilineCode { multiline } => build_multiline_code(cx, multiline)?,
                ast::NodeType::Element { element } => build_element(cx, element)?.into(),
                ast::NodeType::Paragraph { paragraph } => build_paragraph(cx, paragraph)?.into(),
            },
        }]),
    }
}

fn build_element<'src>(
    cx: &mut BuildContext<'src>,
    ast: &ast::Element,
) -> BuildResult<Element<'src>> {
    match ast {
        ast::Element::WithSelector { selector, body } => Ok(Element {
            selector: Selector::build_from_ast(cx, selector)?,
            nodes: build_element_body(cx, body)?,
            kind: get_default_kind(body),
            ..default()
        }),
        ast::Element::Body { body } => Ok(Element {
            selector: default(),
            nodes: build_element_body(cx, body)?,
            kind: get_default_kind(body),
            ..default()
        }),
    }
}

/// Returns the [ElementKind] that most closely matches the given [ast::ElementBody].
fn get_default_kind(body: &ast::ElementBody) -> ElementKind {
    match body {
        ast::ElementBody::Block { .. } => ElementKind::Block,
        ast::ElementBody::Line { .. } => ElementKind::Line,
        ast::ElementBody::LineBlock { .. } => ElementKind::LineBlock,
    }
}

fn build_non_paragraph_node<'src>(
    cx: &mut BuildContext<'src>,
    node_type: &ast::NonParagraphNodeType,
) -> BuildResult<NodeType<'src>> {
    match node_type {
        ast::NonParagraphNodeType::Verbatim { verbatim } => build_verbatim_text(verbatim, cx),
        ast::NonParagraphNodeType::Comment { comment } => {
            Ok(NodeType::Comment(cx.slice(comment.inner)))
        }
    }
}

/// Builds a list of nodes from a collection of [ast::Node].
fn nodes_from_ast<'src, 'ast>(
    cx: &mut BuildContext<'src>,
    ast: impl IntoIterator<Item = &'ast ast::Node>,
) -> BuildResult<Vec<Node<'src>>> {
    let nodes = ast.into_iter().map(|node| {
        Ok(Node {
            range: LocationRange {
                start: node.start,
                end: node.end,
            },
            node_type: match &node.node_type {
                ast::NodeType::NonParagraph { node } => {
                    build_non_paragraph_node(cx, &node.node_type)?
                }
                ast::NodeType::MultilineCode { multiline } => build_multiline_code(cx, multiline)?,
                ast::NodeType::Element { element } => build_element(cx, element)?.into(),
                ast::NodeType::Paragraph { paragraph } => build_paragraph(cx, paragraph)?.into(),
            },
        })
    });
    intersperse_with(nodes, |pre, post| {
        Ok(Node {
            range: LocationRange {
                start: pre.as_ref().map_err(|_| default())?.range.end,
                end: post.as_ref().map_err(|_| default())?.range.start,
            },
            node_type: Space::ParagraphEnd.into(),
        })
    })
    .collect()
}

#[test]
fn document_demo() {
    let src = r#"
        section {
            h1#foo.bar[
                x
            ].baz> <( foo )>

            Hello, world!
            Click <( a[x=1]> here )> to get<!this is a comment!> started.

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
    let _doc = Document::parse(src).unwrap();
    #[cfg(feature = "std")]
    {
        ::std::println!("{:#?}", _doc);
    }
}
