use proc_macro2::Span;
use quote::ToTokens;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use syn::spanned::Spanned;
use syn::{parse_quote, Field, ImplItem, Item, ItemMod, Visibility};

const MODULES: &[&str] = &[
    "common",
    "runtime",
    "service_workflows",
    "browser_operations",
    "remote_view_operations",
    "service_commands",
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

fn line(span: Span) -> usize {
    span.start().line
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

fn module_for(item: &Item) -> &'static str {
    if matches!(item, Item::Use(_) | Item::ExternCrate(_)) {
        return "common";
    }
    let start = line(item.span());
    let name = item_name(item).unwrap_or_default();
    if name.contains("remote_view") || name.contains("route_bound_handoff") {
        return "remote_view_operations";
    }
    if start < 6_469 || (9_341..=9_665).contains(&start) {
        return "runtime";
    }
    if (6_854..=9_340).contains(&start) {
        return "service_workflows";
    }
    if (13_000..=16_287).contains(&start) {
        return "remote_view_operations";
    }
    if (17_583..=20_718).contains(&start) {
        return "service_commands";
    }
    "browser_operations"
}

fn module_file(items: Vec<Item>, module: &str) -> String {
    let mut imports = Vec::new();
    for other in MODULES {
        if *other != module {
            let ident = syn::Ident::new(other, Span::call_site());
            imports.push(parse_quote!(use super::#ident::*;));
        }
    }
    let mut all_items = imports;
    all_items.extend(items);
    let file = syn::File {
        shebang: None,
        attrs: vec![parse_quote!(#![allow(unused_imports)])],
        items: all_items,
    };
    prettyplease::unparse(&file)
}

fn tests_file(module: ItemMod) -> Result<String, String> {
    let Some((_, items)) = module.content else {
        return Err("actions_tests_module_must_be_inline".to_string());
    };
    let file = syn::File {
        shebang: None,
        attrs: vec![parse_quote!(#![allow(unused_imports)])],
        items,
    };
    Ok(prettyplease::unparse(&file))
}

fn write(path: &Path, contents: String) -> Result<(), String> {
    fs::write(path, contents).map_err(|error| format!("write_failed:{}:{error}", path.display()))
}

fn run() -> Result<(), String> {
    let repo = PathBuf::from(env::args().nth(1).ok_or("missing_repo_path")?);
    let actions_path = repo.join("cli/src/native/actions.rs");
    let source = if let Some(git_ref) = env::args().nth(2) {
        let output = Command::new("git")
            .args(["show", &format!("{git_ref}:cli/src/native/actions.rs")])
            .current_dir(&repo)
            .output()
            .map_err(|error| format!("git_show_start_failed:{error}"))?;
        if !output.status.success() {
            return Err(format!(
                "git_show_failed:{}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        String::from_utf8(output.stdout).map_err(|error| format!("git_show_utf8_failed:{error}"))?
    } else {
        fs::read_to_string(&actions_path)
            .map_err(|error| format!("read_failed:{}:{error}", actions_path.display()))?
    };
    let parsed = syn::parse_file(&source).map_err(|error| format!("parse_failed:{error}"))?;
    let mut grouped: BTreeMap<&str, Vec<Item>> =
        MODULES.iter().map(|module| (*module, Vec::new())).collect();
    let mut tests = None;
    for mut item in parsed.items {
        if let Item::Mod(module) = &item {
            if module.ident == "tests"
                && module
                    .attrs
                    .iter()
                    .any(|attr| attr.to_token_stream().to_string().contains("test"))
            {
                tests = Some(module.clone());
                continue;
            }
        }
        let owner = module_for(&item);
        expose_item(&mut item);
        grouped.entry(owner).or_default().push(item);
    }
    let action_runtime_dir = repo.join("cli/src/native/action_runtime");
    fs::create_dir_all(&action_runtime_dir)
        .map_err(|error| format!("create_dir_failed:{}:{error}", action_runtime_dir.display()))?;
    for module in MODULES {
        let items = grouped.remove(module).unwrap_or_default();
        write(
            &action_runtime_dir.join(format!("{module}.rs")),
            module_file(items, module),
        )?;
    }
    write(
        &action_runtime_dir.join("tests.rs"),
        tests_file(tests.ok_or("missing_actions_tests_module")?)?,
    )?;
    let parent = r#"mod browser_operations;
mod common;
mod remote_view_operations;
mod runtime;
mod service_commands;
mod service_workflows;

pub(crate) use browser_operations::*;
pub(crate) use common::*;
pub(crate) use remote_view_operations::*;
pub(crate) use runtime::*;
pub(crate) use service_commands::*;
pub(crate) use service_workflows::*;

#[cfg(test)]
mod tests;
"#;
    write(
        &repo.join("cli/src/native/action_runtime.rs"),
        parent.to_string(),
    )?;
    let actions = r#"//! Serialized daemon command dispatch compatibility seam.
//!
//! Domain behavior lives in cohesive owners under `action_runtime`; callers
//! retain this path while the command vocabulary remains wire-compatible.

pub(crate) use super::action_runtime::{
    action_skips_browser_launch, execute_command, DaemonState,
};
"#;
    write(&actions_path, actions.to_string())?;
    println!(
        "split actions.rs into {} cohesive source owners plus tests",
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
