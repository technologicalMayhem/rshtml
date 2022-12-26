use rshtml::{convert_html_to_rs};

#[test]
fn basic_template() {
    let html_in = include_str!("templates/basic_in.html");
    let html_out = include_str!("examples/basic.rs");

    let result = convert_html_to_rs(html_in.into(), "Basic.html").unwrap();

    assert_eq!(result, html_out);
}