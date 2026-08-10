use proc_macro2::Span;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use syn::parse_quote;
use syn::visit_mut::{self, VisitMut};
use syn::{Item, Path as SynPath, PathSegment, Visibility};

const MODULES: &[&str] = &[
    "browser_operations",
    "common",
    "remote_view_operations",
    "runtime",
    "service_commands",
    "service_workflows",
];

struct NativePathRewriter;

impl VisitMut for NativePathRewriter {
    fn visit_path_mut(&mut self, path: &mut SynPath) {
        let mut segments = path.segments.iter();
        let first = segments.next().map(|segment| segment.ident.to_string());
        let second = segments.next().map(|segment| segment.ident.to_string());
        if first.as_deref() == Some("super")
            && !second
                .as_deref()
                .is_some_and(|segment| MODULES.contains(&segment))
        {
            path.segments.insert(
                0,
                PathSegment {
                    ident: syn::Ident::new("super", Span::call_site()),
                    arguments: syn::PathArguments::None,
                },
            );
        }
        visit_mut::visit_path_mut(self, path);
    }
}

fn fix_file(path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("read_failed:{}:{error}", path.display()))?;
    let mut file = syn::parse_file(&source).map_err(|error| format!("parse_failed:{error}"))?;
    NativePathRewriter.visit_file_mut(&mut file);
    for item in &mut file.items {
        if let Item::Use(item) = item {
            if matches!(item.vis, Visibility::Inherited) {
                item.vis = parse_quote!(pub(crate));
            }
        }
    }
    fs::write(path, prettyplease::unparse(&file))
        .map_err(|error| format!("write_failed:{}:{error}", path.display()))
}

fn run() -> Result<(), String> {
    let repo = PathBuf::from(env::args().nth(1).ok_or("missing_repo_path")?);
    let directory = repo.join("cli/src/native/action_runtime");
    for module in MODULES {
        fix_file(&directory.join(format!("{module}.rs")))?;
    }
    println!("rewrote native-relative paths for split action modules");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
