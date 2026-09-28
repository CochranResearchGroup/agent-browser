# Plan 0220 | Remote View Consumer Integration

Date: 2026-09-28

Plan version: 1

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P220

Predecessor: Agent Browser Issue #195 and the linked P219 supersession audit, cancelled as superseded while incomplete

Work item: [CochranResearchGroup/agent-browser#202](https://github.com/CochranResearchGroup/agent-browser/issues/202)

Consumer dependency: [CochranResearchGroup/remote-view#70](https://github.com/CochranResearchGroup/remote-view/issues/70)

Target: `origin/main` at `a3848e16`

Branch: `platform/p220-remote-view-consumer`

Owner: primary P220 implementation owner

Authority: the operator directed creation and execution preparation on 2026-09-28. This plan authorizes repository planning, provider-free fixtures, and ordinary source implementation after branch admission. Installed, privileged, public-ingress, production, merge, release, destructive cleanup, and Remote View runtime effects remain separately gated.

## Objective

Make Agent Browser a consumer of Remote View desktop and presentation
contracts in the trusted single-user WSL proof of concept. An ordinary request
selects or acquires a fixed Remote View desktop, launches or recovers the
addressed browser there, and returns only the durable opaque operator handoff.

Agent Browser continues to own browser profiles, browser processes, CDP,
logical sessions, tabs, targets, browser-to-desktop affinity, activity,
recovery, and final-session cleanup. Remote View owns desktop identity,
lifecycle, health, viewing routes, viewer admission, presentation, OS-level
input, diagnostics, and exact desktop cleanup.

## Current State

Remote View Plan 0017 has integrated its F0 foundation and an early
provider-free Agent Browser contract probe. That probe uses two existing fixed
desktops and will replay at J1, J2, and J3. It is compatibility feedback rather
than final RV-014 acceptance.

Agent Browser Plan 0219 is cancelled as superseded while incomplete after its
authorized final cold-install attempt failed. Its branch is 36 commits ahead of
the published topic ref and contains both reusable Agent Browser domain work
and retired presentation-specific work. The successor branch was admitted from
`origin/main` at `a3848e16`; P219 custody remains preserved and is not merged
wholesale.

## Product boundary

Agent Browser owns:

- profile, process, CDP, logical-session, tab, and target state;
- selection of the browser-to-desktop association;
- navigation, browser recovery, and addressed-session cleanup; and
- browser readiness and CDP diagnostics.

Remote View owns:

- fixed or bounded desktop selection and allocation;
- desktop generation, readiness, presentation, and OS-level input;
- stable opaque viewing routes and viewer connectivity;
- generic allowlisted application-launch execution when used; and
- exact desktop release, cleanup, and presentation diagnostics.

Neither product imports the other product's private state authority. The
initial operating model is one trusted privileged user in one WSL instance.

## Consolidated batch

The first delivery batch contains four related outcomes:

1. Classify every unpublished P219 commit and changed surface as retain, adapt,
   retire, or evidence-only.
2. Freeze a provider-neutral Agent Browser presentation-client boundary against
   the current Remote View F0 identities and observations.
3. Prove two fixed desktops can carry distinct Agent Browser-owned browser
   associations without moving process, profile, CDP, or session authority.
4. Produce an integration candidate that removes dependency on the retired
   Agent Browser-owned XRDP/Guacamole installation path without activating a
   live Remote View runtime.

Dynamic capacity, iframe embedding, generalized IAM, cross-principal policy,
multi-tenant isolation, and a formal release are deferred.

## Delivery sequence and budget

### S0 — Custody and semantic audit

Inventory the 36 unpublished commits, changed files, schemas, tests, docs, and
runtime assumptions. Record exact retain, adapt, retire, and evidence-only
groups. Select an integration base without rewriting or deleting P219 custody.

Exit: every changed surface has one disposition and conflicting mixed commits
have an explicit extraction strategy.

### S1 — Provider-free consumer contract

Add an Agent Browser-owned Remote View client boundary and fixtures for public
identity, observation, desktop selection, opaque handoff, status, and release
shapes. Use no Remote View internal modules and perform no runtime effects.

Exit: the F0-shaped fixture proves two fixed desktop selections and distinct
browser associations while Agent Browser retains its domain authority.

### S2 — Retained-domain integration

Integrate only the P219 browser/session/runtime changes selected by S0. Adapt
presentation calls behind the new client boundary. Exclude or remove the
retired XRDP user, Guacamole provider-rebuild, route-pool, and
presentation-helper path from the successor candidate.

Exit: changed-surface tests pass and architecture checks reject reintroduction
of the retired presentation ownership.

### S3 — Checkpoint replay and final acceptance preparation

Replay the consumer fixture against Remote View J1, J2, and J3 contracts as
they become available. Extend it for generation-safe viewing, diagnostics,
many-to-many observation, and exact cleanup without importing Remote View
internals.

Exit: provider-free final acceptance is source-bound and the separately gated
installed acceptance packet has exact targets, stop rules, and rollback or
forward-recovery boundaries.

No installed acceptance begins merely because S0 through S3 source work passes.

## Worker assignments

One primary P220 owner controls the integration base, shared client contract,
and final reconciliation. Read-only commit classification and provider-free
fixture design are parallelizable after branch admission because they write
separate audit and test surfaces. Client-contract implementation, extraction
of mixed P219 commits, schema generation, and shared documentation remain on
the serialized critical path.

Remote View retains authority over its repository and Issue #70. Agent Browser
consumes published checkpoints and does not edit Remote View source as part of
this plan.

## Evidence and exit

| Axis | Required evidence | Invalidation |
| --- | --- | --- |
| Custody | Exact P219 commit and file disposition with preserved branch and artifacts | Missing or silently discarded unpublished work |
| Boundary | Tests prove Agent Browser and Remote View retain the ownership split above | Either product becomes authoritative for the other's private state |
| Contract | Provider-free fixtures consume only published Remote View contracts | Importing Remote View internals or inventing a second control plane |
| Browser behavior | Two desktop associations retain distinct browser/process/profile/CDP identities and addressed sessions | Duplicate association, profile substitution, or Remote View browser ownership |
| Presentation | Only opaque handoffs leave the boundary; readiness and failures remain layered | Raw provider URLs, credentials, or fabricated viewer readiness |
| Cleanup | Release targets exact associations and ambiguity fails visibly | Cleanup by slot, PID, window title, or guessed ownership |
| Resources | Fresh process/resource census after separately authorized installed acceptance | Unexplained browser, desktop, daemon, listener, unit, or container residue |

## Acceptance criteria

- The P219 audit assigns every unpublished commit and changed file one durable
  disposition.
- Agent Browser uses a versioned provider-neutral consumer boundary for Remote
  View operations and observations.
- Two fixed desktops support distinct Agent Browser-owned browsers with no
  transfer of profile, process, CDP, session, or recovery authority.
- The same opaque handoff reconnects to the current desktop generation without
  exposing provider credentials or raw Guacamole routes.
- Diagnostics independently report desktop, browser/CDP, transport, viewer,
  and application readiness.
- Provider-free probe replays pass at applicable Remote View F0/J1/J2/J3
  checkpoints; each result is compatibility evidence at that checkpoint only.
- Exact release and cleanup leave no unexplained owned resources, while
  ambiguous ownership fails without broad deletion.
- Required Rust, generated-client, schema, documentation, architecture,
  planning, and lane gates pass for every touched surface.

## Non-goals

- Repairing or accepting the P219 Agent Browser-owned XRDP/Guacamole cold
  installation.
- Moving browser lifecycle or browser-private state into Remote View.
- Implementing Remote View dynamic capacity or embedding from Agent Browser.
- Adding generalized multi-user security to the present single-user proof of
  concept.
- Treating a provider-free test as installed, public, production, or release
  authority.

## Hard stops

- Keep the admitted successor worktree bound to the audited `origin/main`
  baseline and preserve the separate P219 custody branch.
- Do not merge the 36-commit P219 branch wholesale.
- Do not delete, rewrite, or clean the P219 branch, failed overlays, serial
  logs, or receipts without an explicit custody disposition.
- Do not repair the retired XRDP/Guacamole presentation path under this plan.
- Stop on ownership ambiguity, raw provider credential exposure, contract drift
  without an explicit checkpoint decision, or any required live effect without
  exact authority.

## Definition of done

Plan 0220 is complete when the selected presentation-neutral P219 work is
integrated through protected review; Agent Browser consumes Remote View public
contracts through one provider-neutral boundary; provider-free F0/J1/J2/J3
evidence and final installed acceptance are source-bound and truthful; the
Alice/Bob browser/session behavior survives the presentation replacement; the
retired XRDP/Guacamole ownership does not remain on the supported path; exact
cleanup and a fresh resource census pass; and roadmap, runbook, plan, work item,
lane, Git, validation, and installed identities agree.
