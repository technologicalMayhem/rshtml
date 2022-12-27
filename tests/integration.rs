use insta::assert_snapshot;
use rshtml::{convert_file, convert_directory};

#[test]
fn single_file() {
    let path_in = "./tests/input/single_file/index.html";

    let result = convert_file(path_in).unwrap();

    assert_snapshot!(result);
}

#[test]
fn multi_file() {
    let path_in = "./tests/input/multi_file";

    let result = convert_directory(path_in).unwrap();

    assert_snapshot!(result)
}