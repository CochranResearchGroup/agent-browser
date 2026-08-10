use proc_macro2::Span;
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{
    Attribute, FnArg, ImplItemFn, Item, ItemFn, ItemImpl, ItemMod, Pat, Signature, TraitItemFn,
};

const SCHEMA_VERSION: &str = "actions-responsibility-inventory.v1";
const GENERATOR_VERSION: &str = "actions-inventory-syn.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Baseline {
    commit: String,
    source_sha256: String,
    source_bytes: usize,
    source_lines: usize,
    production_definition_count: usize,
    in_file_test_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DefinitionRecord {
    id: String,
    full_digest: String,
    item_kind: String,
    owner: String,
    name: String,
    normalized_signature: String,
    baseline_start_line: usize,
    baseline_end_line: usize,
    associated_test_ids: Vec<String>,
    packet: String,
    responsibility: String,
    target_module: String,
    movement_status: String,
    wrapper_owner: Option<String>,
    deletion_packet: Option<String>,
    final_disposition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AllowlistEntry {
    stable_id: String,
    allowed_responsibility: String,
    rationale: String,
    reviewer: String,
    plan_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TargetDepthEntry {
    target_module: String,
    owned_invariant: String,
    interface_operations: Vec<String>,
    production_callers: Vec<String>,
    deletion_test: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Inventory {
    schema_version: String,
    generator_version: String,
    baseline: Baseline,
    stable_identity_convention: String,
    definitions: Vec<DefinitionRecord>,
    predecessor_reconciliation: Option<PredecessorReconciliation>,
    dispatcher_allowlist: Vec<AllowlistEntry>,
    target_depth_ledger: Vec<TargetDepthEntry>,
    final_architecture: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PredecessorReconciliation {
    current_commit: String,
    current_source_sha256: String,
    removed_baseline_ids: Vec<String>,
    added_definitions: Vec<DefinitionRecord>,
}

#[derive(Clone)]
struct RawDefinition {
    item_kind: String,
    owner: String,
    name: String,
    signature: Signature,
    start_line: usize,
    end_line: usize,
    is_test: bool,
    is_test_case: bool,
}

struct DefinitionVisitor {
    definitions: Vec<RawDefinition>,
    owner: String,
    in_test_module: bool,
}

fn has_test_attribute(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("test")
            || (attr.path().is_ident("cfg")
                && attr.meta.to_token_stream().to_string().contains("test"))
    })
}

fn has_test_case_attribute(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "test")
    })
}

fn line_range(span: Span) -> (usize, usize) {
    (span.start().line, span.end().line)
}

impl<'ast> Visit<'ast> for DefinitionVisitor {
    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        let prior = self.in_test_module;
        self.in_test_module |= has_test_attribute(&node.attrs);
        visit::visit_item_mod(self, node);
        self.in_test_module = prior;
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let (start_line, end_line) = line_range(node.span());
        self.definitions.push(RawDefinition {
            item_kind: "function".to_string(),
            owner: self.owner.clone(),
            name: node.sig.ident.to_string(),
            signature: node.sig.clone(),
            start_line,
            end_line,
            is_test: self.in_test_module || has_test_attribute(&node.attrs),
            is_test_case: has_test_case_attribute(&node.attrs),
        });
        visit::visit_block(self, &node.block);
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        let prior = self.owner.clone();
        let self_ty = compact_tokens(node.self_ty.as_ref());
        self.owner = match &node.trait_ {
            Some((_, trait_path, _)) => format!("{} as {}", self_ty, compact_tokens(trait_path)),
            None => self_ty,
        };
        visit::visit_item_impl(self, node);
        self.owner = prior;
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        let (start_line, end_line) = line_range(node.span());
        self.definitions.push(RawDefinition {
            item_kind: "method".to_string(),
            owner: self.owner.clone(),
            name: node.sig.ident.to_string(),
            signature: node.sig.clone(),
            start_line,
            end_line,
            is_test: self.in_test_module || has_test_attribute(&node.attrs),
            is_test_case: has_test_case_attribute(&node.attrs),
        });
        visit::visit_block(self, &node.block);
    }

    fn visit_trait_item_fn(&mut self, node: &'ast TraitItemFn) {
        if let Some(block) = &node.default {
            let (start_line, end_line) = line_range(node.span());
            self.definitions.push(RawDefinition {
                item_kind: "trait_method".to_string(),
                owner: self.owner.clone(),
                name: node.sig.ident.to_string(),
                signature: node.sig.clone(),
                start_line,
                end_line,
                is_test: self.in_test_module || has_test_attribute(&node.attrs),
                is_test_case: has_test_case_attribute(&node.attrs),
            });
            visit::visit_block(self, block);
        }
    }
}

fn compact_tokens(value: &impl ToTokens) -> String {
    value
        .to_token_stream()
        .to_string()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

fn normalized_signature(signature: &Signature) -> String {
    let mut signature = signature.clone();
    for input in &mut signature.inputs {
        if let FnArg::Typed(argument) = input {
            *argument.pat = Pat::Wild(syn::PatWild {
                attrs: Vec::new(),
                underscore_token: Default::default(),
            });
        }
    }
    compact_tokens(&signature)
}

fn sha256_hex(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes.as_ref()))
}

fn stable_identity(definition: &RawDefinition) -> (String, String, String) {
    let signature = normalized_signature(&definition.signature);
    let digest = sha256_hex(format!(
        "{}|{}|{}|{}",
        definition.item_kind, definition.owner, definition.name, signature
    ));
    let id = format!(
        "ari:{}:{}:{}:{}",
        definition.item_kind,
        definition.owner,
        definition.name,
        &digest[..16]
    );
    (id, digest, signature)
}

fn parse_definitions(source: &str) -> Result<Vec<RawDefinition>, String> {
    let file = syn::parse_file(source).map_err(|error| format!("unparseable_source: {error}"))?;
    let mut visitor = DefinitionVisitor {
        definitions: Vec::new(),
        owner: "native::actions".to_string(),
        in_test_module: false,
    };
    visitor.visit_file(&file);
    Ok(visitor.definitions)
}

fn packet_for(line: usize, name: &str) -> (&'static str, &'static str, &'static str) {
    let remote_named = name.contains("remote_view") || name.contains("route_bound_handoff");
    if remote_named && (13_000..=16_287).contains(&line) {
        return (
            "P0101-A",
            "route-bound open transaction",
            "native::remote_view::open",
        );
    }
    let ranges = [
        (
            6_469,
            6_654,
            "P0101-E01-01",
            "browser navigation",
            "native::browser_navigation",
        ),
        (
            6_655,
            6_735,
            "P0101-E01-02",
            "browser inspection",
            "native::browser_inspection",
        ),
        (
            6_736,
            6_817,
            "P0101-E01-03",
            "page inspection",
            "native::browser_inspection",
        ),
        (
            6_818,
            6_853,
            "P0101-E01-04",
            "browser evaluation",
            "native::browser_inspection",
        ),
        (
            6_854,
            7_325,
            "P0101-D02",
            "service probe workflow",
            "native::service_probe",
        ),
        (
            7_326,
            7_741,
            "P0101-D03",
            "service UI action workflow",
            "native::service_ui_action",
        ),
        (
            7_742,
            8_257,
            "P0101-D04",
            "service network capture workflow",
            "native::service_network_capture",
        ),
        (
            8_258,
            9_042,
            "P0101-D05",
            "service file transfer workflow",
            "native::service_file_transfer",
        ),
        (
            9_043,
            9_340,
            "P0101-D01",
            "service diagnostics workflow",
            "native::service_diagnostics",
        ),
        (
            9_666,
            9_870,
            "P0101-E02",
            "page capture",
            "native::page_capture",
        ),
        (
            9_871,
            10_527,
            "P0101-E03",
            "browser interaction",
            "native::interaction",
        ),
        (
            10_528,
            10_607,
            "P0101-E04-01",
            "browser navigation history",
            "native::browser_navigation",
        ),
        (
            10_608,
            10_726,
            "P0101-E04-02",
            "browser waits",
            "native::browser_wait",
        ),
        (
            10_727,
            10_778,
            "P0101-E05-01",
            "cookie operations",
            "native::cookies",
        ),
        (
            10_779,
            10_837,
            "P0101-E05-02",
            "storage operations",
            "native::storage",
        ),
        (
            10_838,
            10_848,
            "P0101-E05-03",
            "page content inspection",
            "native::browser_inspection",
        ),
        (
            10_849,
            10_875,
            "P0101-E05-04",
            "network posture",
            "native::network",
        ),
        (
            10_876,
            10_890,
            "P0101-E05-05",
            "browser diagnostics",
            "native::browser_inspection",
        ),
        (
            10_891,
            10_930,
            "P0101-E05-06",
            "saved browser state",
            "native::state",
        ),
        (
            10_931,
            11_038,
            "P0101-E05-07",
            "browser diff",
            "native::diff",
        ),
        (
            11_039,
            11_083,
            "P0101-E06",
            "browser credentials",
            "native::auth",
        ),
        (
            11_084,
            11_174,
            "P0101-E07",
            "browser input compatibility",
            "native::browser_input",
        ),
        (
            11_175,
            11_264,
            "P0101-E08",
            "browser tabs",
            "native::browser_tabs",
        ),
        (
            14_369,
            14_910,
            "P0101-F01",
            "remote-view preflight",
            "native::remote_view::preflight",
        ),
        (
            14_911,
            15_868,
            "P0101-F02",
            "remote-view route lifecycle",
            "native::remote_view::route_lifecycle",
        ),
        (
            15_869,
            16_287,
            "P0101-F03",
            "remote-view viewer lease",
            "native::remote_view::viewer_lease",
        ),
        (
            16_288,
            16_346,
            "P0101-E09-01",
            "browser emulation",
            "native::browser_emulation",
        ),
        (
            16_347,
            16_506,
            "P0101-E09-02",
            "browser download",
            "native::browser_download",
        ),
        (
            16_507,
            16_543,
            "P0101-E10-01",
            "browser tracing",
            "native::tracing",
        ),
        (
            16_544,
            16_731,
            "P0101-E10-02",
            "browser recording",
            "native::recording",
        ),
        (
            16_732,
            16_782,
            "P0101-E10-03",
            "PDF capture",
            "native::page_capture",
        ),
        (
            16_783,
            16_932,
            "P0101-E11-01",
            "page interaction",
            "native::interaction",
        ),
        (
            16_933,
            17_044,
            "P0101-E11-02",
            "element inspection",
            "native::element",
        ),
        (
            17_045,
            17_070,
            "P0101-E11-03",
            "computed style inspection",
            "native::browser_inspection",
        ),
        (
            17_071,
            17_129,
            "P0101-E11-04",
            "browser context posture",
            "native::browser_context",
        ),
        (
            17_130,
            17_162,
            "P0101-E11-05",
            "dialog interaction",
            "native::interaction",
        ),
        (
            17_163,
            17_207,
            "P0101-E11-06",
            "authorized upload interaction",
            "native::interaction",
        ),
        (
            17_208,
            17_300,
            "P0101-E11-07",
            "page injection",
            "native::page_injection",
        ),
        (
            17_301,
            17_360,
            "P0101-E11-08",
            "clipboard interaction",
            "native::clipboard",
        ),
        (
            17_361,
            17_385,
            "P0101-E11-09",
            "wheel input",
            "native::browser_input",
        ),
        (
            17_386,
            17_432,
            "P0101-E12-01",
            "device emulation",
            "native::browser_emulation",
        ),
        (
            17_433,
            17_582,
            "P0101-E12-02",
            "stream runtime",
            "native::stream_runtime",
        ),
        (
            17_583,
            17_652,
            "P0101-F04",
            "service status projection command",
            "native::service_status_projection",
        ),
        (
            17_653,
            17_762,
            "P0101-F05",
            "service resources",
            "native::service_resources",
        ),
        (
            17_763,
            18_198,
            "P0101-F06",
            "service access",
            "native::service_access",
        ),
        (
            18_199,
            18_342,
            "P0101-F07",
            "service inventory",
            "native::service_inventory",
        ),
        (
            18_343,
            18_436,
            "P0101-F08",
            "service configuration inventory",
            "native::service_inventory",
        ),
        (
            18_437,
            18_718,
            "P0101-F09",
            "service health",
            "native::service_health",
        ),
        (
            18_719,
            19_839,
            "P0101-F10",
            "service retained state",
            "native::service_retained_state",
        ),
        (
            18_882,
            19_257,
            "P0101-F11",
            "route-pool repair",
            "native::remote_view::route_pool_repair",
        ),
        (
            19_840,
            19_853,
            "P0101-F12",
            "service jobs",
            "native::service_jobs",
        ),
        (
            19_854,
            19_914,
            "P0101-F13",
            "service profile configuration",
            "native::service_config",
        ),
        (
            19_915,
            19_937,
            "P0101-F14",
            "service session lifecycle",
            "native::service_lifecycle",
        ),
        (
            19_938,
            19_960,
            "P0101-F15",
            "service site policy configuration",
            "native::service_config",
        ),
        (
            19_961,
            20_046,
            "P0101-F16",
            "service monitors",
            "native::service_monitors",
        ),
        (
            20_047,
            20_069,
            "P0101-F17",
            "provider configuration",
            "native::providers",
        ),
        (
            20_070,
            20_096,
            "P0101-F18",
            "service capability registry",
            "native::service_access",
        ),
        (
            20_097,
            20_135,
            "P0101-F19",
            "service browser retry",
            "native::service_health",
        ),
        (
            20_136,
            20_160,
            "P0101-F20",
            "service remedies",
            "native::service_incidents",
        ),
        (
            20_161,
            20_213,
            "P0101-F21",
            "service incident lifecycle",
            "native::service_incidents",
        ),
        (
            20_214,
            20_266,
            "P0101-F22",
            "service event query",
            "native::service_activity",
        ),
        (
            20_267,
            20_324,
            "P0101-F23",
            "service incident query",
            "native::service_incidents",
        ),
        (
            20_325,
            20_401,
            "P0101-F24",
            "service job query",
            "native::service_jobs",
        ),
        (
            20_402,
            20_416,
            "P0101-F25",
            "incident activity query",
            "native::service_activity",
        ),
        (
            20_417,
            20_718,
            "P0101-F26",
            "service trace query",
            "native::service_trace",
        ),
        (
            20_719,
            20_796,
            "P0101-E13",
            "screencast",
            "native::stream_runtime",
        ),
        (
            20_797,
            21_008,
            "P0101-E14",
            "browser frame and load",
            "native::browser_frame",
        ),
        (
            21_009,
            21_562,
            "P0101-E15",
            "semantic locators",
            "native::browser_locator",
        ),
        (
            21_563,
            21_645,
            "P0101-E16-01",
            "network response body",
            "native::network",
        ),
        (
            21_646,
            21_706,
            "P0101-E16-02",
            "download completion",
            "native::browser_download",
        ),
        (
            21_707,
            21_787,
            "P0101-E16-03",
            "browser windows",
            "native::browser_tabs",
        ),
        (
            21_788,
            21_851,
            "P0101-E16-04",
            "screenshot diff",
            "native::diff",
        ),
        (
            21_852,
            21_888,
            "P0101-E16-05",
            "video recording",
            "native::recording",
        ),
        (
            21_889,
            22_259,
            "P0101-E17-01",
            "network archive",
            "native::network_archive",
        ),
        (
            22_260,
            22_478,
            "P0101-E17-02",
            "network interception",
            "native::network",
        ),
        (
            22_479,
            22_580,
            "P0101-E18-01",
            "network routing",
            "native::network",
        ),
        (
            22_581,
            22_681,
            "P0101-E18-02",
            "network request query",
            "native::network_requests",
        ),
        (
            22_682,
            22_713,
            "P0101-E18-03",
            "HTTP credentials",
            "native::auth",
        ),
        (
            22_714,
            23_048,
            "P0101-E19",
            "auth workflow",
            "native::auth_workflow",
        ),
        (
            23_049,
            23_194,
            "P0101-E20",
            "mobile gestures",
            "native::webdriver::mobile_gestures",
        ),
        (
            23_195,
            23_472,
            "P0101-E21",
            "low-level browser input",
            "native::browser_input",
        ),
    ];
    for (start, end, packet, responsibility, target) in ranges {
        if (start..=end).contains(&line) {
            return (packet, responsibility, target);
        }
    }
    if line < 6_469 {
        (
            "P0101-C",
            "daemon runtime and browser lifecycle",
            "native::daemon_runtime",
        )
    } else if line < 13_000 {
        (
            "P0101-C",
            "browser lifecycle and shared command preparation",
            "native::browser_lifecycle",
        )
    } else if line < 14_369 {
        (
            "P0101-A",
            "route-bound open transaction",
            "native::remote_view::open",
        )
    } else {
        (
            "P0101-G",
            "dispatch and shared coordination closeout",
            "native::action_dispatch",
        )
    }
}

fn read_source(path: &Path) -> Result<String, String> {
    fs::read_to_string(path)
        .map_err(|error| format!("source_read_failed:{}:{error}", path.display()))
}

fn rust_files_below(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("native_source_read_failed:{}:{error}", directory.display()))?
    {
        let path = entry
            .map_err(|error| format!("native_source_entry_failed:{error}"))?
            .path();
        if path.is_dir() {
            files.extend(rust_files_below(&path)?);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(files)
}

fn check_final_module_boundaries(source_path: &Path, source: &str) -> Result<(), String> {
    let parsed = syn::parse_file(source).map_err(|error| format!("parse_failed:{error}"))?;
    if parsed
        .items
        .iter()
        .any(|item| matches!(item, Item::Struct(_) | Item::Enum(_) | Item::Union(_)))
    {
        return Err("typed_domain_definition_in_actions".to_string());
    }
    let native_dir = source_path
        .parent()
        .ok_or_else(|| "actions_parent_missing".to_string())?;
    for facade in [
        native_dir.join("action_runtime/browser_operations.rs"),
        native_dir.join("action_runtime/service_commands.rs"),
        native_dir.join("action_runtime/remote_view_operations.rs"),
    ] {
        if facade.exists() {
            return Err(format!(
                "transitional_facade_still_present:{}",
                facade.display()
            ));
        }
    }
    let allowed_dispatch_consumers = BTreeSet::from([
        PathBuf::from("action_runtime/tests.rs"),
        PathBuf::from("control_plane.rs"),
        PathBuf::from("e2e_tests.rs"),
        PathBuf::from("parity_tests.rs"),
        PathBuf::from("stream/http.rs"),
    ]);
    for path in rust_files_below(native_dir)? {
        let relative = path
            .strip_prefix(native_dir)
            .map_err(|error| format!("native_relative_path_failed:{error}"))?;
        if relative == Path::new("actions.rs") || allowed_dispatch_consumers.contains(relative) {
            continue;
        }
        let candidate = read_source(&path)?;
        if candidate.contains("crate::native::actions")
            || candidate.contains("super::actions")
            || candidate.contains("native::actions")
        {
            return Err(format!("reverse_actions_import:{}", relative.display()));
        }
    }
    Ok(())
}

fn record_for(definition: &RawDefinition, test_ids: &[String]) -> DefinitionRecord {
    let (id, full_digest, signature) = stable_identity(definition);
    let (packet, responsibility, target_module) =
        packet_for(definition.start_line, &definition.name);
    let associated_test_ids = test_ids
        .iter()
        .filter(|test_id| test_id.contains(&definition.name))
        .cloned()
        .collect();
    DefinitionRecord {
        id,
        full_digest,
        item_kind: definition.item_kind.clone(),
        owner: definition.owner.clone(),
        name: definition.name.clone(),
        normalized_signature: signature,
        baseline_start_line: definition.start_line,
        baseline_end_line: definition.end_line,
        associated_test_ids,
        packet: packet.to_string(),
        responsibility: responsibility.to_string(),
        target_module: target_module.to_string(),
        movement_status: "baseline".to_string(),
        wrapper_owner: None,
        deletion_packet: Some(packet.to_string()),
        final_disposition: "move".to_string(),
    }
}

fn generate(
    source_path: &Path,
    inventory_path: &Path,
    commit: &str,
    current_source_path: Option<&Path>,
    current_commit: Option<&str>,
) -> Result<(), String> {
    let source = read_source(source_path)?;
    let definitions = parse_definitions(&source)?;
    let production: Vec<_> = definitions.iter().filter(|item| !item.is_test).collect();
    let tests: Vec<_> = definitions
        .iter()
        .filter(|item| item.is_test_case)
        .collect();
    let test_ids: Vec<_> = tests
        .iter()
        .map(|test| format!("test:{}", test.name))
        .collect();
    let mut ids = BTreeSet::new();
    let mut records = Vec::new();
    for definition in production {
        let record = record_for(definition, &test_ids);
        if !ids.insert(record.id.clone()) {
            return Err(format!("duplicate_stable_id:{}", record.id));
        }
        records.push(record);
    }
    let reviewed_dispatcher_names = BTreeSet::from([
        "action_skips_browser_launch",
        "active_target_binding",
        "handle_dependent_batch",
        "execute_command",
        "success_response",
        "error_response",
    ]);
    for record in &mut records {
        if reviewed_dispatcher_names.contains(record.name.as_str()) {
            record.packet = "P0101-G".to_string();
            record.responsibility = "command dispatch and shared coordination".to_string();
            record.target_module = "native::action_dispatch".to_string();
            record.deletion_packet = None;
            record.final_disposition = "retain".to_string();
        }
    }
    records.sort_by_key(|record| (record.baseline_start_line, record.id.clone()));
    let predecessor_reconciliation = if let Some(current_source_path) = current_source_path {
        let current_source = read_source(current_source_path)?;
        let current_definitions = parse_definitions(&current_source)?;
        let current_test_ids: Vec<_> = current_definitions
            .iter()
            .filter(|item| item.is_test_case)
            .map(|test| format!("test:{}", test.name))
            .collect();
        let current_records: BTreeMap<_, _> = current_definitions
            .iter()
            .filter(|item| !item.is_test)
            .map(|definition| {
                let record = record_for(definition, &current_test_ids);
                (record.id.clone(), record)
            })
            .collect();
        let baseline_ids: BTreeSet<_> = records.iter().map(|record| record.id.clone()).collect();
        let current_ids: BTreeSet<_> = current_records.keys().cloned().collect();
        let removed_baseline_ids: Vec<_> = baseline_ids.difference(&current_ids).cloned().collect();
        for record in &mut records {
            if removed_baseline_ids.contains(&record.id) {
                record.movement_status = "moved".to_string();
                record.packet = "P0100".to_string();
                record.responsibility = "service status projection predecessor".to_string();
                record.target_module = "native::service_status_projection".to_string();
                record.deletion_packet = Some("P0100".to_string());
                record.final_disposition = "predecessor_moved".to_string();
            }
        }
        let mut added_definitions: Vec<_> = current_ids
            .difference(&baseline_ids)
            .filter_map(|id| current_records.get(id).cloned())
            .collect();
        for record in &mut added_definitions {
            record.packet = "P0101-F04".to_string();
            record.responsibility = "service status projection command".to_string();
            record.target_module = "native::service_status_projection".to_string();
            record.deletion_packet = Some("P0101-F04".to_string());
        }
        Some(PredecessorReconciliation {
            current_commit: current_commit.unwrap_or("unknown").to_string(),
            current_source_sha256: sha256_hex(current_source.as_bytes()),
            removed_baseline_ids,
            added_definitions,
        })
    } else {
        None
    };
    let dispatcher_allowlist = records
        .iter()
        .filter(|record| record.final_disposition == "retain")
        .map(|record| AllowlistEntry {
            stable_id: record.id.clone(),
            allowed_responsibility: record.responsibility.clone(),
            rationale: "Required at the command dispatch seam after all domain decisions move behind deep module interfaces".to_string(),
            reviewer: "P0101 executor P0 review".to_string(),
            plan_version: "P0101 v2 plus P0102".to_string(),
        })
        .collect();
    let mut target_groups: BTreeMap<String, (BTreeSet<String>, Vec<String>)> = BTreeMap::new();
    for record in &records {
        if record.final_disposition == "retain" {
            continue;
        }
        let group = target_groups
            .entry(record.target_module.clone())
            .or_insert_with(|| (BTreeSet::new(), Vec::new()));
        group.0.insert(record.responsibility.clone());
        if group.1.len() < 12 && !group.1.contains(&record.name) {
            group.1.push(record.name.clone());
        }
    }
    let target_depth_ledger = target_groups
        .into_iter()
        .map(|(target_module, (responsibilities, operations))| {
            let owned_invariant = responsibilities.into_iter().collect::<Vec<_>>().join("; ");
            TargetDepthEntry {
                target_module: target_module.clone(),
                owned_invariant: owned_invariant.clone(),
                interface_operations: operations,
                production_callers: vec!["native::action_dispatch".to_string()],
                deletion_test: format!(
                    "Deleting {target_module} would force dispatch or its callers to reproduce the ordered {owned_invariant} decisions and safety checks"
                ),
            }
        })
        .collect();
    let inventory = Inventory {
        schema_version: SCHEMA_VERSION.to_string(),
        generator_version: GENERATOR_VERSION.to_string(),
        baseline: Baseline {
            commit: commit.to_string(),
            source_sha256: sha256_hex(source.as_bytes()),
            source_bytes: source.len(),
            source_lines: source.lines().count(),
            production_definition_count: records.len(),
            in_file_test_count: tests.len(),
        },
        stable_identity_convention: "ari:<item-kind>:<qualified-impl-or-trait-owner>:<name>:<normalized-signature-sha256-prefix-16>; source locations and parameter binding names are excluded".to_string(),
        definitions: records,
        predecessor_reconciliation,
        dispatcher_allowlist,
        target_depth_ledger,
        final_architecture: false,
    };
    let json = serde_json::to_string_pretty(&inventory)
        .map_err(|error| format!("inventory_serialize_failed:{error}"))?;
    fs::write(inventory_path, format!("{json}\n")).map_err(|error| {
        format!(
            "inventory_write_failed:{}:{error}",
            inventory_path.display()
        )
    })?;
    println!(
        "generated definitions={} tests={} source_sha256={} inventory={}",
        inventory.baseline.production_definition_count,
        inventory.baseline.in_file_test_count,
        inventory.baseline.source_sha256,
        inventory_path.display()
    );
    Ok(())
}

fn check(source_path: &Path, inventory_path: &Path) -> Result<(), String> {
    let source = read_source(source_path)?;
    let inventory: Inventory =
        serde_json::from_str(&fs::read_to_string(inventory_path).map_err(|error| {
            format!("inventory_read_failed:{}:{error}", inventory_path.display())
        })?)
        .map_err(|error| format!("inventory_parse_failed:{error}"))?;
    if inventory.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "unknown_schema_version:{}",
            inventory.schema_version
        ));
    }
    let definitions = parse_definitions(&source)?;
    let current: Vec<_> = definitions
        .iter()
        .filter(|definition| !definition.is_test)
        .collect();
    let current_tests = definitions
        .iter()
        .filter(|definition| definition.is_test)
        .count();
    let all_records: Vec<_> = inventory
        .definitions
        .iter()
        .chain(
            inventory
                .predecessor_reconciliation
                .iter()
                .flat_map(|reconciliation| reconciliation.added_definitions.iter()),
        )
        .collect();
    let inventory_by_id: BTreeMap<_, _> = all_records
        .iter()
        .copied()
        .map(|record| (record.id.as_str(), record))
        .collect();
    let mut seen = BTreeSet::new();
    for definition in &current {
        let (id, digest, _) = stable_identity(definition);
        let Some(record) = inventory_by_id.get(id.as_str()) else {
            return Err(format!("unclassified_definition:{id}"));
        };
        if record.full_digest != digest {
            return Err(format!("identity_digest_mismatch:{id}"));
        }
        if !seen.insert(id.clone()) {
            return Err(format!("duplicate_stable_id:{id}"));
        }
        if !matches!(record.movement_status.as_str(), "baseline" | "retained") {
            return Err(format!("moved_definition_still_present:{id}"));
        }
    }
    for record in all_records {
        if !seen.contains(&record.id)
            && !matches!(record.movement_status.as_str(), "moved" | "deleted")
        {
            return Err(format!(
                "missing_definition_without_disposition:{}",
                record.id
            ));
        }
        if !matches!(
            record.movement_status.as_str(),
            "baseline" | "retained" | "moved" | "deleted"
        ) {
            return Err(format!("unknown_movement_status:{}", record.id));
        }
        if record.packet.trim().is_empty() || record.target_module.trim().is_empty() {
            return Err(format!("unclassified_definition:{}", record.id));
        }
    }
    let wrapper_count = inventory
        .definitions
        .iter()
        .chain(
            inventory
                .predecessor_reconciliation
                .iter()
                .flat_map(|reconciliation| reconciliation.added_definitions.iter()),
        )
        .filter(|record| record.wrapper_owner.is_some())
        .count();
    if inventory.final_architecture {
        check_final_module_boundaries(source_path, &source)?;
        if source.lines().count() > 2_500 {
            return Err(format!(
                "actions_line_budget_exceeded:{}",
                source.lines().count()
            ));
        }
        if current.len() > 35 {
            return Err(format!(
                "actions_definition_budget_exceeded:{}",
                current.len()
            ));
        }
        if current_tests > 20 {
            return Err(format!("actions_test_budget_exceeded:{current_tests}"));
        }
        if wrapper_count != 0 {
            return Err(format!("compatibility_wrapper_count:{wrapper_count}"));
        }
        let allowed: BTreeSet<_> = inventory
            .dispatcher_allowlist
            .iter()
            .map(|entry| entry.stable_id.as_str())
            .collect();
        for id in &seen {
            if !allowed.contains(id.as_str()) {
                return Err(format!("unreviewed_retained_definition:{id}"));
            }
        }
        for forbidden in [
            "reqwest::",
            "std::process::Command",
            "LockedServiceStateRepository",
            "ServiceState",
            ".mutate(",
            "send_command(",
            "route_pool_entries",
            "acquisition_leases",
            "durable_remote_view_handoffs",
            "XOpenDisplay",
        ] {
            if source.contains(forbidden) {
                return Err(format!("forbidden_actions_dependency:{forbidden}"));
            }
        }
    }
    println!(
        "actions structural regression check passed definitions={} tests={} lines={} wrappers={} final={}",
        current.len(),
        current_tests,
        source.lines().count(),
        wrapper_count,
        inventory.final_architecture
    );
    Ok(())
}

fn self_test(fixture_dir: &Path) -> Result<(), String> {
    let classified = read_source(&fixture_dir.join("classified-action.rs"))?;
    let unclassified = read_source(&fixture_dir.join("unclassified-action.rs"))?;
    let classified_definitions = parse_definitions(&classified)?;
    let unclassified_definitions = parse_definitions(&unclassified)?;
    if classified_definitions.len() != 1 || unclassified_definitions.len() != 1 {
        return Err("self_test_fixture_shape_failed".to_string());
    }
    let (classified_id, _, _) = stable_identity(&classified_definitions[0]);
    let allowed = BTreeSet::from([classified_id]);
    let (candidate_id, _, _) = stable_identity(&unclassified_definitions[0]);
    if allowed.contains(&candidate_id) {
        return Err("self_test_unclassified_fixture_was_accepted".to_string());
    }
    println!(
        "self-test passed classified_fixture=accepted unclassified_fixture=unclassified_definition:{}",
        candidate_id
    );
    Ok(())
}

fn value_after(args: &[String], name: &str) -> Result<String, String> {
    let index = args
        .iter()
        .position(|arg| arg == name)
        .ok_or_else(|| format!("missing_argument:{name}"))?;
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("missing_argument_value:{name}"))
}

fn optional_value_after(args: &[String], name: &str) -> Option<String> {
    let index = args.iter().position(|arg| arg == name)?;
    args.get(index + 1).cloned()
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mode = args.first().ok_or("missing_mode")?.as_str();
    match mode {
        "generate" => {
            let current_source = optional_value_after(&args, "--current-source").map(PathBuf::from);
            let current_commit = optional_value_after(&args, "--current-commit");
            generate(
                &PathBuf::from(value_after(&args, "--source")?),
                &PathBuf::from(value_after(&args, "--inventory")?),
                &value_after(&args, "--commit")?,
                current_source.as_deref(),
                current_commit.as_deref(),
            )
        }
        "check" => check(
            &PathBuf::from(value_after(&args, "--source")?),
            &PathBuf::from(value_after(&args, "--inventory")?),
        ),
        "self-test" => self_test(&PathBuf::from(value_after(&args, "--fixtures")?)),
        _ => Err(format!("unknown_mode:{mode}")),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
