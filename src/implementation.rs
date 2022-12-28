use codegen::{Impl, Type};
use lazy_static::lazy_static;
use minify::html::minify;
use regex::{Captures, Regex};

use crate::header::HeaderData;

pub fn generate_implementation(html: &str, for_: Type, header: &HeaderData) -> Impl {
    lazy_static! {
        static ref MATCH_VARIABLE: Regex =
            Regex::new(r"(?P<escape>@@)|@(?P<field>[_a-zA-Z0-9]+?)@").unwrap();
        static ref MATCH_HEADER: Regex = Regex::new(r"\{[\s\S]*\}\n").unwrap();
    }

    let mut impl_ = Impl::new(for_);

    let without_header = MATCH_HEADER.replace(html, "");
    let processed_hmtl = minify(&without_header);
    let output = MATCH_VARIABLE.replace_all(&processed_hmtl, |caps: &Captures| {
        if caps.name("escape").is_some() {
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
