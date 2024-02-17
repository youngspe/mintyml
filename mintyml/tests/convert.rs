use mintyml::OutputConfig;

const SIMPLE_SRC: &'static str = r#"
    {
        foo bar
        baz
        img[src="./pic.png"]>

        > <(a.foo[href=www.example.com]> a )>
        b
        .foo>
        .foo#bar.baz> c
    }
"#;

#[test]
fn simple_doc() {
    let out = mintyml::convert(SIMPLE_SRC, OutputConfig::new()).unwrap();

    assert_eq!(
        out,
        concat!(
            r#"<div>"#,
            r#"<p>foo bar baz</p>"#,
            r#"<img src="./pic.png">"#,
            r#"<p><a class="foo" href="www.example.com">a</a> b</p>"#,
            r#"<p class="foo"></p>"#,
            r#"<p id="bar" class="foo baz">c</p>"#,
            r#"</div>"#,
        )
    );
}

#[test]
fn simple_doc_xml() {
    let out = mintyml::convert(SIMPLE_SRC, OutputConfig::new().xml(true)).unwrap();

    assert_eq!(
        out,
        concat!(
            r#"<div>"#,
            r#"<p>foo bar baz</p>"#,
            r#"<img src="./pic.png"/>"#,
            r#"<p><a class="foo" href="www.example.com">a</a> b</p>"#,
            r#"<p class="foo"/>"#,
            r#"<p id="bar" class="foo baz">c</p>"#,
            r#"</div>"#,
        )
    );
}

#[test]
fn special_tags() {
    let src = r#"
        section {
            Abc </def/><#ghi#>?
            <_jkl_>

            mno <~pqr <"stu"> vwx~> yz
        }
    "#;

    let out = mintyml::convert(src, OutputConfig::new()).unwrap();

    assert_eq!(
        out,
        concat!(
            r#"<section>"#,
            r#"<p>Abc <em>def</em><strong>ghi</strong>? <u>jkl</u></p>"#,
            r#"<p>mno <s>pqr <q>stu</q> vwx</s> yz</p>"#,
            r#"</section>"#,
        )
    )
}
