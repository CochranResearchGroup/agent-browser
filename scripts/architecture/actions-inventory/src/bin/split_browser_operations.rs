use proc_macro2::Span;
use quote::ToTokens;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use syn::visit::Visit;
use syn::{parse_quote, ImplItem, Item, ItemMod, ItemUse, Visibility};

struct SplitSpec {
    source_commit: &'static str,
    source_path: &'static str,
    nested_module: &'static str,
    label: &'static str,
}

fn split_spec(mode: &str) -> Result<SplitSpec, String> {
    match mode {
        "browser" => Ok(SplitSpec {
            source_commit: "ef15a932",
            source_path: "cli/src/native/action_runtime/browser_operations.rs",
            nested_module: "action_commands",
            label: "browser_operation",
        }),
        "service" => Ok(SplitSpec {
            source_commit: "de5ec433",
            source_path: "cli/src/native/action_runtime/service_commands.rs",
            nested_module: "service_commands",
            label: "service_command",
        }),
        _ => Err(format!("unsupported_split_mode:{mode}")),
    }
}

fn source_at_commit(repo: &Path, commit: &str, path: &str) -> Result<String, String> {
    let output = Command::new("git")
        .args(["show", &format!("{commit}:{path}")])
        .current_dir(repo)
        .output()
        .map_err(|error| format!("git_show_start_failed:{path}:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git_show_failed:{path}:{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| format!("git_show_utf8_failed:{path}:{error}"))
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

fn impl_owner(item: &Item) -> Option<String> {
    let Item::Impl(item) = item else {
        return None;
    };
    let syn::Type::Path(owner) = item.self_ty.as_ref() else {
        return None;
    };
    owner
        .path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

fn expose_item(item: &mut Item) {
    let public_crate: Visibility = parse_quote!(pub(crate));
    match item {
        Item::Const(item) => item.vis = public_crate,
        Item::Enum(item) => item.vis = public_crate,
        Item::Fn(item) => item.vis = public_crate,
        Item::Static(item) => item.vis = public_crate,
        Item::Struct(item) => {
            item.vis = public_crate.clone();
            for field in &mut item.fields {
                if matches!(field.vis, Visibility::Inherited) {
                    field.vis = public_crate.clone();
                }
            }
        }
        Item::Trait(item) => item.vis = public_crate,
        Item::Type(item) => item.vis = public_crate,
        Item::Union(item) => item.vis = public_crate,
        Item::Impl(item) if item.trait_.is_none() => {
            for implementation_item in &mut item.items {
                if let ImplItem::Fn(method) = implementation_item {
                    if matches!(method.vis, Visibility::Inherited) {
                        method.vis = public_crate.clone();
                    }
                }
            }
        }
        _ => {}
    }
}

fn target_map(repo: &Path) -> Result<BTreeMap<String, String>, String> {
    let inventory_path =
        repo.join("docs/dev/architecture/actions-responsibility-inventory.v1.json");
    let inventory: Value = serde_json::from_str(
        &fs::read_to_string(&inventory_path)
            .map_err(|error| format!("inventory_read_failed:{error}"))?,
    )
    .map_err(|error| format!("inventory_parse_failed:{error}"))?;
    let mut targets = BTreeMap::new();
    for collection in [
        inventory.get("definitions").and_then(Value::as_array),
        inventory
            .pointer("/predecessorReconciliation/addedDefinitions")
            .and_then(Value::as_array),
    ]
    .into_iter()
    .flatten()
    {
        for record in collection {
            let Some(name) = record.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(target) = record.get("targetModule").and_then(Value::as_str) else {
                continue;
            };
            targets.insert(name.to_string(), target.to_string());
        }
    }
    targets.insert(
        "ConfirmationExecution".to_string(),
        "native::auth_workflow".to_string(),
    );
    targets.insert(
        "begin_confirmation".to_string(),
        "native::auth_workflow".to_string(),
    );
    for (name, target) in [
        ("BrowserPreferenceCommandInput", "native::service_access"),
        (
            "ServiceRetentionPruneOptions",
            "native::service_retained_state",
        ),
        (
            "ServiceRetentionRepairOptions",
            "native::service_retained_state",
        ),
        (
            "ServiceRoutePoolRepairOptions",
            "native::remote_view::route_pool_repair",
        ),
        ("SessionAgeStatus", "native::service_retained_state"),
    ] {
        targets.insert(name.to_string(), target.to_string());
    }
    Ok(targets)
}

#[derive(Default)]
struct ReferencedNames {
    names: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for ReferencedNames {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        if let Some(segment) = path.segments.last() {
            self.names.insert(segment.ident.to_string());
        }
        syn::visit::visit_path(self, path);
    }
}

fn tokens_names(item: &Item) -> BTreeSet<String> {
    item.to_token_stream()
        .to_string()
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .filter(|token| !token.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn target_path(target: &str) -> Result<String, String> {
    target
        .strip_prefix("native::")
        .map(ToString::to_string)
        .ok_or_else(|| format!("unsupported_target:{target}"))
}

fn target_file(repo: &Path, target: &str) -> Result<PathBuf, String> {
    let relative = target_path(target)?.replace("::", "/");
    Ok(repo.join("cli/src/native").join(format!("{relative}.rs")))
}

fn import_for(target: &str, nested_module: &str, names: &[String]) -> Result<ItemUse, String> {
    let path = target
        .strip_prefix("native::")
        .ok_or_else(|| format!("unsupported_import_target:{target}"))?;
    syn::parse_str(&format!(
        "use crate::native::{}::{}::{{{}}};",
        path,
        nested_module,
        names.join(", ")
    ))
    .map_err(|error| format!("cross_import_parse_failed:{target}:{error}"))
}

fn module_source(
    items: Vec<Item>,
    target: &str,
    declarations: &BTreeMap<String, String>,
    nested_module: &str,
) -> Result<String, String> {
    let mut referenced = ReferencedNames::default();
    let mut token_names = BTreeSet::new();
    for item in &items {
        referenced.visit_item(item);
        token_names.extend(tokens_names(item));
    }
    referenced.names.extend(token_names);
    let mut cross: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for name in referenced.names {
        let Some(owner) = declarations.get(&name) else {
            continue;
        };
        if owner == target {
            continue;
        }
        cross.entry(owner.clone()).or_default().push(name);
    }
    let mut module_items: Vec<Item> = vec![
        parse_quote!(
            use crate::native::action_runtime::common::*;
        ),
        parse_quote!(
            use crate::native::action_runtime::runtime::{
                account_ids_from_command, apply_service_browser_capability_selection,
                browser_build_from_command, browser_build_label, browser_host_from_command,
                handle_close, is_stale_page_session_error, launch_profile_from_sources,
                optional_command_string, parse_control_input_provider, parse_view_stream_provider,
                recover_browser_command_channel, registry_string_field, relaunch_and_restore_page,
                remote_headed_display_isolation_from_command, runtime_profile_from_sources,
                service_browser_id, target_service_ids_from_command, target_url_from_command,
                validate_service_tab_handle_for_current_session,
                validate_service_tab_handle_route_for_current_session, DaemonState,
                FetchPausedRequest, HarEntry, MouseState, RouteEntry, RouteResponse,
                TrackedRequest, AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS,
                AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS, AUTH_LOGIN_WAIT_UNTIL,
            };
        ),
        parse_quote!(
            use crate::native::service_diagnostics::truncate_utf8;
        ),
    ];
    for (owner, mut names) in cross {
        names.sort();
        names.dedup();
        module_items.push(Item::Use(import_for(&owner, nested_module, &names)?));
    }
    module_items.extend(items);
    let module: ItemMod = ItemMod {
        attrs: vec![parse_quote!(#[allow(dead_code, unused_imports)])],
        vis: parse_quote!(pub(crate)),
        unsafety: None,
        mod_token: Default::default(),
        ident: syn::Ident::new(nested_module, Span::call_site()),
        content: Some((Default::default(), module_items)),
        semi: None,
    };
    let nested_ident = syn::Ident::new(nested_module, Span::call_site());
    let file = syn::File {
        shebang: None,
        attrs: Vec::new(),
        items: vec![
            Item::Mod(module),
            parse_quote!(
                pub(crate) use #nested_ident::*;
            ),
        ],
    };
    Ok(prettyplease::unparse(&file))
}

fn ensure_native_modules(repo: &Path, targets: &BTreeSet<String>) -> Result<(), String> {
    let mod_path = repo.join("cli/src/native/mod.rs");
    let mut source =
        fs::read_to_string(&mod_path).map_err(|error| format!("native_mod_read_failed:{error}"))?;
    let existing: BTreeSet<_> = syn::parse_file(&source)
        .map_err(|error| format!("native_mod_parse_failed:{error}"))?
        .items
        .into_iter()
        .filter_map(|item| match item {
            Item::Mod(module) => Some(module.ident.to_string()),
            _ => None,
        })
        .collect();
    let mut additions = String::new();
    for target in targets {
        let path = target_path(target)?;
        if path.contains("::") || existing.contains(&path) {
            continue;
        }
        additions.push_str(&format!("#[allow(dead_code)]\npub mod {path};\n"));
    }
    let marker = "#[allow(dead_code)]\npub(crate) mod action_runtime;\n";
    source = source.replace(marker, &format!("{marker}{additions}"));
    fs::write(&mod_path, source).map_err(|error| format!("native_mod_write_failed:{error}"))?;
    if targets.contains("native::webdriver::mobile_gestures") {
        let webdriver_mod_path = repo.join("cli/src/native/webdriver/mod.rs");
        let mut webdriver_mod = fs::read_to_string(&webdriver_mod_path)
            .map_err(|error| format!("webdriver_mod_read_failed:{error}"))?;
        if !webdriver_mod.contains("pub mod mobile_gestures;") {
            webdriver_mod =
                webdriver_mod.replace("pub mod ios;\n", "pub mod ios;\npub mod mobile_gestures;\n");
        }
        fs::write(&webdriver_mod_path, webdriver_mod)
            .map_err(|error| format!("webdriver_mod_write_failed:{error}"))?;
    }
    if targets.contains("native::remote_view::route_pool_repair") {
        let remote_view_path = repo.join("cli/src/native/remote_view.rs");
        let mut remote_view = fs::read_to_string(&remote_view_path)
            .map_err(|error| format!("remote_view_read_failed:{error}"))?;
        if !remote_view.contains("pub(crate) mod route_pool_repair;") {
            remote_view = remote_view.replace(
                "pub(crate) mod open;\n",
                "pub(crate) mod open;\npub(crate) mod route_pool_repair;\n",
            );
        }
        fs::write(&remote_view_path, remote_view)
            .map_err(|error| format!("remote_view_write_failed:{error}"))?;
    }
    Ok(())
}

fn run() -> Result<(), String> {
    let repo = PathBuf::from(env::args().nth(1).ok_or("missing_repo_path")?);
    let mode = env::args().nth(2).unwrap_or_else(|| "browser".to_string());
    let spec = split_spec(&mode)?;
    let source = source_at_commit(&repo, spec.source_commit, spec.source_path)?;
    let parsed = syn::parse_file(&source).map_err(|error| format!("parse_failed:{error}"))?;
    let targets = target_map(&repo)?;
    let mut declarations = BTreeMap::new();
    for item in &parsed.items {
        if matches!(item, Item::Use(_)) {
            continue;
        }
        if let Some(name) = item_name(item) {
            let target = targets
                .get(&name)
                .ok_or_else(|| format!("unclassified_browser_item:{name}"))?;
            declarations.insert(name, target.clone());
        }
    }
    let mut grouped: BTreeMap<String, Vec<Item>> = BTreeMap::new();
    for mut item in parsed.items {
        if matches!(item, Item::Use(_)) {
            continue;
        }
        let target = if let Some(name) = item_name(&item) {
            targets
                .get(&name)
                .cloned()
                .ok_or_else(|| format!("unclassified_browser_item:{name}"))?
        } else if let Some(owner) = impl_owner(&item) {
            targets
                .get(&owner)
                .cloned()
                .ok_or_else(|| format!("unclassified_browser_impl:{owner}"))?
        } else {
            return Err(format!(
                "unsupported_browser_item:{}",
                item.to_token_stream()
            ));
        };
        expose_item(&mut item);
        grouped.entry(target).or_default().push(item);
    }
    let target_names: BTreeSet<_> = grouped.keys().cloned().collect();
    ensure_native_modules(&repo, &target_names)?;
    for (target, items) in grouped {
        let file_path = target_file(&repo, &target)?;
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!("target_dir_create_failed:{}:{error}", parent.display())
            })?;
        }
        let existing = if file_path.exists() {
            fs::read_to_string(&file_path)
                .map_err(|error| format!("target_read_failed:{}:{error}", file_path.display()))?
        } else {
            String::new()
        };
        if existing.contains(&format!("mod {}", spec.nested_module)) {
            return Err(format!("target_already_split:{}", file_path.display()));
        }
        let addition = module_source(items, &target, &declarations, spec.nested_module)?;
        fs::write(&file_path, format!("{}\n{}", existing.trim_end(), addition))
            .map_err(|error| format!("target_write_failed:{}:{error}", file_path.display()))?;
    }
    let facade = target_names
        .iter()
        .map(|target| {
            let path = target.strip_prefix("native::").unwrap();
            format!("pub(crate) use crate::native::{path}::*;")
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(
        repo.join(spec.source_path),
        format!(
            "//! Transitional {} ownership facade.\n\n#![allow(unused_imports)]\n{facade}\n",
            spec.label
        ),
    )
    .map_err(|error| format!("facade_write_failed:{error}"))?;
    println!(
        "{}_modules={} declarations={}",
        spec.label,
        target_names.len(),
        declarations.len()
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
