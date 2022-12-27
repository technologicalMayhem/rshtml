use std::{
    error::Error,
    fs::{read_dir, read_to_string},
    path::{Path, PathBuf},
};

use codegen::{Impl, Scope, Struct, Type};
use lazy_static::lazy_static;
use minify::html::minify;
use regex::{Captures, Regex};
use thiserror::Error;

pub fn convert_directory(path: &str) -> Result<String, Box<dyn Error>> {
    let path = Path::new(path);
    if path.is_dir() == false {
        return Err(Box::new(DirectoryConversionError::IsNotADirectory));
    }

    let mut scope = Scope::new();
    process_directory(&path, &mut scope)?;

    Ok(scope.to_string())
}

fn process_directory(path: &Path, scope: &mut Scope) -> Result<(), Box<dyn Error>> {
    let dir = read_dir(path)?;

    for entry in dir {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();
        let file_stem = path.file_stem().unwrap().to_string_lossy();
        if file_type.is_file() {
            let contents = read_to_string(&path)?;
            let struct_name = uppercase_first_letter(&file_stem);
            convert_html_to_rs(&contents, &struct_name, scope)?;
        }
        if file_type.is_dir() {
            let new_scope = scope.new_module(&file_stem).vis("pub").scope();
            process_directory(&path, new_scope)?;
        }
    }

    Ok(())
}

pub fn convert_file(path: &str) -> Result<String, Box<dyn Error>> {
    let path = Path::new(path);
    let html = read_to_string(&path)?;

    let file_stem = path.file_stem().unwrap().to_string_lossy();
    let struct_name = uppercase_first_letter(&file_stem);
    let mut scope = Scope::new();
    convert_html_to_rs(&html, &struct_name, &mut scope)?;

    Ok(scope.to_string())
}

fn convert_html_to_rs(
    html: &str,
    struct_name: &str,
    scope: &mut Scope,
) -> Result<(), Box<dyn Error>> {
    let header_data = extract_header(&html, &struct_name)?;
    let struct_ = generate_struct(&header_data);
    let impl_ = generate_implementation(&html, struct_.ty().clone(), &header_data);

    scope.push_struct(struct_);
    scope.push_impl(impl_);

    Ok(())
}

fn extract_header(html: &str, struct_name: &str) -> Result<HeaderData, HeaderExtractionError> {
    lazy_static! {
        static ref MATCH_HEADER: Regex = Regex::new(r"(\w+: \w+)").unwrap();
        static ref MATCH_FIELD: Regex = Regex::new(r"(\w+): (\w+)").unwrap();
    }

    let header: Vec<&str> = MATCH_HEADER.find_iter(&html).map(|m| m.as_str()).collect();

    if header.is_empty() {
        return Ok(HeaderData {
            struct_name: struct_name.into(),
            fields: Vec::new(),
        });
    }

    let mut fields: Vec<(String, String)> = Vec::new();
    let mut line_count = 0;

    for line in header {
        line_count += 1;

        let (name, type_) = match MATCH_FIELD.captures(line) {
            Some(f) => (f.get(1), f.get(2)),
            None => {
                break;
            }
        };

        let name: String = match name {
            Some(n) => n.as_str().into(),
            None => return Err(HeaderExtractionError::MalformedFieldName { line: line_count }),
        };

        let type_: String = match type_ {
            Some(t) => t.as_str().into(),
            None => {
                return Err(HeaderExtractionError::MalformedFieldType { line: line_count });
            }
        };

        fields.push((name, type_));
    }

    Ok(HeaderData {
        struct_name: struct_name.into(),
        fields,
    })
}

fn generate_struct(data: &HeaderData) -> Struct {
    let mut s = Struct::new(&data.struct_name);
    s.vis("pub");
    for field in &data.fields {
        s.new_field(&field.0, &field.1);
    }

    s
}

fn generate_implementation(html: &str, for_: Type, header: &HeaderData) -> Impl {
    lazy_static! {
        static ref MATCH_VARIABLE: Regex =
            Regex::new(r"(?P<escape>@@)|@(?P<field>[_a-zA-Z0-9]+?)@").unwrap();
        static ref MATCH_HEADER: Regex = Regex::new(r"\{[\s\S]*\}\n").unwrap();
    }

    let mut impl_ = Impl::new(for_);

    let without_header = MATCH_HEADER.replace(html, "");
    let processed_hmtl = minify(&without_header);
    let output = MATCH_VARIABLE.replace_all(&processed_hmtl, |caps: &Captures| {
        if let Some(_) = caps.name("escape") {
            return "@".into();
        }
        if let Some(field) = caps.name("field") {
            let field = field.as_str();
            return format!("{{{field}}}");
        }
        panic!("An unknown match occured during document processing. This should never happen.")
    });

    let render = impl_.new_fn("render");
    render.vis("pub");
    render.arg_self();
    render.ret(Type::new("String"));
    for field in &header.fields {
        render.line(format!("let {} = self.{};", field.0, field.0));
    }
    render.line(format!("format!(\"{output}\")"));

    let new = impl_.new_fn("new");
    new.vis("pub");
    for field in &header.fields {
        new.arg(&field.0, &field.1);
    }
    new.ret(Type::new("Self"));

    let new_assignments = header
        .fields
        .iter()
        .map(|field| field.0.clone())
        .reduce(|a, b| a + ", " + &b)
        .unwrap_or_default();
    new.line(format!("Self {{ {new_assignments} }}"));

    impl_
}

/// Removes the `base` part for the given `path`.
///
/// If the paths are identical, `None` is returned.
/// If `base` is not a part of `path`, `None` is returned.
fn remove_base(path: &Path, base: &Path) -> Option<PathBuf> {
    let mut path_components = path.components();
    let mut base_components = base.components();

    loop {
        //We clone path_components as not not actually advance it
        let p = path_components.clone().next();
        let b = base_components.next();

        if p == None {
            //If path end before it's base, we return None
            return None;
        }
        if b == None {
            //The base ended before the path, so we return the rest
            //If we had it advanced it for real we would be missing one piece of the path
            return Some(path_components.as_path().to_path_buf());
        }
        if p? == b? {
            //Both pieces are the same
            //Advance it for real this time
            path_components.next();
            continue;
        }

        //There is mismatch between the two, we return None
        return None;
    }
}

//Copied from https://stackoverflow.com/a/38406885/9627790
/// Capitalizes the first letter.
fn uppercase_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Check if the given `path` starts with `base`
fn does_path_start_with(path: &Path, base: &Path) -> bool {
    let mut path_c = path.components();
    let mut base_c = base.components();

    loop {
        let p = path_c.next();
        let b = base_c.next();

        if b == None {
            return true;
        }
        if p == b {
            continue;
        }
        return false;
    }
}

#[derive(Debug, PartialEq)]
struct HeaderData {
    struct_name: String,
    fields: Vec<(String, String)>,
}

#[derive(Error, Debug)]
pub enum DirectoryConversionError {
    #[error("The path does not point to a valid directory")]
    IsNotADirectory,
}

#[derive(Error, Debug)]
pub enum FileConversionError {
    #[error("The file name is invalid")]
    FileNameInvalid,
}

#[derive(Error, Debug)]
pub enum HeaderExtractionError {
    #[error("The document does not have a header")]
    HeaderNotFound,
    #[error("The header is malformed: Missing field on line {line}")]
    MalformedFieldName { line: u32 },
    #[error("The header is malformed: Missing type on line {line}")]
    MalformedFieldType { line: u32 },
}

#[test]
fn extract_header_single_file() {
    let html_in = include_str!("../tests/input/single_file/index.html");

    let result = extract_header(&html_in, "Basic").unwrap();

    let expected_result = HeaderData {
        struct_name: String::from("Basic"),
        fields: vec![
            ("name".into(), "String".into()),
            ("page_name".into(), "String".into()),
        ],
    };

    assert_eq!(expected_result, result);
}

#[cfg(test)]
mod tests {
    use super::*;

    mod remove_base {
        use super::*;

        #[test]
        fn remove_base_diferent_base() {
            let path = Path::new("foo/bar/foobar/baz");
            let base = Path::new("baz/foobar");

            let result = remove_base(path, base);

            assert_eq!(result, None)
        }

        #[test]
        fn remove_base_identical_path() {
            let path = Path::new("foo/bar/foobar/baz");
            let base = Path::new("foo/bar/foobar/baz");

            let result = remove_base(path, base);

            assert_eq!(result, None)
        }

        #[test]
        fn remove_base_same_base() {
            let path = Path::new("foo/bar/foobar/baz");
            let base = Path::new("foo/bar");

            let result = remove_base(path, base).unwrap();

            assert_eq!(result, Path::new("foobar/baz"))
        }
    }
    mod does_path_start_with {
        use super::*;

        #[test]
        fn does_start_with() {
            let a = Path::new("/foo/bar/baz");
            let b = Path::new("/foo/bar");

            assert!(does_path_start_with(a, b));
        }

        #[test]
        fn does_not_start_with() {
            let a = Path::new("/foo/bar/baz");
            let b = Path::new("/flim/flam");

            assert!(does_path_start_with(a, b) == false);
        }
    }
}
