use alloc::vec::Vec;

use gramma::parse::LocationRange;

use super::{tokens::*, Content, Node};

gramma::define_token!(
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
        #[transform(discard_before<Whitespace>)]
        pub content: Content,
        #[transform(ignore_before<Whitespace>)]
        pub r_brace: Option<RightBrace>,
    }

    pub struct Inline {
        pub open: OpenInline,
        #[transform(ignore_around<Whitespace>)]
        pub inner: Vec<(Option<Space>, Node)>,
        pub close: Option<CloseInline>,
    }

    #[non_exhaustive]
    pub enum InlineSpecial {
        #[non_exhaustive]
        Emphasis {
            open: OpenEmphasis,
            inner: Content,
            close: Option<CloseEmphasis>,
        },
        #[non_exhaustive]
        Strong {
            open: OpenStrong,
            inner: Content,
            close: Option<CloseStrong>,
        },
        #[non_exhaustive]
        Underline {
            open: OpenUnderline,
            inner: Content,
            close: Option<CloseUnderline>,
        },
        #[non_exhaustive]
        Strike {
            open: OpenStrike,
            inner: Content,
            close: Option<CloseStrike>,
        },
        #[non_exhaustive]
        Quote {
            open: OpenQuote,
            inner: Content,
            close: Option<CloseQuote>,
        },
        #[non_exhaustive]
        Code { code: InlineCode },
    }

    #[non_exhaustive]
    pub enum Element {
        #[non_exhaustive]
        Line {
            #[transform(parse_as<RightAngle>)]
            combinator: LocationRange,
        },
        #[non_exhaustive]
        Block { value: Block },
        #[non_exhaustive]
        MultilineCode { value: MultilineCode },
        #[non_exhaustive]
        Inline { value: Inline },
        #[non_exhaustive]
        InlineSpecial { value: InlineSpecial },
    }
);
