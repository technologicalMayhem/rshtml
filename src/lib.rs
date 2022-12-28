use std::{
    error::Error,
    fs::{read_dir, read_to_string},
    path::Path,
};

use codegen::Scope;

use thiserror::Error;

use crate::header::*;
use crate::implementation::*;
use crate::util::*;

mod header;
mod implementation;
mod util;

pub fn convert_directory(path: &str) -> Result<String, Box<dyn Error>> {
    let path = Path::new(path);
    if !path.is_dir() {
        return Err(Box::new(DirectoryConversionError::IsNotADirectory));
    }

    let mut scope = Scope::new();
    process_directory(path, &mut scope)?;

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
    let html = read_to_string(path)?;

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
    let header_data = extract_header(html, struct_name)?;
    let struct_ = generate_struct(&header_data);
    let impl_ = generate_implementation(html, struct_.ty().clone(), &header_data);

    scope.push_struct(struct_);
    scope.push_impl(impl_);

    Ok(())
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
