mod elements;
mod selectors;
mod text;
mod tokens;

use alloc::vec::Vec;
use gramma::ParseError;
// TODO: figure out why importing isn't working and change this to an import
type Location = gramma::parse::Location;

pub(crate) use elements::*;
pub(crate) use selectors::*;
pub(crate) use text::*;
pub(crate) use tokens::*;

gramma::define_rule!(
    pub struct Node {
        pub start: Location,
        pub node_type: NodeType,
        pub end: Location,
    }

    pub enum NodeType {
        Multiline { multiline: Multiline },
        Text { text: InlineText },
        Selector { selector: Selector },
        Element { element: Element },
    }

    pub struct Line {
        pub start: Location,
        #[transform(ignore_around<Space>)]
        pub nodes: Vec<(Option<Space>, Node)>,
        pub end: Location,
    }

    #[transform(ignore_after<Whitespace>)]
    pub struct Content {
        pub start: Location,
        #[transform(ignore_before<Whitespace>, delimited<NewLine, false>)]
        pub lines: Vec<Line>,
        pub end: Location,
    }

    #[transform(ignore_around<Whitespace>)]
    pub struct Document {
        pub content: Content,
    }
);

const LOOKAHEAD: usize = 2;

pub fn parse(src: &str) -> Result<Document, ParseError> {
    gramma::parse_tree::<Document, LOOKAHEAD>(src)
}

#[cfg(test)]
mod test {
    use super::*;
    use gramma::{parse::LocationRange, Token};

    #[test]
    fn text_segment_excludes_line_breaks() {
        let src = ["foo\nbar", "foo\n bar", "foo \n bar", "foo \nbar"];

        for src in src {
            assert_eq!(
                TextSegment::try_lex(src, Location { position: 0 }).unwrap(),
                LocationRange {
                    start: Location { position: 0 },
                    end: Location { position: 3 },
                },
            );
        }
    }

    #[test]
    fn text_segment_cannot_start_with_whitespace() {
        assert!(TextSegment::try_lex(" foo", Location { position: 0 }).is_none());
    }

    #[test]
    fn text_segment_can_start_after_whitespace() {
        assert_eq!(
            TextSegment::try_lex(" foo", Location { position: 1 }).unwrap(),
            LocationRange {
                start: Location { position: 1 },
                end: Location { position: 4 },
            },
        );
    }

    #[test]
    fn text_segment_alphanumeric_word_fails_before_gt() {
        let src = ["abc1> ", "abcd> "];

        for src in src {
            assert_eq!(
                TextSegment::try_lex(src, Location { position: 0 }),
                None,
                "for {src:?}"
            );
        }
    }

    #[test]
    fn text_segment_non_alphanumeric_before_gt_excluded() {
        let src = ["abc)> ", "abc?> ", "abc,>>\n"];

        for src in src {
            assert_eq!(
                TextSegment::try_lex(src, Location { position: 0 }),
                Some(LocationRange {
                    start: Location { position: 0 },
                    end: Location { position: 3 },
                }),
                "for {src:?}"
            )
        }
    }

    #[test]
    fn selector_chain_alphanumeric_lexes_before_gt() {
        let src = ["abc1> ", "abcd> "];

        for src in src {
            assert_eq!(
                SelectorChain::try_lex(src, Location { position: 0 }),
                Some(LocationRange {
                    start: Location { position: 0 },
                    end: Location { position: 4 },
                }),
            );
        }
    }

    #[test]
    fn selector_chain_non_alphanumeric_before_gt_excluded() {
        let src = ["abc)> ", "abc?> ", "abc,>>\n"];

        for src in src {
            assert_eq!(
                SelectorChain::try_lex(src, Location { position: 0 }),
                None,
                "for {src:?}"
            )
        }
    }

    #[test]
    fn ast_demo() {
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
        let _ast = parse(src).unwrap();
        #[cfg(feature = "std")]
        {
            ::std::println!("{:#}", gramma::display_tree(src, &_ast));
        }
    }

    #[test]
    fn ast_demo2() {
        let src = r#"
section {
    a

    b
}
    "#;
        let _ast = parse(src).unwrap();
        #[cfg(feature = "std")]
        {
            ::std::println!("{:#}", gramma::display_tree(src, &_ast));
        }
    }

    #[test]
    fn empty_line() {
        let src = "";
        let line = gramma::parse_tree::<Line, LOOKAHEAD>(src).unwrap();
        assert_eq!(line.nodes.len(), 0);
    }

    #[test]
    fn wrapped_line() {
        let src = "{ foo }";
        let line = gramma::parse_tree::<(LeftBrace, Line, RightBrace), 2>(src)
            .unwrap()
            .1;
        assert_eq!(line.nodes.len(), 1);
    }

    #[test]
    fn multi_paragraph_block() {
        let src = "{
    foo

    foo
}";
        parse_as_vec::<Node>(src);
    }

    #[track_caller]
    pub fn parse_as_vec_before<R: gramma::Rule, Post: gramma::Rule>(src: &str) -> R {
        let ast_vec = gramma::parse_tree::<(Vec<R>, Post), LOOKAHEAD>(src)
            .unwrap()
            .0;
        assert_eq!(ast_vec.len(), 1);
        ast_vec.into_iter().next().unwrap()
    }

    #[track_caller]
    pub fn parse_as_vec<R: gramma::Rule>(src: &str) -> R {
        parse_as_vec_before::<R, ()>(src)
    }

    #[test]
    fn right_arrow_node() {
        let src = ">";
        let node = parse_as_vec::<Node>(src);
        assert!(matches!(
            node,
            Node {
                node_type: NodeType::Element {
                    element: Element::Line { .. }
                },
                ..
            }
        ));
    }
}
