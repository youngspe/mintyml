use mintyml::OutputConfig;
use utils::convert_unwrap;

mod utils;

#[test]
fn infer_style_raw() {
    let src = r#"
head {
  style>'''
  .foo {
    color: red;
    background: blue;
  }
  '''
}
"#;

    let actual = convert_unwrap(src, OutputConfig::new().indent("  "));

    assert_eq!(
        actual,
        r#"<head>
  <style>.foo {
  color: red;
  background: blue;
}</style>
</head>
"#
    );
}

#[test]
fn infer_script_raw() {
    let src = r#"
head {
  script>'''
  function foo(a, b) {
    return a < b
  }
  '''
}
"#;

    let actual = convert_unwrap(src, OutputConfig::new().indent("  "));

    assert_eq!(
        actual,
        r#"<head>
  <script>function foo(a, b) {
  return a < b
}</script>
</head>
"#
    );
}
