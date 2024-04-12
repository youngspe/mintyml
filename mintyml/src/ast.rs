use alloc::{boxed::Box, vec::Vec};
use gramma::parse::{Location, LocationRange};

gramma::define_string_pattern!(
    fn unicode_escape() {
        exactly("\\u{") + repeat(.., !char(('\n', '{', '}'))).simple() + exactly("}")
    }

    fn escape() {
        unicode_escape() | char('\\') + char(..)
    }

    fn identifier_char() {
        char((ascii_alphanumeric(), "-:")) | escape()
    }

    fn identifier() {
        identifier_char().repeat(1..).simple()
    }

    fn class_name() {
        (identifier_char() | char("!@$%^&*+=_/?();") + !precedes(char('>')))
            .repeat(1..)
            .simple()
    }

    fn attr_string() {
        char('\'') + (!char(('\\', '\'')) | escape()).repeat(1..).simple() + char('\'')
            | char('"') + (!char(('\\', '"')) | escape()).repeat(1..).simple() + char('"')
    }

    fn element_name() {
        !precedes(ascii_digit()) + identifier()
    }
);

gramma::define_token!(
    #[pattern(exact = ">")]
    pub struct RightAngle;

    #[pattern(exact = "[")]
    pub struct LeftBracket;
    #[pattern(exact = "]")]
    pub struct RightBracket;

    #[pattern(exact = "<(")]
    pub struct OpenInline;
    #[pattern(exact = ")>")]
    pub struct CloseInline;

    #[pattern(exact = "{")]
    pub struct LeftBrace;
    #[pattern(exact = "}")]
    pub struct RightBrace;

    #[pattern(exact = "*")]
    pub struct Star;
    #[pattern(exact = ".")]
    pub struct Dot;
    #[pattern(exact = "#")]
    pub struct Hash;
    #[pattern(exact = "=")]
    pub struct Equals;
    #[pattern(matcher = element_name())]
    pub struct ElementName;
    #[pattern(matcher = identifier())]
    pub struct Ident;
    #[pattern(matcher = class_name())]
    pub struct ClassName;

    #[pattern(matcher = {
        precedes(!whitespace())
        + (
            (whitespace() & !char('\n')).repeat(..).simple()
            + (
                !char("\\{}<>()@#$^&*?~_[]|/-+=\"'`!,.") & !whitespace()
                | char("()@#$^&*?~_[]|/-+=\"'`!,.") + !precedes(char('>'))
                | escape()
            ).repeat(1..).simple()
        ).repeat(1..).simple()
    })]
    /// Matches any part of a paragraph line that is not an element.
    pub struct TextSegment;

    #[pattern(matcher = {
        exactly("```") | exactly(r#"""""#) | exactly("'''")
    })]
    /// Fail if TextSegment matches this so it doesn't match multiline elements.
    pub struct InvalidTextSegment;

    #[pattern(matcher = {
        whitespace().repeat(..).lazy()
        + char('\n')
        + char(..).repeat(..).lazy()
        + (line_end() | precedes(!whitespace()))
    })]
    pub struct NewLine;

    #[pattern(matcher = (whitespace() & !char('\n')).repeat(1..))]
    pub struct Space;
    #[pattern(matcher = whitespace().repeat(1..))]
    pub struct Whitespace;

    #[pattern(matcher = {
        // Element type
        (char('*') | element_name()).optional().simple()
         +(
            // Id/class
            char(".#").repeat(1..).simple() + class_name()
            // Attribute
            | char('[') + (!char("[]\\\"'") | attr_string() | escape()).repeat(..).simple() + char(']')
        ).repeat(..)
    })]
    pub struct SelectorString;

    #[pattern(matcher = {
        repeat(1.., !char(("=>'\"/[]\\", whitespace())) | escape()).simple()
    })]
    pub struct AttributeName;

    #[pattern(matcher = {
        repeat(1.., !char(("[]\\\"'", whitespace())) | escape()).simple()
    })]
    pub struct UnquotedAttributeValue;

    #[pattern(matcher = {
        char('"') + (!char("\\\"") | escape()).repeat(..).simple() + char('"')
        | char('\'') + (!char("\\'") | escape()).repeat(..).simple() + char('\'')
    })]
    pub struct QuotedString;

    #[pattern(matcher = {
        !char("<!")
        | char('<') + !precedes(char('!'))
        | char('!') + !precedes(char('>'))
    })]
    pub struct CommentText;

    #[pattern(exact = "<!")]
    pub struct OpenComment;
    #[pattern(exact = "!>")]
    pub struct CloseComment;

    #[pattern(exact = "</")]
    pub struct OpenEmphasis;
    #[pattern(exact = "/>")]
    pub struct CloseEmphasis;

    #[pattern(exact = "<#")]
    pub struct OpenStrong;
    #[pattern(exact = "#>")]
    pub struct CloseStrong;

    #[pattern(exact = "<_")]
    pub struct OpenUnderline;
    #[pattern(exact = "_>")]
    pub struct CloseUnderline;

    #[pattern(exact = "<~")]
    pub struct OpenStrike;
    #[pattern(exact = "~>")]
    pub struct CloseStrike;

    #[pattern(exact = "<\"")]
    pub struct OpenQuote;
    #[pattern(exact = "\">")]
    pub struct CloseQuote;

    #[pattern(matcher = {
        exactly("<`") + char(..).repeat(..).lazy() + exactly("`>")
    })]
    pub struct InlineCode;

    #[pattern(exact = r"<[")]
    pub struct VerbatimOpen;

    #[pattern(exact = "raw")]
    pub struct KeywordRaw;

    #[pattern(matcher = {
        char('[') + char(..).repeat(..).lazy() + exactly("]]>")
    })]
    pub struct VerbatimTail0;

    #[pattern(matcher = {
        exactly("#[") + char(..).repeat(..).lazy() + exactly("]#]>")
    })]
    pub struct VerbatimTail1;

    #[pattern(matcher = {
        exactly("##[") + char(..).repeat(..).lazy() + exactly("]##]>")
    })]
    pub struct VerbatimTail2;

    #[pattern(matcher = {
        exactly("\"\"\"") + (!char("\n\"")).repeat(..).simple() + char("\n")
        + char(..).repeat(..).lazy()
        + line_start() + char(" \t").repeat(..).simple() + exactly("\"\"\"")
    })]
    pub struct MultilineEscaped;

    #[pattern(matcher = {
        exactly("'''") + (!char("\n'")).repeat(..).simple() + char("\n")
        + char(..).repeat(..).lazy()
        + line_start() + char(" \t").repeat(..).simple() + exactly("'''")
    })]
    pub struct MultilineUnescaped;

    #[pattern(matcher = {
        exactly("```") + (!char("\n`")).repeat(..).simple() + char("\n")
        + char(..).repeat(..).lazy()
        + line_start() + char(" \t").repeat(..).simple() + exactly("```")
    })]
    pub struct MultilineCode;
);

gramma::define_rule!(
    pub struct Block {
        pub l_brace: LeftBrace,
        #[transform(ignore_around<Whitespace>)]
        pub nodes: Option<Nodes>,
        pub r_brace: RightBrace,
    }

    pub struct TextLine {
        pub part1: TextLinePart,
        pub parts: Vec<(Option<Space>, TextLinePart)>,
    }

    pub enum TextLinePart {
        #[transform(not<InvalidTextSegment>)]
        TextSegment {
            text: TextSegment,
        },
        InlineSpecial {
            inline_special: InlineSpecial,
        },
        Inline {
            inline: Inline,
        },
        NonParagraph {
            node: NonParagraphNode,
        },
    }

    pub struct Verbatim {
        pub open: VerbatimOpen,
        pub raw: Option<KeywordRaw>,
        pub tail: VerbatimTail,
    }

    pub enum VerbatimTail {
        Verbatim0 { value: VerbatimTail0 },
        Verbatim1 { value: VerbatimTail1 },
        Verbatim2 { value: VerbatimTail2 },
    }

    pub enum Multiline {
        Escaped { value: MultilineEscaped },
        Unescaped { value: MultilineUnescaped },
    }

    pub struct Comment {
        pub open: OpenComment,
        #[transform(parse_as<Vec<CommentPart>>)]
        pub inner: LocationRange,
        pub close: CloseComment,
    }

    pub enum CommentPart {
        Text { text: CommentText },
        Comment { comment: Box<Comment> },
    }

    pub struct Inline {
        pub open: OpenInline,
        #[transform(ignore_around<Whitespace>)]
        pub inner: Option<Box<Node>>,
        pub close: CloseInline,
    }

    #[non_exhaustive]
    pub enum InlineSpecial {
        Emphasis {
            open: OpenEmphasis,
            inner: Nodes,
            close: CloseEmphasis,
        },
        Strong {
            open: OpenStrong,
            inner: Nodes,
            close: CloseStrong,
        },
        Underline {
            open: OpenUnderline,
            inner: Nodes,
            close: CloseUnderline,
        },
        Strike {
            open: OpenStrike,
            inner: Nodes,
            close: CloseStrike,
        },
        Quote {
            open: OpenQuote,
            inner: Nodes,
            close: CloseQuote,
        },
        Code {
            code: InlineCode,
        },
    }

    pub enum ParagraphItem {
        Multiline { multiline: Multiline },
        Line { line: TextLine },
    }

    pub struct Paragraph {
        pub line1: ParagraphItem,
        pub lines: Vec<(NewLine, ParagraphItem)>,
    }

    pub enum ElementBody {
        #[transform(ignore_before<Space>)]
        Block { block: Block },

        LineBlock {
            #[transform(ignore_around<Space>)]
            angle: RightAngle,
            block: Block,
        },

        Line {
            #[transform(ignore_around<Space>)]
            angle: RightAngle,
            body: Option<Box<Node>>,
        },
    }

    pub enum Element {
        WithSelector {
            selector: Selector,
            body: ElementBody,
        },
        Body {
            body: ElementBody,
        },
    }

    pub struct NonParagraphNode {
        pub start: Location,
        pub node_type: NonParagraphNodeType,
        pub end: Location,
    }

    pub enum NonParagraphNodeType {
        Verbatim { verbatim: Verbatim },
        Comment { comment: Comment },
    }

    pub struct Node {
        pub start: Location,
        pub node_type: NodeType,
        pub end: Location,
    }

    pub enum NodeType {
        NonParagraph { node: NonParagraphNode },
        MultilineCode { multiline: MultilineCode },
        Element { element: Element },
        Paragraph { paragraph: Paragraph },
    }

    #[transform(ignore_after<Whitespace>)]
    pub struct Nodes {
        #[transform(for_each<ignore_before<Whitespace>>, delimited<NewLine, false>)]
        pub nodes: Vec<Node>,
    }

    #[transform(parse_as<SelectorString>)]
    pub struct Selector {
        pub element: Option<ElementSelector>,
        pub parts: Vec<SelectorPart>,
    }

    pub enum SelectorPart {
        Attribute { value: AttributeSelector },
        ClassSelector { value: ClassSelector },
        IdSelector { value: IdSelector },
    }

    pub enum ElementSelector {
        Name { name: ElementName },
        Star { star: Star },
    }

    pub struct AttributeSelector {
        pub l_bracket: LeftBracket,
        pub parts: Vec<Attribute>,
        #[transform(ignore_before<Whitespace>)]
        pub r_bracket: RightBracket,
    }

    #[transform(ignore_before<Whitespace>)]
    pub struct Attribute {
        pub name: AttributeName,
        pub assignment: Option<AttributeAssignment>,
    }

    pub struct AttributeAssignment {
        #[transform(ignore_around<Whitespace>)]
        pub eq: Equals,
        pub value: AttributeValue,
    }

    pub enum AttributeValue {
        Unquoted { value: UnquotedAttributeValue },
        Quoted { value: QuotedString },
    }

    pub struct ClassSelector {
        pub dot: Dot,
        pub ident: ClassName,
    }

    pub struct IdSelector {
        pub hash: Hash,
        pub ident: ClassName,
    }

    #[transform(ignore_around<Whitespace>)]
    pub struct Document {
        pub start: Location,
        pub nodes: Option<Nodes>,
        pub end: Location,
    }
);

#[test]
fn ast_demo() {
    use gramma::ast::parse_tree;

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
    let _ast = parse_tree::<Document, 3>(src).unwrap();
    #[cfg(feature = "std")]
    {
        ::std::println!("{:#}", gramma::display_tree(src, &_ast));
    }
}

#[test]
fn ast_demo2() {
    use gramma::ast::parse_tree;

    let src = r#"
section {
    a

    b
}
    "#;
    let _ast = parse_tree::<Document, 2>(src).unwrap();
    #[cfg(feature = "std")]
    {
        ::std::println!("{:#}", gramma::display_tree(src, &_ast));
    }
}
