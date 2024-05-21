use alloc::vec::Vec;

use gramma::parse::{Location, LocationRange};

use super::tokens::*;

gramma::define_string_pattern!(
    fn identifier_char() {
        char((ascii_alphanumeric(), "-:")) | escape()
    }

    fn class_name() {
        (!char(("\\{}[]()<>`~!@#$%^&*+=,./?\"'|; \t\r\n", whitespace())))
            .repeat(1..)
            .simple()
    }

    fn attr_string() {
        char('\'') + (!char(('\\', '\'')) | escape()).repeat(1..).simple() + char('\'')
            | char('"') + (!char(('\\', '"')) | escape()).repeat(1..).simple() + char('"')
    }

    fn element_name() {
        !precedes(ascii_digit()) + identifier_char().repeat(1..).simple()
    }
    fn selector_chain() {
        repeat(
            1..,
            (!char("\\[]{}<> \n\r\t") & !whitespace()) + !precedes(char('>'))
                | char((alphanumeric(), "*"))
                | escape(),
        )
        .simple()
    }
    pub fn post_selector_chain() {
        char(" \t").repeat(..).simple()
            + (char('>') | char('{') + !precedes(!(whitespace() | char('}'))))
            | char('[')
            | src_end()
    }
);

gramma::define_token!(
    #[pattern(matcher = element_name())]
    pub struct ElementName;
    #[pattern(matcher = class_name())]
    pub struct ClassName;

    #[pattern(matcher = {
        selector_chain() + precedes(post_selector_chain())
    })]
    pub struct SelectorChain;

    #[pattern(matcher = {
        repeat(1.., !char(("=>'\"/[]\\", whitespace())) | escape()).simple()
    })]
    pub struct AttributeName;

    #[pattern(matcher = {
        repeat(1.., !char(("[]\\\"'", whitespace())) | escape()).simple()
    })]
    pub struct UnquotedAttributeValue;
);

gramma::define_rule!(
    pub struct Selector {
        pub start: Location,
        pub first: SelectorStart,
        pub segments: Vec<SelectorSegment>,
        pub end: Location,
    }

    pub enum SelectorStart {
        Attributes {
            attributes: AttributeSelector,
        },
        #[transform(parse_as<SelectorChain>)]
        Tag {
            element: Option<ElementSelector>,
            class_like: Vec<ClassLike>,
        },
    }

    pub enum SelectorSegment {
        Attributes {
            attributes: AttributeSelector,
        },
        ClassLike {
            #[transform(parse_as<SelectorChain>)]
            items: Vec<ClassLike>,
        },
    }

    pub enum ClassLike {
        Class {
            value: ClassSelector,
        },
        Id {
            value: IdSelector,
        },
        Invalid {
            #[transform(parse_as<AnyChars>)]
            range: LocationRange,
        },
    }

    pub enum ElementSelector {
        Name { name: ElementName },
        Star { star: Star },
    }

    pub struct AttributeSelector {
        pub start: Location,
        pub l_bracket: LeftBracket,
        pub parts: Vec<Attribute>,
        #[transform(ignore_before<Whitespace>)]
        pub r_bracket: Option<RightBracket>,
        pub end: Location,
    }

    #[transform(ignore_before<Whitespace>)]
    pub struct Attribute {
        pub start: Location,
        pub name: AttributeName,
        pub assignment: Option<AttributeAssignment>,
        pub end: Location,
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
);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn selector_example() {
        gramma::parse_tree::<Selector, 1>("a.b#c[x].b").unwrap();
    }

    #[test]
    fn selector_empty_repeated() {
        gramma::parse_tree::<Vec<Selector>, 1>("").unwrap();
    }

    #[test]
    fn selector_example_repeated() {
        super::super::test::parse_as_vec::<Selector>("a.b#c[x].b");
    }

    /// Test that in invalid selector parses correctly so we can report it as an error later.
    #[test]
    fn invalid_selector() {
        let src = "a.b#c[x]d.e";
        let selector = gramma::parse_tree::<Selector, 1>(src).unwrap();

        let SelectorSegment::ClassLike { ref items } = selector.segments[1] else {
            panic!();
        };

        let ClassLike::Invalid { range } = items[0] else {
            panic!();
        };

        assert_eq!(range.slice(src), "d.e");
    }
}
