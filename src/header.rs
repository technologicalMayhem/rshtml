use codegen::Struct;
use lazy_static::lazy_static;
use regex::Regex;
use thiserror::Error;

pub fn extract_header(html: &str, struct_name: &str) -> Result<HeaderData, HeaderExtractionError> {
    lazy_static! {
        static ref MATCH_HEADER: Regex = Regex::new(r"(\w+: \w+)").unwrap();
        static ref MATCH_FIELD: Regex = Regex::new(r"(\w+): (\w+)").unwrap();
    }

    let header: Vec<&str> = MATCH_HEADER.find_iter(html).map(|m| m.as_str()).collect();

    if header.is_empty() {
        return Ok(HeaderData {
            struct_name: struct_name.into(),
            fields: Vec::new(),
        });
    }

    let mut fields: Vec<(String, String)> = Vec::new();

    for (line_count, line) in header.into_iter().enumerate() {
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

pub fn generate_struct(data: &HeaderData) -> Struct {
    let mut s = Struct::new(&data.struct_name);
    s.vis("pub");
    for field in &data.fields {
        s.new_field(&field.0, &field.1);
    }

    s
}

#[derive(Debug, PartialEq)]
pub struct HeaderData {
    pub struct_name: String,
    pub fields: Vec<(String, String)>,
}

#[derive(Error, Debug)]
pub enum HeaderExtractionError {
    #[error("The header is malformed: Missing field on line {line}")]
    MalformedFieldName { line: usize },
    #[error("The header is malformed: Missing type on line {line}")]
    MalformedFieldType { line: usize },
}

#[cfg(test)]
mod tests {
    use crate::header::{extract_header, HeaderData};

    #[test]
    fn extract_header_single_file() {
        let html_in = include_str!("../tests/input/single_file/index.html");

        let result = extract_header(html_in, "Basic").unwrap();

        let expected_result = HeaderData {
            struct_name: String::from("Basic"),
            fields: vec![
                ("name".into(), "String".into()),
                ("page_name".into(), "String".into()),
            ],
        };

        assert_eq!(expected_result, result);
    }
}
