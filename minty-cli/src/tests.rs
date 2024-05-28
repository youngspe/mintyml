use crate::{
    io_helper::test_helper::{any, contains},
    test_main,
};

const BASIC_SRC: &str = r#"
div {
    <#Hello#>, </world/>!
}
"#;

const BASIC_SRC2: &str = r#"
ul {
    <#Hello#>,

    </world/>!
}
"#;

const BASIC_OUT: &str = "<div><p><strong>Hello</strong>, <em>world</em>!</p></div>";
const BASIC_OUT2: &str = "<ul><li><strong>Hello</strong>,</li> <li><em>world</em>!</li></ul>";

macro_rules! or_default {
    ([$($default:tt)*]) => {
        $($default)*
    };
    ([$($value:tt)+] $default:tt) => {
        $($value)*
    };
}

macro_rules! test_main_inner {
    (
        $([args = $args:expr])?
        $([stdin = $input:expr])?
        $([files = $root:expr])?
        $([cwd = $cwd:expr])?
        $([assert_empty_stderr = $assert_empty_stderr:expr])?
    ) => {
        match test_main(
            $($args)?,
            or_default!($([$input])? [None]),
            or_default!($([crate::io_helper::test_helper::TestDir::from_file_list($root)])? [None]),
            or_default!($([$cwd])? [None]),
        ) { actual => {
            if or_default!($([$assert_empty_stderr])? [true]) {
                assert_eq!(&actual.stderr, "")
            }
            actual
        } }
    };
}
macro_rules! test_main {
    (args = $args:expr $(,$( $key:ident = $value:expr),+ $(,)?)?) => {
        test_main_inner!([args = $args] $($([$key = $value])*)?)
    };
    ($args:expr $(,$( $key:ident = $value:expr),+ $(,)?)?) => {
        test_main_inner!(
            [args = &format!(r#"mintyml-cli convert --complete-page=false {}"#, $args)]
            $($([$key = $value])*)?
        )
    };
}

#[test]
fn basic_test() {
    let actual = test_main!("--stdin --stdout", stdin = BASIC_SRC);

    assert!(actual.outcome.unwrap());
    assert_eq!(actual.stdout, BASIC_OUT);
}

fn complete_page(explicit: bool) {
    let actual = test_main!(
        args = "mintyml-cli convert --stdin --stdout ".to_string()
            + if explicit { "--complete-page=true" } else { "" },
        stdin = r#"
title> Foo
div {
    <#Hello#>, </world/>!
}
"#,
    );

    assert!(actual.outcome.unwrap());
    assert_eq!(
        actual.stdout,
        concat!(
            "<!DOCTYPE html>\n",
            r#"<html><head><title>Foo</title></head> "#,
            r#"<body><div><p><strong>Hello</strong>, <em>world</em>!</p></div></body></html>"#
        )
    );
}

#[test]
fn complete_page_implicit() {
    complete_page(false)
}

#[test]
fn complete_page_explicit() {
    complete_page(true)
}

#[test]
fn convert_explicit_file_implicit_dest() {
    let actual = test_main!(
        "c/d/foo.mty",
        files = [("/a/b/c/d/foo.mty", Some(BASIC_SRC))],
        cwd = "/a/b"
    );

    assert!(actual.outcome.unwrap());
    assert_eq!(actual.stdout, "");
    actual
        .root
        .compare_file_list([
            ("/a/b/c/d/foo.mty", contains(BASIC_SRC)),
            ("/a/b/c/d/foo.html", contains(BASIC_OUT)),
        ])
        .unwrap()
}

#[test]
fn convert_explicit_file_explicit_dest() {
    let actual = test_main!(
        "c/d/foo.mty -o c/foo2.html",
        files = [("/a/b/c/d/foo.mty", Some(BASIC_SRC))],
        cwd = "/a/b"
    );

    assert!(actual.outcome.unwrap());
    assert_eq!(actual.stdout, "");
    actual
        .root
        .compare_file_list([
            ("/a/b/c/foo2.html", contains(BASIC_OUT)),
            ("/a/b/c/d/foo.mty", contains(BASIC_SRC)),
        ])
        .unwrap()
}

#[test]
fn convert_explicit_file_explicit_absolute_dest() {
    let actual = test_main!(
        "c/d/foo.mty -o /a/foo2.html",
        files = [("/a/b/c/d/foo.mty", Some(BASIC_SRC))],
        cwd = "/a/b"
    );

    assert!(actual.outcome.unwrap());
    assert_eq!(actual.stdout, "");
    actual
        .root
        .compare_file_list([
            ("/a/foo2.html", contains(BASIC_OUT)),
            ("/a/b/c/d/foo.mty", contains(BASIC_SRC)),
        ])
        .unwrap()
}

#[test]
fn convert_explicit_files_implicit_dest() {
    let actual = test_main!(
        "c/d/foo.minty /a/bar.mty",
        files = [
            ("/a/bar.mty", Some(BASIC_SRC2)),
            ("/a/b/c/d/foo.minty", Some(BASIC_SRC)),
        ],
        cwd = "/a/b"
    );

    assert!(actual.outcome.unwrap());
    assert_eq!(actual.stdout, "");
    actual
        .root
        .compare_file_list([
            ("/a/bar.mty", contains(BASIC_SRC2)),
            ("/a/bar.html", contains(BASIC_OUT2)),
            ("/a/b/c/d/foo.minty", contains(BASIC_SRC)),
            ("/a/b/c/d/foo.html", contains(BASIC_OUT)),
        ])
        .unwrap()
}

#[test]
fn convert_explicit_files_explicit_dest() {
    let actual = test_main!(
        "c/d/foo.minty /a/bar.mty --out c/new_dir1/new_dir2/",
        files = [
            ("/a/bar.mty", Some(BASIC_SRC2)),
            ("/a/b/c/d/foo.minty", Some(BASIC_SRC)),
        ],
        cwd = "/a/b"
    );

    assert!(actual.outcome.unwrap());
    assert_eq!(actual.stdout, "");
    actual
        .root
        .compare_file_list([
            ("/a/bar.mty", contains(BASIC_SRC2)),
            ("/a/b/c/d/foo.minty", contains(BASIC_SRC)),
            ("/a/b/c/new_dir1/new_dir2/bar.html", contains(BASIC_OUT2)),
            ("/a/b/c/new_dir1/new_dir2/foo.html", contains(BASIC_OUT)),
        ])
        .unwrap()
}

#[test]
fn convert_explicit_files_explicit_absolute_dest() {
    let actual = test_main!(
        "c/d/foo.minty /a/bar.mty --out /a/new_dir1/new_dir2/",
        files = [
            ("/a/bar.mty", Some(BASIC_SRC2)),
            ("/a/b/c/d/foo.minty", Some(BASIC_SRC)),
        ],
        cwd = "/a/b"
    );

    assert!(actual.outcome.unwrap());
    assert_eq!(actual.stdout, "");
    actual
        .root
        .compare_file_list([
            ("/a/new_dir1/new_dir2/bar.html", contains(BASIC_OUT2)),
            ("/a/new_dir1/new_dir2/foo.html", contains(BASIC_OUT)),
            ("/a/bar.mty", contains(BASIC_SRC2)),
            ("/a/b/c/d/foo.minty", contains(BASIC_SRC)),
        ])
        .unwrap()
}

#[test]
fn convert_dir_implicit_dest_no_recurse() {
    let actual = test_main!(
        "--dir c",
        files = [
            ("/a/b/c/d2/e2/bar.mty", Some(BASIC_SRC2)),
            ("/a/b/c/d/foo.minty", Some(BASIC_SRC)),
            ("/a/b/c/baz.mty", Some(BASIC_SRC)),
            ("/a/b/c/qux.txt", Some("no mintyml extension")),
        ],
        cwd = "/a/b"
    );

    assert!(actual.outcome.unwrap());
    assert_eq!(actual.stdout, "");
    actual
        .root
        .compare_file_list([
            ("/a/b/c/d2/e2/bar.mty", contains(BASIC_SRC2)),
            ("/a/b/c/d/foo.minty", contains(BASIC_SRC)),
            ("/a/b/c/baz.mty", contains(BASIC_SRC)),
            ("/a/b/c/baz.html", contains(BASIC_OUT)),
            ("/a/b/c/qux.txt", contains("no mintyml extension")),
        ])
        .unwrap()
}

#[test]
fn convert_dir_implicit_dest_recurse() {
    let actual = test_main!(
        "--dir c --recurse",
        files = [
            ("/a/b/c/d2/e2/bar.mty", Some(BASIC_SRC2)),
            ("/a/b/c/d/foo.minty", Some(BASIC_SRC)),
            ("/a/b/c/baz.mty", Some(BASIC_SRC)),
            ("/a/b/c/qux.txt", Some("no mintyml extension")),
        ],
        cwd = "/a/b"
    );

    assert!(actual.outcome.unwrap());
    assert_eq!(actual.stdout, "");
    actual
        .root
        .compare_file_list([
            ("/a/b/c/d2/e2/bar.mty", contains(BASIC_SRC2)),
            ("/a/b/c/d2/e2/bar.html", contains(BASIC_OUT2)),
            ("/a/b/c/d/foo.minty", contains(BASIC_SRC)),
            ("/a/b/c/d/foo.html", contains(BASIC_OUT)),
            ("/a/b/c/baz.mty", contains(BASIC_SRC)),
            ("/a/b/c/baz.html", contains(BASIC_OUT)),
            ("/a/b/c/qux.txt", contains("no mintyml extension")),
        ])
        .unwrap()
}
