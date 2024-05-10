use mintyml::OutputConfig;
use utils::convert_unwrap;

mod utils;

#[test]
fn comment_does_not_create_paragraph() {
    let src = r#"
    foo

    <!bar!>

    baz
    "#;
    let out = convert_unwrap(src, None);

    assert_eq!(out, "<p>foo</p> <!--bar--> <p>baz</p>")
}

#[test]
fn comment_does_not_split_paragraph() {
    let src = r#"
    foo
    <!bar!>
    baz
    "#;
    let out = convert_unwrap(src, None);

    assert_eq!(out, "<p>foo <!--bar--> baz</p>")
}

#[test]
fn comment_does_not_escape_html() {
    let src = r#"
    foo
    <! <>
    bar
    !>
    baz
    "#;
    {
        let out = convert_unwrap(src, None);

        assert_eq!(
            out, "<p>foo <!-- <>\n    bar\n    --> baz</p>",
            "comments should not escape in HTML"
        );
    }
    {
        let out = convert_unwrap(src, OutputConfig::new().xml(true));

        assert_eq!(
            out, "<p>foo <!-- <>\n    bar\n    --> baz</p>",
            "comments should not escape in XML"
        )
    }
}

#[test]
fn nested_comment() {
    let src = r#"
    A
    <!B <! C D!> E !>
    F
    "#;
    let out = convert_unwrap(src, None);

    assert_eq!(out, "<p>A <!--B <! C D!> E --> F</p>")
}

#[test]
fn comment_separates_double_dash() {
    let src = r#"
    A
    <! B -- C !>
    F
    "#;
    let out = convert_unwrap(src, None);

    assert_eq!(out, "<p>A <!-- B - - C --> F</p>")
}

#[test]
fn comment_separates_triple_dash() {
    let src = r#"
    A
    <! B --- C !>
    F
    "#;
    let out = convert_unwrap(src, None);

    assert_eq!(out, "<p>A <!-- B - - - C --> F</p>")
}

#[test]
fn comment_separates_opening_right_angle() {
    let src = r#"
    A
    <!> B C !>
    F
    "#;
    let out = convert_unwrap(src, None);

    assert_eq!(out, "<p>A <!-- > B C --> F</p>")
}

#[test]
fn comment_separates_opening_right_arrow() {
    let src = r#"
    A
    <!-> B C !>
    F
    "#;
    let out = convert_unwrap(src, None);

    assert_eq!(out, "<p>A <!-- -> B C --> F</p>")
}

#[test]
fn comment_separates_closing_dash() {
    let src = r#"
    A
    <! B C -!>
    F
    "#;
    let out = convert_unwrap(src, None);

    assert_eq!(out, "<p>A <!-- B C - --> F</p>")
}