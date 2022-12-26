use std::{error::Error, path::Path};

use lazy_static::lazy_static;
use quote::{__private::TokenStream, format_ident, quote};
use regex::{Captures, Regex};
use thiserror::Error;

pub fn convert_html_to_rs(html: &str, filename: &str) -> Result<String, Box<dyn Error>> {
    let struck_name = match Path::new(filename).file_stem() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => {
            return Err(Box::new(ConversionError::FileNameInvalid));
        }
    };

    let header_data = extract_header(&html, &struck_name)?;
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
        return Err(HeaderExtractionError::HeaderNotFound);
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

fn generate_implementation(
    html: &str,
    header: &HeaderData,
) -> TokenStream {
    lazy_static! {
        static ref MATCH_VARIABLE: Regex =
            Regex::new(r"(?P<escape>@@)|@(?P<field>[_a-z0-9]+?)@").unwrap();
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

    let code = quote! {
        impl #struct_name {
            pub fn render(self) -> String {
                #(#assignments)*
                format!(#output)
            }
        }
    };

    code
}

#[derive(Debug, PartialEq)]
struct HeaderData {
    struct_name: String,
    fields: Vec<(String, String)>,
}

#[derive(Error, Debug)]
enum ConversionError {
    #[error("The file name is invalid")]
    FileNameInvalid,
}

#[derive(Error, Debug)]
enum HeaderExtractionError {
    #[error("The document does not have a header")]
    HeaderNotFound,
    #[error("The header is malformed: Missing field on line {line}")]
    MalformedFieldName { line: u32 },
    #[error("The header is malformed: Missing type on line {line}")]
    MalformedFieldType { line: u32 },
}

#[test]
fn extract_header__single_file() {
    let html_in = include_str!("../tests/input/single_file/index.html");
    let expected_result = HeaderData {
        struct_name: String::from("Basic"),
        fields: vec![
            ("name".into(), "String".into()),
            ("page_name".into(), "String".into()),
        ],
    };

    let result = extract_header(&html_in, "Basic").unwrap();

    assert_eq!(expected_result, result);
}
