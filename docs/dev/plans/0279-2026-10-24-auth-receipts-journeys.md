# P279: Governed Google and Microsoft sign-in journeys

Status: OPEN
Product lane: PL-AUTH
Work item: CochranResearchGroup/auth-receipts#81, CochranResearchGroup/auth-receipts#79
Branch: auth/receipts-journeys
Owner: ecochran76
Target: main
Integration: source only; no installed skill publication

## Current State

Provider sign-ins succeeded in the Auth evaluation, but isolated temporary
authorities bypassed installed approval/notification. Auth's source repairs and
synthetic hosted tests do not qualify installed native receiving. The primary
checkout was clean at integrated40900dec4; issue365 is CLOSED. It is reused from
origin/main cc981b2be. The previous branch is retained; both peer checkouts and
all ignored candidates are preserved. No checkout was created or removed.
The canonical catalog contains older stale ACTIVE_WORKTREE claims; this task
adds only its own assignment and does not infer authority to clean foreign work.

## Bounded outcome

Primary owns the existing authentication skill/reference guidance and pointers.
Record the verified Google headed/native and Microsoft password-first journeys,
with clear installed-authority gates and explicit source/runtime evidence limits.
The result should stop agents recreating private live authorities while keeping
profile reuse, handoff and provider progression easy to follow. No new public
commands, authority model, provider solver or browser runtime is introduced.

## Validation and terminal condition

Check changed Markdown/MDX links, the existing handoff documentation gate and
relevant client login fixtures. Review Standards and Spec separately. Preserve
provider qualification separately from installed workflow qualification. Source
remains uncommitted until requested; no installed skill refresh, provider login,
Slack send, real credential access, host restart or profile mutation is included.

## Source review and validation

Standards: the reference is reached by explicit login/helper/number-matching/
saved-profile branches from the existing skill. Reusable procedure stays with
Agent Browser; credential authority stays with Auth and channels with parent
Receipts. No account identifiers, private profile paths, passwords or device
codes were copied into Git. Existing generic examples point to the governing
provider reference. No new command/flag or runtime implementation was introduced.

Spec: Google headed/native, Microsoft work/school password-first, explicit field
focus, hardware-key modal dismissal, owner factor choice, watcher readiness,
number matching, profile continuity, current control ownership and exact account
verification are retained. The guide explicitly rejects replacing the installed
authority and distinguishes appropriate authority holds from provider-contract
defects. Historical provider success is not promoted to integrated acceptance.
Installed native receiving, deterministic real delivery and acceptance stay open
in Auth #81/#79. These documentation changes do not satisfy those runtime gates.

Validation: all seven added relative Markdown links resolve; git diff --check
passes. Existing handoff documentation, login playbook restart/exact binding and
hosted version2 identity/readback/no-replay fixtures pass. pnpm checked existing
workspace dependencies and its source postinstall reported the local native
binary ready; it did not launch a browser or publish a runtime. Source changes
remain uncommitted. No installed/shared skill sync was run.


## Bounded source integration revision

The active goal requires a supported browser client path, beyond guidance alone.
This revision adds `followLogin` to the existing hosted Node adapter and its types,
delegating the existing private attempt to Auth's supported follow SDK. The primary
owns the adapter, synthetic contract script and required documentation surfaces.
No new CLI operation, provider solver, owner authority, browser receiver or installed
change is included. Private request creation/use integration remains open.

Standards: installed verification and private transport remain producer-owned;
callers cannot supply a replacement client. Missing SDK/shared-owner capability
fails explicitly rather than using legacy authentication. Auth retains authority
and parent Receipts retains delivery. Existing legacy discovery remains usable.

Spec: delayed MFA follow-up is now reachable through the source browser adapter.
This is synthetic adapter qualification, not installed owner/native/Slack acceptance.
The existing request/use playbook still uses its legacy contract and remains fenced
under shared-owner composition; do not call the full journey repaired yet.

Validation: `pnpm exec node scripts/test-hosted-credential-follow.js` first failed
with followLogin absent, then passed with supported delegation, replacement-client
rejection, incompatible authority and missing SDK coverage. Existing installed
producer SDK discovery and both plain/hosted login playbook fixtures pass. The
installed-producer check used a synthetic isolated host and read no vault credential.


## Final adapter review and qualification

Standards: accepted blocking diagnostic finding fixed. A capability authorization
failure was initially relabelled as migration. A synthetic regression first failed
with auth_authority_migration_required instead of auth_not_authorized. Only an
unknown operation now maps to missing support; existing authority/transport errors
retain their original codes. This preserves the distinction between a legitimate
authority hold and an incompatible installed contract.

Spec: accepted blocking end-to-end gaps remain request/use playbook migration,
installed owner/native enrollment and actual Google/Microsoft/Slack acceptance.
The source adapter's original-attempt follow-up is qualified; no broader journey
completion or deployment is claimed. No extra public CLI operation was introduced.

Focused validation: wired `pnpm test:hosted-auth-helper`, plain/hosted login fixtures,
JS type checking, MDX compile and handoff documentation checks pass. The full
`pnpm test:service-client` no-launch suite passes, as does Cargo formatting.
The broad selector recommends workstation/provider and Rust compilation gates from
path matches; this slice changes only a Rust help string, the Node adapter/types
and documentation. No workstation/provider implementation changed. No full docs
build, Clippy, live browser test or shared skill sync was run; this is a source
checkpoint rather than merge/deployment qualification. CI was not run.


## Owner request/use source integration

The same bounded path now adds requestLogin (private login only) and advanceLogin
(original-request approval polling and first private use). Shared-owner preflight
is common to all three methods and also requires the supported follow SDK before
any new login. Strict caller fields cannot override mode, operation or client.
Unknown attempts do not re-enter use. The legacy playbook remains for legacy hosts;
its labels are not silently translated into owner credential/receiving enrollment.

A pinned actual-source composition in Auth's opt-in test:browser-owner-source
exercises this adapter, Auth SDK, private discovery, hosted IPC and one owner
service. Synthetic approval/vault/browser/notification ports prove pending approval
and queued notifications produce zero reads, attributable Allow Once plus delivered
notification permits one input, repeated advancement does not replay, follow-up
reconciles the original attempt, and idempotency retains one owner request.
Candidate-verification metadata is synthetic; this is not installed acceptance.

Standards review: no parallel authority, caller policy, raw release, private path
or client override. Common preflight preserves genuine authorization errors.
Spec review: source request/use/follow is reachable through the supported adapter;
actual installed adoption, native receiver qualification, reverse-device input and
real Google/Microsoft/Slack acceptance remain full-issue blockers.

Owner-path validation checkpoint: Auth check/default suite passes (502 passed,
2 explicit skips), plus the separate pinned browser-owner source composition.
Agent hosted helper/discovery and type checks pass, alongside retained plain and
hosted login fixture coverage, MDX compile, handoff docs and Cargo formatting.
Changed-file secret scans: Agent Browser 17 files and Auth 48 files, zero findings.
All source remains uncommitted; this checkpoint authorizes no installed effects.


## Verified Native receiving guidance

Auth's actual isolated Native/stock Chrome qualification now proves one synthetic
source read and one matching HTTPS form submission, with safe fixture retirement.
The existing provider reference now carries the verified full-URL requirement,
separate click/focused-field/private-input sequence, actual dispatched receipt
meaning, conservative AT-SPI account-label normalization and original-request
inspection after an observation timeout. The failed combined batch and successful
readiness handshake are retained in Auth's native qualification note. No real
provider or installed approval acceptance is inferred; this revision changes
reference guidance only and does not refresh installed/shared skills.


Reference revision review: Standards keeps procedure in the existing disclosed
reference and leaves profile/account bindings outside Git. Spec distinguishes
actual synthetic receiving from installed approval and real MFA delivery. No
new runtime authority, manual secret path or unconditional retry was introduced.
Validation: `pnpm test:remote-view-handoff-docs` and `pnpm --dir docs build` passed.
The build changes no tracked generated files. Installed/shared skill publication
and the Auth runtime acceptance gates remain pending.
