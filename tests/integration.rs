use rshtml::{convert_html_to_rs};

#[test]
fn single_file() {
    let html_in = include_str!("input/single_file/index.html");
    let html_out = include_str!("output/single_file/pages.rs");

    let result = convert_html_to_rs(html_in.into(), "Basic.html").unwrap();

    assert_eq!(result, html_out);
}