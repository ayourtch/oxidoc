use std::fmt;

use ansi_term::Style;
use catmark::{self, OutputKind};
use conversion::*;
use document::ModPath;
use generation::ast_ty_wrappers::{FnKind, Attributes};
use term_size;

pub enum Markup {
    Header(String),
    Section(String),
    Block(String),
    Markdown(String),
    Rule(usize),
    LineBreak,
}

use self::Markup::*;

fn get_term_width() -> u16 {
    match term_size::dimensions() {
        Some((w, _)) => w as u16,
        None => 80,
    }
}

impl fmt::Display for Markup {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let string = match *self {
            Header(ref text) => {
                Style::new()
                    .bold()
                    .paint(format!("==== {}", text))
                    .to_string()
            }
            Section(ref text) => {
                Style::new()
                    .bold()
                    .paint(format!("== {}", text))
                    .to_string()
            }
            Block(ref text) => text.clone(),
            Markdown(ref md) => {
                let width = get_term_width();
                catmark::render_ansi(md, width, OutputKind::Color)
            }
            Rule(ref count) => "-".repeat(*count),
            LineBreak => "".to_string(),
        };
        write!(f, "{}", string)
    }
}

/// A formatted piece of documentation made up of individual markup pieces.
pub struct MarkupDoc {
    pub parts: Vec<Markup>,
}

impl MarkupDoc {
    pub fn new(parts: Vec<Markup>) -> Self {
        MarkupDoc { parts: parts }
    }
}

impl fmt::Display for MarkupDoc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for part in self.parts.iter() {
            part.fmt(f)?;
            write!(f, "\n")?;
        }
        Ok(())
    }
}

/// Describes an item that can be inserted into documentation markup.
pub trait Format {
    fn format(&self) -> MarkupDoc;
}

impl Format for Documentation {
    fn format(&self) -> MarkupDoc {
        let header = doc_header(self);
        let info = doc_inner_info(self);
        let signature = doc_signature(self);
        let body = doc_body(self);
        let related_items = doc_related_items(self);

        let mut result = Vec::new();
        result.extend(header.parts);
        result.extend(info.parts);
        result.extend(signature.parts);
        result.extend(body.parts);
        result.extend(related_items.parts);

        MarkupDoc::new(result)
    }
}

impl Format for ModPath {
    fn format(&self) -> MarkupDoc {
        MarkupDoc::new(vec![Header(self.to_string())])
    }
}

impl Format for Attributes {
    fn format(&self) -> MarkupDoc {
        let body = self.doc_strings.join("\n");

        MarkupDoc::new(vec![Markdown(body)])
    }
}

fn doc_header(data: &Documentation) -> MarkupDoc {
    let name = match data.inner_data {
        DocInnerData::FnDoc(..) => "Function",
        DocInnerData::StructDoc(..) => "Struct",
        DocInnerData::ConstDoc(..) => "Constant",
        DocInnerData::EnumDoc(..) => "Enum",
        DocInnerData::TraitDoc(..) => "Trait",
        DocInnerData::TraitItemDoc(..) => "Trait Item",
        DocInnerData::ModuleDoc(ref module) => if module.is_crate { "Crate" } else { "Module" },
    };

    MarkupDoc::new(vec![
        Block(format!("({})", data.crate_info)),
        Header(format!("{} {}", name, data.mod_path)),
    ])
}

fn doc_body(data: &Documentation) -> MarkupDoc {
    data.attrs.format()
}

fn doc_related_items(data: &Documentation) -> MarkupDoc {
    MarkupDoc::new(vec![])
}

fn doc_inner_info(data: &Documentation) -> MarkupDoc {
    let markup = match data.inner_data {
        DocInnerData::FnDoc(ref func) => {
            match func.kind {
                FnKind::MethodFromImpl => Header(format!(
                    "Impl on type {}",
                    data.mod_path.parent().unwrap()
                )),
                _ => LineBreak,
            }
        }
        DocInnerData::TraitItemDoc(..) => {
            Header(format!("From trait {}", data.mod_path.parent().unwrap()))
        }
        DocInnerData::StructDoc(..) |
        DocInnerData::ConstDoc(..) |
        DocInnerData::EnumDoc(..) |
        DocInnerData::TraitDoc(..) |
        DocInnerData::ModuleDoc(..) => LineBreak,
    };
    MarkupDoc::new(vec![markup])
}

fn doc_signature(data: &Documentation) -> MarkupDoc {
    let vis_string = match data.visibility {
        Some(ref v) => v.to_string(),
        None => "".to_string(),
    };

    let header = match data.inner_data {
        DocInnerData::ModuleDoc(ref module) => {
            if module.is_crate {
                return MarkupDoc::new(vec![Rule(10), LineBreak]);
            } else {
                doc_module(data)
            }
        }
        DocInnerData::FnDoc(ref func) => doc_fn(data, func),
        DocInnerData::EnumDoc(..) => doc_enum(data),
        DocInnerData::StructDoc(..) => doc_struct(data),
        DocInnerData::ConstDoc(ref konst) => doc_const(data, konst),
        DocInnerData::TraitDoc(..) => doc_trait(data),
        DocInnerData::TraitItemDoc(ref item) => doc_trait_item(data, item),
    };

    MarkupDoc::new(vec![
        Rule(10),
        LineBreak,
        Block(format!("  {} {}", vis_string, header)),
        LineBreak,
        Rule(10),
        LineBreak,
    ])
}

fn doc_module(data: &Documentation) -> String {
    format!("mod {}", data.mod_path)
}

fn doc_fn(data: &Documentation, func: &Function) -> String {
    format!("fn {} {}", data.name, func.header)
}

fn doc_enum(data: &Documentation) -> String {
    format!("enum {}", data.name)
}

fn doc_struct(data: &Documentation) -> String {
    format!("struct {} {{ /* fields omitted */ }}", data.name)
}

fn doc_const(data: &Documentation, konst: &Constant) -> String {
    format!("const {}: {} = {}", data.name, konst.ty.name, konst.expr)
}

fn doc_trait(data: &Documentation) -> String {
    format!("trait {} {{ /* fields omitted */ }}", data.name)
}

fn doc_trait_item(data: &Documentation, item: &TraitItem) -> String {
    let item_string = match item.node {
        TraitItemKind::Const(ref ty, ref expr) => {
            let expr_string = match *expr {
                Some(ref e) => e.clone(),
                None => "".to_string(),
            };
            format!("const {}: {} = {}", data.name, ty.name, expr_string)
        }
        TraitItemKind::Method(ref sig) => format!("fn {} {}", data.name, sig.header),
        TraitItemKind::Type(ref ty) => {
            let ty_string = match *ty {
                Some(ref t) => t.name.clone(),
                None => "".to_string(),
            };
            format!("type {}", ty_string)
        }
        TraitItemKind::Macro(ref mac) => format!("macro {} {}", data.name, mac),
    };
    item_string
}
