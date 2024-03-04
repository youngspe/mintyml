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
