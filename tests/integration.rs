use rshtml::{convert_html_to_rs, convert_directory};

#[test]
fn single_file() {
    let html_in = include_str!("input/single_file/index.html");
    let html_out = include_str!("output/single_file/pages.rs");

    let result = convert_html_to_rs(html_in.into(), "Basic.html").unwrap();

    assert_eq!(result, html_out);
}

#[test]
fn multi_file() {
    let path_in = "./tests/input/multi_file";
    let code_out = include_str!("output/multi_file/pages.rs");

    let result = convert_directory(path_in).unwrap();

    assert_eq!(result, code_out)
}