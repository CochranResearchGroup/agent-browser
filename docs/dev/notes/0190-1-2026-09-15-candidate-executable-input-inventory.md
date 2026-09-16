# Plan 0190 Candidate Executable-Input Inventory

Date: 2026-09-15

Product lane: PL-PLATFORM

Disposition: active-input

Owning plan or work item: Plan 0190 and issue #136

Related lanes: P169, P196

## Purpose

This note freezes the first implementation packet's input boundary before a
candidate collector or production-shaped build is introduced. It distinguishes
artifact provenance from the content and build configuration used to decide
whether one already sealed artifact may be reused after integration.

No build or runtime effect was performed for this inventory. The empirical
source was the newest existing release dep-info file under the canonical
checkout's `cli/target/release/deps/` directory, plus the current workspace
manifests, root Cargo profiles, `cli/build.rs`, and literal embedding sites.

## Executable-input closure

The content portion must bind normalized repository-relative paths and SHA-256
digests for:

- every Rust source compiled into the CLI and every linked local workspace
  crate;
- the root and participating package `Cargo.toml` files, `Cargo.lock`, Cargo
  configuration, and any selected toolchain file;
- `cli/build.rs` and every other participating package build script;
- the complete `packages/dashboard/out/` tree embedded by
  `cli/src/native/stream/http.rs`;
- the workstation assets under `cli/assets/` that are embedded into the
  executable;
- the installer, privilege helper, RDP, Guacamole, and environment scripts
  named by production `include_str!` sites;
- `config/site-login-recipes/bill-login-v1.json`, which is embedded by the
  current CLI; and
- any other non-Rust input reported by Cargo dep-info for the selected release
  build.

The identity also binds the target triple, exact Rust toolchain, Cargo profile,
resolved profile configuration, sorted feature set, and SHA-256 digests of an
explicitly reviewed allowlist of build-affecting environment values. Raw
environment values are not stored in the manifest.

The collector must use Cargo's selected package graph and compiler dep-info as
the authoritative inclusion evidence. A directory-only rule is insufficient:
some files under `docs/dev/fixtures/` are compile-time inputs for test targets,
while ordinary documentation is not an input to the production binary.

## Provenance boundary

`cli/build.rs` embeds the exact source revision and the clean, dirty, or unknown
tree state. Those values remain mandatory candidate provenance. They are not
part of post-merge content equivalence because the sealed pre-merge artifact
must retain its original provenance while canonical `origin/main` gains a merge
commit. Rebuilding at the merge commit would produce different embedded
provenance and defeats the build-once objective.

Production-shaped promotion still requires the original build provenance to
name a clean source commit, that commit to be an ancestor of current canonical
`origin/main`, and a freshly computed executable-input closure at `origin/main`
to be equivalent. A docs-only or merge-metadata-only difference therefore does
not force a rebuild, but any changed included path or build dimension does.

## Completeness interlock

The existing build script creates a placeholder dashboard when
`packages/dashboard/out/index.html` is absent. That behavior is acceptable for
ordinary Rust iteration but not promotion evidence. A production-shaped
manifest fails closed unless it binds at least one dashboard output and the
embedded support assets. Later adapter work must add the stronger dashboard
completeness check before a candidate becomes promotable.

## First kernel contract

The new `agent-browser-candidate` crate accepts already reviewed digests and
owns deterministic canonicalization, equivalence, manifest identity, advice,
and fenced idempotent state transitions. It does not inspect the repository,
run Cargo, store artifacts, or mutate a runtime. Those remain adapter concerns
for later Plan 0190 packets.
