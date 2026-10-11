# Native desktop dashboard and independent handoff lifetimes

Product lane: PL-PLATFORM
Work items: #261, #260
Branch: platform/native-dashboard-261
State: IN_PROGRESS
Version: 1
Lane: P237
Baseline: 3e5a38679d28ac8a20439e29004ea1315475ef8e

## Problem and agreed outcome

Replace the existing operator dashboard with a simple workspace modeled on Remote View's root desktop. A person expands a browser app to its tabs, selects a tab to select its native target and raise/maximize its owning browser on the correct desktop, shares a durable handoff, and finds handoffs in a prominent compact table. Properties explains attributable origin and purpose. Keep the native Remote View viewer, desktop operations and existing authentication authoritative.

Link lifetime defaults perpetual but is configurable. Tab lifetime is independently configurable. Link deletion explicitly revokes a link without closing its tab/browser. Link expiry does not end a tab or delete a record. The browser closes when its last live tab ends. A still-valid link recreates its expired target tab in an eligible browser or wakes a browser as needed; this is settled behavior, not an interview question. Recovery does not promise restoration of lost in-memory state. Pending-human tab retention and pressure policy are configuration driven; four hours is an example.

## Authority and source custody

Issues [261](https://github.com/CochranResearchGroup/agent-browser/issues/261) and [260](https://github.com/CochranResearchGroup/agent-browser/issues/260) contain the confirmed operator requirements. CONTEXT.md owns domain language. This plan owns one combined delivery outcome; it does not redefine earlier serial repair acceptance or close #183, #105 or #181.

This session is the primary writer for the replacement dashboard, handoff/tab lifecycle adapter and matching contract/docs changes. P232 integrated through PR259; its retained clean worktree and artifacts remain foreign. Do not remove or modify them. Related #68 must be reconciled for old inspector overlap; #105's wider resource ceiling/retention program remains independent. Each lane owns its catalog/index entry. Shared service/model changes require an explicit overlap readback before editing.

The primary checkout is agent-browser-dashboard. Prototype custody uses a child prototype branch in the same checkout, with a pointer and return ref; no extra checkout. Prototype code must not enter main. Source integration begins from freshly fetched main. Preserve existing opaque handoff URLs, service APIs, profile identity and authentication while retiring old operator UI code. No production/development installation, provider mutation, CI, broad cleanup or reboot effects are included.

## Delivery sequence and blocking edges

1. Confirmed model and reviewable UI: persist domain vocabulary, inspect Remote View's actual root assets, present three structurally distinct layouts within the agreed left-rail/native-viewer contract. Synthetic interactions demonstrate tabs, menus, Properties and independent link/tab lifetime controls. Capture on a throwaway branch and publish one preview session. The operator chooses a layout before production UI replacement.
2. Independent lifecycle, blocked by step 1's model: deliver configurable link expiry, tab retention and explicit deletion through the existing service authority, public contracts and one real isolated workflow. Valid-link target recreation and last-tab browser closure must pass. Establish meaningful failing regressions at the public lifecycle seam before changes. Pressure admission protects active users/operations/peers and never silently changes link validity.
3. Native workspace, blocked by the chosen layout: real browser/tab inventory and exact generation-bound selection, native tab activation and browser raise/maximize. Reuse Remote View's viewer; preserve peer state and selection under refresh. Include Share, Properties, Copy URL and allowed Close Tab with keyboard-accessible menus and shadcn dialogs.
4. Handoff management and retirement, blocked by steps 2 and 3: compact real table, human metadata, independently labeled link/tab lifetime controls and explicit link deletion. Inventory/remove obsolete operator UI while preserving software APIs and existing handoff routes. No competing legacy dashboard remains as the default operator surface.

These are proposed tracer-bullet delivery slices, not horizontal mock layers. Publish child tickets only after the operator reviews granularity and genuine blocking edges. Do not reinterpret a prototype or planning commit as feature acceptance.

## Acceptance and validation

Use the highest existing public seams: actual service HTTP/MCP/client lifecycle contracts, production dashboard assets with isolated synthetic inventory, and real disposable browser/native desktop behavior. The final workflow must prove exact native tab/window/desktop selection, peer preservation, independent configured link/tab lifetimes, deletion without tab close, target recreation after expiry and last-tab browser closure. A synthetic UI click does not prove native effects. A perpetual URL alone does not prove recovery or viewer readiness.

Controlled-clock checks cover multiple configured durations, perpetual defaults, first human use versus prefetch/agent resolution, restart persistence, expiry/deletion ordering and pressure eligibility. Owner/generation-fenced checks cover rapid selection changes, stale targets, denied input and consistent failure outputs. No raw provider URLs or private credentials appear in ordinary UI or evidence. Menus and SVG controls are accessible on desktop/mobile.

Run relevant dashboard/client parity and contract gates, documentation/link/planning audits, Rust focused suites and cargo-safe format/strict workspace Clippy for any Rust changes. Respect changed surfaces across the whole batch. Isolated development publication and installed acceptance require their own exact qualified gate. Formal release and production retirement/publication are separate.

## Current evidence and next action

Only confirmed requirements and source inspection exist for the new outcome. No replacement, lifecycle implementation or retirement is qualified. Build and publish the throwaway UI prototype, collect the operator's layout/slice feedback, then drive one complete public red/green slice at a time. Carry the full requirement inventory forward until all final gates pass.

## Resource boundary

The resumed goal's current tool accounting is active, approximately 886k tokens and 2h57m active time at admission. Preserve the user's stop-and-checkpoint bounds before 2MM tokens or 6h; refresh actual counters during work. This is not a reset or a new zero baseline. Preserve all prior checkpoints and failed evidence.
