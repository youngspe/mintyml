use alloc::{boxed::Box, vec::Vec};
use gramma::parse::{Location, LocationRange};

use super::tokens::*;

gramma::define_string_pattern!(
    fn text_word() {
        repeat(
            1..,
            (!char("\\[]{}<> \n\r\t") & !whitespace()) + !precedes(char('>'))
                | alphanumeric()
                | escape(),
        )
        .simple()
            + !precedes(space().repeat(..).simple() + (char(">[{")))
    }

    fn interpolation() {
        !follows(char("\\{"))
            + exactly("{{")
            + !precedes(char('{'))
            + char(..).repeat(..).lazy()
            + !follows(char("\\}"))
            + exactly("}}")
            + !precedes(char('}'))
            | exactly("{%") + char(..).repeat(..).lazy() + exactly("%}")
            | exactly("<%") + char(..).repeat(..).lazy() + exactly("%>")
            | exactly("<?") + char(..).repeat(..).lazy() + exactly("?>")
    }
);

gramma::define_token!(
    #[pattern(matcher = {
        text_word() + repeat(.., space() + text_word()).simple()
    })]
    /// Matches any part of a paragraph line that is not an element.
    pub struct TextSegment;

    #[pattern(matcher = {
        exactly("```") | exactly(r#"""""#) | exactly("'''")
    })]
    /// Fail if TextSegment matches this so it doesn't match multiline elements.
    pub struct InvalidTextSegment;

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
    #[pattern(matcher = interpolation())]
    pub struct Interpolation;

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
);

gramma::define_rule!(
    pub enum InlineText {
        Segment { value: TextSegment },
        Verbatim { value: Verbatim },
        Comment { comment: Comment },
        Interpolation { interpolation: Interpolation },
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
        pub start: Location,
        pub open: OpenComment,
        #[transform(parse_as<Vec<CommentPart>>)]
        pub inner: LocationRange,
        pub close: Option<CloseComment>,
        pub end: Location,
    }

    pub enum CommentPart {
        Text { text: CommentText },
        Comment { comment: Box<Comment> },
    }
);
