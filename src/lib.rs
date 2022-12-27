use std::{
    error::Error,
    fs::read_to_string,
    path::{Path, PathBuf},
};

use lazy_static::lazy_static;
use quote::{__private::TokenStream, format_ident, quote};
use regex::{Captures, Regex};
use thiserror::Error;
use walkdir::WalkDir;

pub fn convert_directory(path: &str) -> Result<String, Box<dyn Error>> {
    let path = Path::new(path);
    if path.is_dir() == false {
        return Err(Box::new(DirectoryConversionError::IsNotADirectory));
    }

    let mut walker = WalkDir::new(path).into_iter();
    //Walk once to get rid of the root directory
    walker.next();
    let mut full_code = String::new();
    let mut cur_path = PathBuf::new();

    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                return Err(Box::new(e));
            }
        };

        if entry.file_type().is_dir() {
            full_code += &format!("pub mod {} {{", entry.file_name().to_string_lossy());
            cur_path = entry.path().to_path_buf();
            continue;
        }
        if entry.file_type().is_file() {
            let path = entry.path().parent().unwrap();
            while does_path_start_with(path, cur_path.as_path()) == false {
                cur_path = cur_path.as_path().parent().unwrap().to_path_buf();
                full_code += "}";
            }
        }

        let html = match read_to_string(entry.path()) {
            Ok(html) => html,
            Err(e) => {
                return Err(Box::new(e));
            }
        };

        let path_buf = remove_base(entry.path(), &path).unwrap();
        let filename = path_buf.as_path();
        let code = match convert_html_to_rs(&html, &filename.to_string_lossy()) {
            Ok(out) => out,
            Err(e) => {
                return Err(e);
            }
        };

        full_code += &code;
    }

    let syn = syn::parse_file(&full_code)?;
    let out = prettyplease::unparse(&syn);

    Ok(out)
}

pub fn convert_html_to_rs(html: &str, filename: &str) -> Result<String, Box<dyn Error>> {
    let struct_name = match Path::new(filename).file_stem() {
        Some(name) => uppercase_first_letter(&name.to_string_lossy()),
        None => {
            return Err(Box::new(FileConversionError::FileNameInvalid));
        }
    };

    let header_data = extract_header(&html, &struct_name)?;
    let struct_ = generate_struct(&header_data);
    let impl_ = generate_implementation(&html, &header_data);

    let tokens = quote!(
        #struct_
        #impl_
    )
    .to_string();

    let syn = syn::parse_file(&tokens)?;

    let out = prettyplease::unparse(&syn);
    Ok(out)
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

fn generate_struct(data: &HeaderData) -> TokenStream {
    let ident = format_ident!("{}", data.struct_name);
    let fields: Vec<TokenStream> = data
        .fields
        .iter()
        .map(|f| format!("{}: {}", f.0, f.1).parse().unwrap())
        .collect();

    quote! {
        pub struct #ident {
            #(#fields),*
        }
    }
}

fn generate_implementation(html: &str, header: &HeaderData) -> TokenStream {
    lazy_static! {
        static ref MATCH_VARIABLE: Regex =
            Regex::new(r"(?P<escape>@@)|@(?P<field>[_a-zA-Z0-9]+?)@").unwrap();
        static ref MATCH_HEADER: Regex = Regex::new(r"\{[\s\S]*\}\n").unwrap();
    }

    let without_header = MATCH_HEADER.replace(html, "");
    let output = MATCH_VARIABLE.replace_all(&without_header, |caps: &Captures| {
        if let Some(_) = caps.name("escape") {
            return "@".into();
        }
        if let Some(field) = caps.name("field") {
            let field = field.as_str();
            return format!("{{{field}}}");
        }
        panic!("An unknown match occured during document processing. This should never happen.")
    });

    let struct_name: TokenStream = header.struct_name.parse().unwrap();
    let assignments: Vec<TokenStream> = header
        .fields
        .iter()
        .map(|field| {
            format!("let {} = self.{};", field.0, field.0)
                .parse()
                .unwrap()
        })
        .collect();

    let mut new_arguments = String::new();
    for field in &header.fields {
        if new_arguments.is_empty() == false {
            new_arguments += ", "
        }

        new_arguments += &format!("{}: {}", field.0, field.1);
    }

    let new_arguments: TokenStream = new_arguments.parse().unwrap();
    let new_assignments: Vec<TokenStream> = header
        .fields
        .iter()
        .map(|field| format!("{}, ", field.0).parse().unwrap())
        .collect();

    let code = quote! {
        impl #struct_name {
            pub fn render(self) -> String {
                #(#assignments)*
                format!(#output)
            }

            pub fn new(#new_arguments) -> Self {
                Self {
                    #(#new_assignments)*
                }
            }
        }
    };

    code
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
