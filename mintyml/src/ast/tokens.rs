gramma::define_string_pattern!(
    pub(super) fn unicode_escape() {
        exactly("\\u{") + repeat(.., !char(('\n', '{', '}'))).simple() + exactly("}")
    }

    pub(super) fn escape() {
        unicode_escape() | char('\\') + char(..)
    }

    pub(super) fn space() {
        repeat(1.., whitespace() & !char('\n')).simple()
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

    #[pattern(matcher = {
        (whitespace() & !char('\n')).repeat(..).simple()
        + char('\n')
        + (whitespace() & !char('\n')).repeat(..).simple()
    })]
    pub struct NewLine;

    #[pattern(matcher = (whitespace() & !char('\n')).repeat(1..))]
    pub struct Space;
    #[pattern(matcher = whitespace().repeat(1..))]
    pub struct Whitespace;

    #[pattern(matcher = {
        char('"') + (!char("\\\"") | escape()).repeat(..).simple() + char('"')
        | char('\'') + (!char("\\'") | escape()).repeat(..).simple() + char('\'')
    })]
    pub struct QuotedString;

    #[pattern(matcher = char(..).repeat(1..).simple())]
    pub struct AnyChars;
);
