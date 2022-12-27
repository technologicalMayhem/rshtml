use rshtml::{convert_file, convert_directory};

#[test]
fn single_file() {
    let html_out = include_str!("output/single_file/pages.rs");

    let result = convert_file("./tests/input/single_file/index.html").unwrap();

    assert_eq!(result, html_out);
}

#[test]
fn multi_file() {
    let path_in = "./tests/input/multi_file";
    let code_out = include_str!("output/multi_file/pages.rs");

    let result = convert_directory(path_in).unwrap();

    assert_eq!(result, code_out)
}