mod utils;

use mintyml::{error::UnclosedDelimiterKind, ConvertError, SyntaxError, SyntaxErrorKind};
use utils::convert_fail;

/// Test that an unclosed block is detected and a best-effort parsing is used.
#[test]
fn unclosed_block() {
    let src = r#"
        dl {
            > term1
            {
                details1

            > term2

            { details2 }
        }
    "#;

    let (partial, e) = convert_fail(src, None);

    let ConvertError::Syntax { syntax_errors, .. } = e else {
        panic!()
    };
    match_set!(
        &syntax_errors,
        [
            |SyntaxError {
                 kind:
                     SyntaxErrorKind::Unclosed {
                         delimiter: UnclosedDelimiterKind::Block { .. },
                         ..
                     },
                 range,
                 ..
             }| range.start.position == 12 && range.end.position == 13,
        ],
    );

    assert_eq!(
        partial.unwrap(),
        concat!(
            r#"<dl>"#,
            r#"<dt>term1</dt>"#,
            r#" <dd><p>details1</p>"#,
            r#" <p>term2</p>"#,
            r#" <div><p>details2</p></div>"#,
            r#"</dd></dl>"#,
        )
    );
}

#[test]
fn unclosed_special() {
    let src = r#"
        section {
            Abc </def/><#ghi#>?
            <_jkl

            mno <~pqr <"stu"> vwx~> yz

            An <`inline {code <block>}`>
        }
    "#;

    let (out, e) = convert_fail(src, None);

    let ConvertError::Syntax { syntax_errors, .. } = e else {
        panic!()
    };

    match_set!(
        &syntax_errors,
        [
            |SyntaxError {
                 kind:
                     SyntaxErrorKind::Unclosed {
                         delimiter: UnclosedDelimiterKind::SpecialInline { .. },
                         ..
                     },
                 range,
                 ..
             }| range.start.position == 63 && range.end.position == 65,
        ],
    );

    assert_eq!(
        out.unwrap(),
        concat!(
            r#"<section>"#,
            r#"<p>Abc <em>def</em><strong>ghi</strong>? <u>jkl"#,
            r#" mno <s>pqr <q>stu</q> vwx</s> yz"#,
            r#" An <code>inline {code &lt;block&gt;}</code>"#,
            r#"</u></p></section>"#,
        )
    )
}

#[test]
fn double_unclosed_special() {
    let src = r#"
        section {
            Abc </def/><#ghi#>?
            <_jkl

            mno <~pqr <"stu vwx~> yz

            An <`inline {code <block>}`>
        }
    "#;

    let (out, e) = convert_fail(src, None);

    let ConvertError::Syntax { syntax_errors, .. } = e else {
        panic!()
    };

    match_set!(
        &syntax_errors,
        [
            |SyntaxError {
                 kind:
                     SyntaxErrorKind::Unclosed {
                         delimiter: UnclosedDelimiterKind::SpecialInline { .. },
                         ..
                     },
                 range,
                 ..
             }| range.start.position == 63 && range.end.position == 65,
            |SyntaxError {
                 kind:
                     SyntaxErrorKind::Unclosed {
                         delimiter: UnclosedDelimiterKind::SpecialInline { .. },
                         ..
                     },
                 range,
                 ..
             }| range.start.position == 92 && range.end.position == 94,
        ],
    );

    assert_eq!(
        out.unwrap(),
        concat!(
            r#"<section>"#,
            r#"<p>Abc <em>def</em><strong>ghi</strong>? <u>jkl"#,
            r#" mno <s>pqr <q>stu vwx</q></s> yz"#,
            r#" An <code>inline {code &lt;block&gt;}</code>"#,
            r#"</u></p></section>"#,
        )
    )
}

#[test]
fn unclosed_inline() {
    let src = r#"
        section {
            foo <(bar baz
        }
    "#;

    let (out, e) = convert_fail(src, None);

    let ConvertError::Syntax { syntax_errors, .. } = e else {
        panic!()
    };

    match_set!(
        &syntax_errors,
        [
            |SyntaxError {
                 kind:
                     SyntaxErrorKind::Unclosed {
                         delimiter: UnclosedDelimiterKind::Inline { .. },
                         ..
                     },
                 range,
                 ..
             }| range.start.position == 35 && range.end.position == 37,
        ],
    );

    assert_eq!(
        out.unwrap(),
        concat!(
            r#"<section>"#,
            r#"<p>foo <span>bar baz</span></p>"#,
            r#"</section>"#,
        )
    )
}

#[test]
fn unclosed_comment() {
    let src = r#"
    foo
    <!bar
    baz
    "#;

    let (out, e) = convert_fail(src, None);

    let ConvertError::Syntax { syntax_errors, .. } = e else {
        panic!()
    };

    match_set!(
        &syntax_errors,
        [
            |SyntaxError {
                 kind:
                     SyntaxErrorKind::Unclosed {
                         delimiter: UnclosedDelimiterKind::Comment { .. },
                         ..
                     },
                 range,
                 ..
             }| range.start.position == 13 && range.end.position == 15,
        ],
    );

    assert_eq!(out.unwrap(), "<p>foo <!--bar\n    baz\n    --></p>")
}
