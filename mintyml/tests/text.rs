use crate::utils::convert_unwrap;

mod utils;

#[test]
fn interpolation_angle_mod() {
    let src = r#"
        section { <%foo </bar/>%>baz }
    "#;

    let out = convert_unwrap(src, None);

    assert_eq!(
        out,
        concat!(
            r#"<section>"#,
            r#"<p><%foo </bar/>%>baz</p>"#,
            r#"</section>"#,
        )
    )
}

#[test]
fn interpolation_angle_qmark() {
    let src = r#"
        section { <?foo </bar/>?>baz }
    "#;

    let out = convert_unwrap(src, None);

    assert_eq!(
        out,
        concat!(
            r#"<section>"#,
            r#"<p><?foo </bar/>?>baz</p>"#,
            r#"</section>"#,
        )
    )
}

#[test]
fn interpolation_curly_mod() {
    let src = r#"
        section { {%foo </bar/>%}baz }
    "#;

    let out = convert_unwrap(src, None);

    assert_eq!(
        out,
        concat!(
            r#"<section>"#,
            r#"<p>{%foo </bar/>%}baz</p>"#,
            r#"</section>"#,
        )
    )
}

#[test]
fn interpolation_curly_double() {
    let src = r#"
        section { {{foo </bar/>}}baz }
    "#;

    let out = convert_unwrap(src, None);

    assert_eq!(
        out,
        concat!(
            r#"<section>"#,
            r#"<p>{{foo </bar/>}}baz</p>"#,
            r#"</section>"#,
        )
    )
}

#[test]
fn interpolation_curly_double_follows_single_word() {
    let src = r#"
        foo {{ </bar/> }}
    "#;

    let out = convert_unwrap(src, None);

    assert_eq!(
        out,
        concat!(
            r#"<p>foo {{ </bar/> }}</p>"#,
        )
    )
}
