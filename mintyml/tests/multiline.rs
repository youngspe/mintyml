use mintyml::OutputConfig;

mod utils;

#[test]
fn multiline_code_indentation() {
    let src = r#"

    section {
        ```
        Hello
            there,
        world!
        ```
    }

    "#;

    let out = utils::convert_unwrap(src, None);

    assert_eq!(
        out,
        r#"<section><pre><code>Hello&NewLine;    there,&NewLine;world!</code></pre></section>"#
    );
}

#[test]
fn multiline_code_indentation_pretty() {
    let src = r#"
    section {
        ```
        Hello
            there,
        world!
        ```
    }

    "#;

    let out = utils::convert_unwrap(src, OutputConfig::new().indent("  "));

    assert_eq!(
        out,
        r#"<section>
  <pre><code>Hello&NewLine;    there,&NewLine;world!</code></pre>
</section>
"#
    );
}

#[test]
fn multiline_single_indentation() {
    let src = r#"

    section {
        '''
        Hello
            there,
        world!
        '''
    }

    "#;

    let out = utils::convert_unwrap(src, None);

    assert_eq!(
        out,
        r#"<section><p>Hello&NewLine;    there,&NewLine;world!</p></section>"#
    );
}

#[test]
fn multiline_single_indentation_pretty() {
    let src = r#"

    section {
        '''
        Hello
            there,
        world!
        '''
    }

    "#;

    let out = utils::convert_unwrap(src, OutputConfig::new().indent("  "));

    assert_eq!(
        out,
        r#"<section>
  <p>Hello&NewLine;    there,&NewLine;world!</p>
</section>
"#
    );
}

#[test]
fn multiline_double_indentation() {
    let src = r#"

    section {
        """
        Hello
            there,
        world!
        """
    }

    "#;

    let out = utils::convert_unwrap(src, OutputConfig::new());

    assert_eq!(
        out,
        r#"<section><p>Hello&NewLine;    there,&NewLine;world!</p></section>"#
    );
}

#[test]
fn multiline_double_indentation_pretty() {
    let src = r#"

    section {
        """
        Hello
            there,
        world!
        """
    }

    "#;

    let out = utils::convert_unwrap(src, OutputConfig::new().indent("  "));

    assert_eq!(
        out,
        r#"<section>
  <p>Hello&NewLine;    there,&NewLine;world!</p>
</section>
"#
    );
}
