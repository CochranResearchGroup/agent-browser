use proc_macro2::Span;
use quote::ToTokens;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use syn::parse_quote;
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;
use syn::{Field, ImplItem, Item, PathSegment, Visibility};

const MODULES: &[&str] = &[
    "deadline",
    "runtime",
    "coordinator",
    "planner",
    "compensation",
    "operator_route",
    "proof",
    "preflight",
    "route_lifecycle",
    "target",
    "route_pool",
];

fn public_crate() -> Visibility {
    parse_quote!(pub(crate))
}

fn expose_field(field: &mut Field) {
    if matches!(field.vis, Visibility::Inherited) {
        field.vis = public_crate();
    }
}

fn expose_item(item: &mut Item) {
    match item {
        Item::Const(item) => item.vis = public_crate(),
        Item::Enum(item) => item.vis = public_crate(),
        Item::Fn(item) => item.vis = public_crate(),
        Item::Static(item) => item.vis = public_crate(),
        Item::Struct(item) => {
            item.vis = public_crate();
            for field in &mut item.fields {
                expose_field(field);
            }
        }
        Item::Trait(item) => item.vis = public_crate(),
        Item::Type(item) => item.vis = public_crate(),
        Item::Union(item) => {
            item.vis = public_crate();
            for field in &mut item.fields.named {
                expose_field(field);
            }
        }
        Item::Impl(item) if item.trait_.is_none() => {
            for implementation_item in &mut item.items {
                if let ImplItem::Fn(method) = implementation_item {
                    if matches!(method.vis, Visibility::Inherited) {
                        method.vis = public_crate();
                    }
                }
            }
        }
        _ => {}
    }
}

fn item_name(item: &Item) -> Option<String> {
    match item {
        Item::Const(item) => Some(item.ident.to_string()),
        Item::Enum(item) => Some(item.ident.to_string()),
        Item::Fn(item) => Some(item.sig.ident.to_string()),
        Item::Static(item) => Some(item.ident.to_string()),
        Item::Struct(item) => Some(item.ident.to_string()),
        Item::Trait(item) => Some(item.ident.to_string()),
        Item::Type(item) => Some(item.ident.to_string()),
        Item::Union(item) => Some(item.ident.to_string()),
        _ => None,
    }
}

fn module_for(item: &Item) -> Option<&'static str> {
    if matches!(item, Item::Use(_)) {
        return None;
    }
    let line = item.span().start().line;
    Some(match line {
        18..=55 | 645..=831 | 1056..=1676 => "coordinator",
        56..=312 => "deadline",
        313..=644 => "runtime",
        832..=1055 | 1677..=1868 => "planner",
        1869..=1978 => "compensation",
        1979..=2250 => "operator_route",
        2251..=2330 => "proof",
        2331..=2810 => "preflight",
        2811..=3492 => "route_lifecycle",
        3493..=4342 => "target",
        4343.. => "route_pool",
        _ => return None,
    })
}

fn add_native_parent(path: &mut syn::Path) {
    let mut segments = path.segments.iter();
    if segments
        .next()
        .is_some_and(|segment| segment.ident == "super")
        && segments
            .next()
            .is_some_and(|segment| segment.ident == "super")
    {
        path.segments.insert(
            0,
            PathSegment {
                ident: syn::Ident::new("super", Span::call_site()),
                arguments: syn::PathArguments::None,
            },
        );
    }
}

struct NativeParentRewriter;

impl VisitMut for NativeParentRewriter {
    fn visit_path_mut(&mut self, path: &mut syn::Path) {
        add_native_parent(path);
        syn::visit_mut::visit_path_mut(self, path);
    }
}

fn shared_file(mut imports: Vec<Item>) -> String {
    for item in &mut imports {
        let Item::Use(import) = item else { continue };
        import.vis = public_crate();
        NativeParentRewriter.visit_item_use_mut(import);
    }
    prettyplease::unparse(&syn::File {
        shebang: None,
        attrs: vec![parse_quote!(#![allow(unused_imports)])],
        items: imports,
    })
}

fn declarations(grouped: &BTreeMap<&str, Vec<Item>>) -> BTreeMap<String, String> {
    let mut declarations = BTreeMap::new();
    for (module, items) in grouped {
        for item in items {
            if let Some(name) = item_name(item) {
                declarations.insert(name, (*module).to_string());
            }
            if let Item::Impl(item_impl) = item {
                let self_type = item_impl
                    .self_ty
                    .to_token_stream()
                    .to_string()
                    .replace(' ', "");
                if item_impl.trait_.is_none() {
                    declarations
                        .entry(self_type)
                        .or_insert_with(|| (*module).to_string());
                }
            }
        }
    }
    declarations
}

fn module_file(
    module: &str,
    mut items: Vec<Item>,
    declarations: &BTreeMap<String, String>,
) -> String {
    for item in &mut items {
        NativeParentRewriter.visit_item_mut(item);
    }
    let body = items
        .iter()
        .map(ToTokens::to_token_stream)
        .map(|value| value.to_string())
        .collect::<String>();
    let mut imports: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (name, owner) in declarations {
        if owner != module && body.contains(name) {
            imports.entry(owner.as_str()).or_default().insert(name);
        }
    }
    let mut generated: Vec<Item> = vec![parse_quote!(
        use super::shared::*;
    )];
    for (owner, names) in imports {
        let owner = syn::Ident::new(owner, Span::call_site());
        let names: Vec<_> = names
            .into_iter()
            .map(|name| syn::Ident::new(name, Span::call_site()))
            .collect();
        generated.push(parse_quote!(use super::#owner::{#(#names),*};));
    }
    generated.extend(items);
    prettyplease::unparse(&syn::File {
        shebang: None,
        attrs: vec![parse_quote!(#![allow(unused_imports)])],
        items: generated,
    })
}

fn write(path: &Path, contents: String) -> Result<(), String> {
    fs::write(path, contents).map_err(|error| format!("write_failed:{}:{error}", path.display()))
}

fn run() -> Result<(), String> {
    let repo = PathBuf::from(env::args().nth(1).ok_or("missing_repo_path")?);
    let source_path = repo.join("cli/src/native/remote_view/open.rs");
    let source = fs::read_to_string(&source_path)
        .map_err(|error| format!("read_failed:{}:{error}", source_path.display()))?;
    let parsed = syn::parse_file(&source).map_err(|error| format!("parse_failed:{error}"))?;
    let mut grouped: BTreeMap<&str, Vec<Item>> =
        MODULES.iter().map(|module| (*module, Vec::new())).collect();
    let mut shared = Vec::new();
    for mut item in parsed.items {
        if let Item::Mod(module) = &item {
            if module.ident == "tests" {
                continue;
            }
        }
        match module_for(&item) {
            Some(module) => {
                expose_item(&mut item);
                grouped.entry(module).or_default().push(item);
            }
            None if matches!(item, Item::Use(_)) => shared.push(item),
            None => return Err(format!("unclassified_item:{}", item.to_token_stream())),
        }
    }
    let declarations = declarations(&grouped);
    let directory = repo.join("cli/src/native/remote_view/open");
    write(&directory.join("shared.rs"), shared_file(shared))?;
    for module in MODULES {
        write(
            &directory.join(format!("{module}.rs")),
            module_file(
                module,
                grouped.remove(module).unwrap_or_default(),
                &declarations,
            ),
        )?;
    }
    let declarations = MODULES
        .iter()
        .map(|module| format!("mod {module};\npub(crate) use {module}::*;"))
        .collect::<Vec<_>>()
        .join("\n");
    write(
        &source_path,
        format!(
            "//! Route-bound browser acquisition and durable handoff resolution.\n\n{declarations}\nmod shared;\n\n#[cfg(test)]\nmod tests;\n"
        ),
    )?;
    println!(
        "split route-bound open into {} explicit modules",
        MODULES.len()
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
