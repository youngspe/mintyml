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
            r#" <img src="./pic.png">"#,
            r#" <p><a class="foo" href="www.example.com">a</a> b</p>"#,
            r#" <p class="foo"></p>"#,
            r#" <p id="bar" class="foo baz">c</p>"#,
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
            r#" <img src="./pic.png"/>"#,
            r#" <p><a class="foo" href="www.example.com">a</a> b</p>"#,
            r#" <p class="foo"/>"#,
            r#" <p id="bar" class="foo baz">c</p>"#,
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

            An <`inline {code <block>}`>
        }
    "#;

    let out = mintyml::convert(src, OutputConfig::new()).unwrap();

    assert_eq!(
        out,
        concat!(
            r#"<section>"#,
            r#"<p>Abc <em>def</em><strong>ghi</strong>? <u>jkl</u></p>"#,
            r#" <p>mno <s>pqr <q>stu</q> vwx</s> yz</p>"#,
            r#" <p>An <code>inline {code &lt;block&gt;}</code></p>"#,
            r#"</section>"#,
        )
    )
}

#[test]
fn paragraph_infer_inline() {
    let src = r#"
        section {
            foo <(bar)> baz
        }
    "#;

    let out = mintyml::convert(src, OutputConfig::new()).unwrap();

    assert_eq!(
        out,
        concat!(
            r#"<section>"#,
            r#"<p>foo <span>bar</span> baz</p>"#,
            r#"</section>"#,
        )
    )
}

#[test]
fn paragraph_infer_inline_with_attr() {
    let src = r#"
        section {
            foo <([style='color: red']>bar)> baz
        }
    "#;

    let out = mintyml::convert(src, OutputConfig::new()).unwrap();

    assert_eq!(
        out,
        concat!(
            r#"<section>"#,
            r#"<p>foo <span style="color: red">bar</span> baz</p>"#,
            r#"</section>"#,
        )
    )
}

#[test]
fn table_infer_inline() {
    let src = r#"
        table {
            <(foo)> <(bar)>
        }
    "#;

    let out = mintyml::convert(src, OutputConfig::new()).unwrap();

    assert_eq!(
        out,
        concat!(
            r#"<table>"#,
            r#"<tr><td>foo</td> <td>bar</td></tr>"#,
            r#"</table>"#,
        )
    )
}

#[test]
fn table_infer_inline_row() {
    let src = r#"table> <(<(foo)> <(bar)>)>"#;

    let out = mintyml::convert(src, OutputConfig::new()).unwrap();

    assert_eq!(
        out,
        concat!(
            r#"<table>"#,
            r#"<tr><td>foo</td> <td>bar</td></tr>"#,
            r#"</table>"#,
        )
    )
}

#[test]
fn table_infer_inline_all() {
    let src = r#"section {
        <(table> <(<(foo)> <(bar)>)> )>
    }"#;

    let out = mintyml::convert(src, OutputConfig::new()).unwrap();

    assert_eq!(
        out,
        concat!(
            r#"<section><p><table>"#,
            r#"<tr><td>foo</td> <td>bar</td></tr>"#,
            r#"</table></p></section>"#,
        )
    )
}

#[test]
fn table_infer_inline_with_id() {
    let src = r#"
        table {
            <(#cell1> foo)> <(#cell2> bar)>
        }
    "#;

    let out = mintyml::convert(src, OutputConfig::new()).unwrap();

    assert_eq!(
        out,
        concat!(
            r#"<table>"#,
            r#"<tr><td id="cell1">foo</td> <td id="cell2">bar</td></tr>"#,
            r#"</table>"#,
        )
    )
}

#[test]
fn table_infer_inline_with_star() {
    let src = r#"
        table {
            <(*> foo)> <(*> bar)>
        }
    "#;

    let out = mintyml::convert(src, OutputConfig::new()).unwrap();

    assert_eq!(
        out,
        concat!(
            r#"<table>"#,
            r#"<tr><td>foo</td> <td>bar</td></tr>"#,
            r#"</table>"#,
        )
    )
}

#[test]
fn details_infer_summary() {
    let src = r#"
        details> {
            > foo

            bar

            baz
        }
    "#;

    let out = mintyml::convert(src, OutputConfig::new()).unwrap();

    assert_eq!(
        out,
        concat!(
            r#"<details>"#,
            r#"<summary>foo</summary> bar baz"#,
            r#"</details>"#,
        )
    )
}

#[test]
fn details_infer_summary_with_paragraphs() {
    let src = r#"
        details {
            > foo

            bar

            baz
        }
    "#;

    let out = mintyml::convert(src, OutputConfig::new()).unwrap();

    assert_eq!(
        out,
        concat!(
            r#"<details>"#,
            r#"<summary>foo</summary> <p>bar</p> <p>baz</p>"#,
            r#"</details>"#,
        )
    )
}

#[test]
fn dl_infer() {
    let src = r#"
        dl {
            > term1

            details1

            > term2

            details2
        }
    "#;

    let out = mintyml::convert(src, OutputConfig::new()).unwrap();

    assert_eq!(
        out,
        concat!(
            r#"<dl>"#,
            r#"<dt>term1</dt>"#,
            r#" <dd>details1</dd>"#,
            r#" <dt>term2</dt>"#,
            r#" <dd>details2</dd>"#,
            r#"</dl>"#,
        )
    )
}

#[test]
fn dl_infer_with_blocks() {
    let src = r#"
        dl {
            > term1
            {
                details1
            }

            > term2

            { details2 }
        }
    "#;

    let out = mintyml::convert(src, OutputConfig::new()).unwrap();

    assert_eq!(
        out,
        concat!(
            r#"<dl>"#,
            r#"<dt>term1</dt>"#,
            r#" <dd>details1</dd>"#,
            r#" <dt>term2</dt>"#,
            r#" <dd>details2</dd>"#,
            r#"</dl>"#,
        )
    )
}

#[test]
fn multiline_escaped() {
    let src = r#"
        """
        foo
        bar\nbaz

        qux
        """
    "#;

    let out = mintyml::convert(src, OutputConfig::new()).unwrap();

    assert_eq!(
        out,
        concat!(
            r#"<p>"#,
            r#"foo&NewLine;bar&NewLine;baz&NewLine;&NewLine;qux"#,
            r#"</p>"#,
        )
    )
}

#[test]
fn multiline_unescaped() {
    let src = r#"
        '''
        foo
        bar\nbaz

        qux
        '''
    "#;

    let out = mintyml::convert(src, OutputConfig::new()).unwrap();

    assert_eq!(
        out,
        concat!(
            r#"<p>"#,
            r#"foo&NewLine;bar\nbaz&NewLine;&NewLine;qux"#,
            r#"</p>"#,
        )
    )
}

#[test]
fn multiline_code() {
    let src = r#"
        ```
        foo
        bar\nbaz

        qux
        ```
    "#;

    let out = mintyml::convert(src, OutputConfig::new()).unwrap();

    assert_eq!(
        out,
        concat!(
            r#"<pre><code>"#,
            r#"foo&NewLine;bar\nbaz&NewLine;&NewLine;qux"#,
            r#"</code></pre>"#,
        )
    )
}
