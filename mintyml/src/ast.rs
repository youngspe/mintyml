mod elements;
mod selectors;
mod text;
mod tokens;

use alloc::vec::Vec;
use gramma::ast::Discard;
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
        Text { text: InlineText },
        Selector { selector: Selector },
        Element { element: Element },
    }

    pub enum Line {
        EmptyLine {
            #[transform(ignore_after<Whitespace>)]
            _newline: Discard<NewLine>,
        },
        #[transform(ignore_around<Space>)]
        NonEmptyLine {
            start: Location,
            nodes: Vec<(Option<Space>, Node)>,
            end: Location,
        },
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

#[cfg(test)]
mod test {
    use gramma::{parse::LocationRange, Token};

    use super::*;

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
    fn text_segment_alphanumeric_multi_word_lexes_before_gt() {
        let src = ["abc1 abc1> ", "abcd abcd> "];

        for src in src {
            assert_eq!(
                TextSegment::try_lex(src, Location { position: 0 }),
                Some(LocationRange {
                    start: Location { position: 0 },
                    end: Location { position: 9 },
                }),
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
        let _ast = parse_tree::<Document, 2>(src).unwrap();
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
}
