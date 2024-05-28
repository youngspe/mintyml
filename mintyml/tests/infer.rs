use mintyml::OutputConfig;
use utils::convert_unwrap;

mod utils;

#[test]
fn list_with_paragraph_items_after_paragraph() {
    let src = r#"
    paragraph

    ol {
        a

        b
    }
    "#;

    let out = convert_unwrap(src, OutputConfig::new());

    assert_eq!(
        out,
        concat!(
            r#"<p>paragraph</p> "#,
            r#"<ol>"#,
            r#"<li>a</li> <li>b</li>"#,
            r#"</ol>"#,
        )
    )
}

#[test]
fn table_nested_in_div() {
    let src = r#"
    >table {
      thead>> <(A)> <(B)>

      <(C)> <(D)>
    }
    "#;

    let out = convert_unwrap(src, OutputConfig::new());

    assert_eq!(
        out,
        concat!(
            r#"<div><table>"#,
            r#"<thead>"#,
            r#"<tr><th>A</th> <th>B</th></tr>"#,
            r#"</thead> "#,
            r#"<tr><td>C</td> <td>D</td></tr>"#,
            r#"</table></div>"#,
        )
    )
}
