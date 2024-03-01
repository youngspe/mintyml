
use utils::convert_unwrap;

mod utils;

#[test]
fn x_letters() {
    let out = convert_unwrap(r#"
        \x41\x42
    "#, None);

    assert_eq!(out, r"<p>AB</p>");
}

#[test]
fn x_letters_multiline() {
    let out = convert_unwrap(r#"
        """
        \x41\x42
        """
    "#, None);

    assert_eq!(out, r"<p>AB</p>");
}

#[test]
fn u_letters() {
    let out = convert_unwrap(r#"
        \u{41}\u{000042}
    "#, None);

    assert_eq!(out, r"<p>AB</p>");
}

#[test]
fn u_letters_multiline() {
    let out = convert_unwrap(r#"
        """
        \u{41}\u{000042}
        """
    "#, None);

    assert_eq!(out, r"<p>AB</p>");
}
