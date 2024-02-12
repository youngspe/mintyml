use alloc::{boxed::Box, vec::Vec};
use gramma::{ast::Ignore, parse::LocationRange};

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
    #[pattern(regex = r"[a-zA-Z][:a-zA-Z0-9\-]*")]
    pub struct ElementName;
    #[pattern(regex = r"(?x)(
        [ : a-z A-Z 0-9 \- ]
        | \\.
    )+")]
    pub struct Ident;

    #[pattern(regex = r"(?xm) (
        [\  \t]* (
            [^ < > \) \\ \{ \} \s ]
            | \\ ( . | u\{[^ \} \s ]*\} )
            | < [^ \( ! \n ]
            | \) [^ > \n ]
            | [ < \) ] $
        )+
    )+")]
    pub struct TextSegment;

    #[pattern(regex = r"[ \t]*\n[ \t]*")]
    pub struct NewLine;
    #[pattern(regex = r"[ \t]+")]
    pub struct Space;
    #[pattern(regex = r"\s+")]
    pub struct Whitespace;

    #[pattern(regex = r#"(?x) (
        # wildcard element:
        \*

        # element, class, or id:
        | [ \. \# ]? (
            [ : a-z A-Z 0-9 \- ] | \\.
        )+

        # attribute:
        | \[ (
            [^ \[ \] \\ " ' ]
            | \\.
            | "( [^ \\ " ] | \\. )*"
            | '( [^ \\ ' ] | \\. )*'
        )* \]
    )+"#)]
    pub struct SelectorString;

    #[pattern(regex = r#"(?x)
        [^ \s = > ' " / \] \[ ]+
    "#)]
    pub struct AttributeName;

    #[pattern(regex = r#"(?x)(
        [^ \s \[ \] \\ " ' ]
        | \\.
    )+"#)]
    pub struct UnquotedAttributeValue;

    #[pattern(regex = r#"(?x)
        " ( [^ \\ " ] | \\. )* "
        | ' ( [^ \\ ' ] | \\. )* '
    "#)]
    pub struct QuotedString;

    #[pattern(regex = r"(?xm) (
        [^ < ! ]
        | < [^ ! \n\ ]
        | ! [^ > \n ]
        | [ < ! ] $
    )+")]
    pub struct CommentText;

    #[pattern(exact = "<!")]
    pub struct OpenComment;
    #[pattern(exact = "!>")]
    pub struct CloseComment;
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
        TextSegment { text: TextSegment },
        Inline { inline: Inline },
        Comment { comment: Comment },
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

    pub struct Paragraph {
        pub line1: TextLine,
        #[transform(for_each<discard_before<NewLine>>)]
        pub lines: Vec<TextLine>,
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

    pub enum Node {
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
        pub ident: Ident,
    }

    pub struct IdSelector {
        pub hash: Hash,
        pub ident: Ident,
    }

    #[transform(ignore_around<Whitespace>)]
    pub struct Document {
        pub nodes: Option<Nodes>,
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