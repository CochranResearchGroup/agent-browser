# Plan 0181 | hCaptcha Fixture Checkbox Acceptance

Date: 2026-09-13

State: ACTIVE

Consolidation: required

Product lane: PL-CHALLENGE

Lane: P169

Work item: `CochranResearchGroup/agent-browser#66`

Branch: `feature/turnstile-desktop-challenge`

Target: `main`

Integration: merge through the protected `main` workflow

## Objective

Add a distinct service-owned hCaptcha checkbox locator and one-click desktop
interaction recipe, qualify it with deterministic provider-free fixtures, and
exercise exactly one click against the repository CAPTCHA lab using the
operator-provided non-production hCaptcha credentials. Stop without solving or
retrying if an image, audio, or accessibility challenge opens.

## Current State

Plan 0169 owns the existing `cloudflare-turnstile-v1` locator and guarded X11
recipe. Its discovery checkpoint correctly records that an hCaptcha checkbox
is a separate challenge family and must not be treated as Turnstile. Plan 0180
froze the provider-neutral CAPTCHA guard contracts without adding another
locator or solver.

The repository CAPTCHA lab supplies a local hCaptcha application and keeps its
secret server-side. The operator supplied a custom non-production sitekey and
secret in `~/credentials/API-keys.env`. The lab launcher reads only the named
hCaptcha values from that operator-managed file without copying or printing
them. They must not enter source, logs, receipts, or durable runtime state.

Development presentation-provider apply reached a no-effect quarantine before
the fixture run. Its exact Service trace showed that route bootstrap opened an
owned `about:blank` tab, then attempted a second navigation without carrying
the service-owned tab binding. The current repair applies the Guacamole header
and destination URL on the first owned `open` request.

The single post-diagnosis apply was consumed on 2026-09-14 and quarantined
because development Guacamole web readiness timed out. The receipt is
`/home/ecochran76/.local/share/agent-browser-dev/presentation-provider/receipts/apply-1789383547444-268695.json`.
Quarantine cleanup left all three development provider containers stopped and
their ports closed. The development runtime itself remains ready on installed
generation `0.28.0-33ceedaf302b`, but provider-required doctor is not ready.
The fixture was not started and no checkbox input was emitted.

## Authority And Effect Boundary

The operator authorized repository feature work, use of the repository test
fixture, the non-production hCaptcha credentials, development-provider repair,
and one checkbox interaction. Source and documentation changes, focused tests,
one development candidate build and install, one repaired provider apply, one
fixture browser launch, observation, controller acquisition, one hover and one
left-button click are in scope.

No production runtime, protected third-party site, profile replacement,
credential persistence, image selection, audio interaction, accessibility
challenge interaction, second click, reset, automatic retry, token farming,
fingerprint manipulation, or release is authorized.

## Consolidated Batch

- Preserve the no-effect provider quarantine receipt and qualify the exact
  first-navigation route repair.
- Add `hcaptcha-checkbox-v1` as a separate locator and interaction recipe.
- Require a bounded checkbox geometry corroborated by both the visible human
  prompt and hCaptcha brand evidence; reject prompt-only, duplicate, hidden,
  malformed, stale, and unsupported observations.
- Reuse the existing controller, process, display, effect-journal, replay, and
  XTEST fences. Emit pointer motion plus exactly one left-down and left-up pair,
  with no keyboard events.
- Preserve a typed post-click state. `passed` means the checkbox is gone;
  `challenge_open` or `inconclusive` is terminal and authorizes no further
  input.
- Align CLI, HTTP/MCP request metadata, generated client types, provider
  admission, help, README, agent skill, docs site, and inline documentation.

## Delivery Sequence And Budget

1. Freeze the separate locator and one-click recipe behavior in focused tests.
2. Implement the smallest end-to-end product path and update public surfaces.
3. Run changed-surface validation, formatting, strict Clippy, and patch hygiene.
4. Build and install one development candidate.
5. Restage, preflight, and apply the repaired presentation provider once.
6. Start the lab from the `/tmp` credential files, observe once, acquire exact
   controller authority, click once, and record the fresh terminal state.

Maximum implementation attempts: 2. Maximum provider apply attempts after the
recorded diagnosis: 1. Maximum candidate builds: 1. Maximum fixture checkbox
clicks: 1. Maximum challenge-solving actions: 0. Maximum live retries: 0.
Overall active-work ceiling: 90 minutes from this successor packet. Reassess
after two checkpoints or 30 minutes without outcome progress.

## Acceptance Criteria

1. The hCaptcha profile is a distinct registered ID and cannot alias the
   Turnstile detector or receipt identity.
2. Deterministic fixtures produce one stable checkbox candidate only when the
   prompt, hCaptcha brand, geometry, and visible control agree.
3. Missing brand, generic human text, hidden control, duplicate widgets,
   malformed OCR, stale capture, and unsupported layouts fail closed without
   input.
4. The public action accepts no caller pixels, OCR text, coordinates,
   executable, display, process ID, motion tuning, credential, or retry count.
5. Every event revalidates exact controller authority, route and display
   geometry, admitted provider generation, and active browser process identity.
6. The recipe emits no keyboard events and at most one left-down and one
   left-up pair. Replay emits no new effects.
7. Post-click evidence reports `passed`, `challenge_open`, or `inconclusive`;
   the latter two stop without challenge-solving input or retry.
8. Durable projections contain no pixels, OCR transcript, page text,
   credentials, tokens, IP addresses, local paths, commands, or provider
   stderr.
9. The repaired development provider reaches ready state with non-null
   presentation capacity before the fixture attempt.
10. The installed candidate performs at most one click against the custom
    local hCaptcha fixture and preserves the browser and handoff afterward.

## Evidence And Exit

Record the source commit, focused and selected validation, candidate and
installed generation, preserved quarantine receipt, repaired provider receipt,
observation receipt, controller identity, interaction receipt, fresh terminal
state, and zero-repeat disposition. Close only when all criteria are proven.
If a visual, audio, or accessibility challenge opens, record
`challenge_open`, stop input immediately, and treat that as successful bounded
interaction evidence rather than CAPTCHA completion.

## 2026-09-14 Blocked Checkpoint

- The qualified source packet is commit
  `1d71ed39aa19fa6273e2b80f184d9c2d68a474ea` on
  `feature/turnstile-desktop-challenge`.
- Source qualification passed: Rust format, strict workspace Clippy, focused
  hCaptcha, locator, interaction, and service-contract tests, 158 workstation
  installer tests, route-confusion gates, service collections no-launch, and
  the source-free workstation install fixture.
- The single optimized candidate build passed and installed development
  generation `0.28.0-33ceedaf302b`; the installer reported production
  unchanged and the development skill synchronized successfully.
- Provider plan, stage, and preflight passed with the reviewed external-ingress
  binding and exact six-route identity. The single authorized repaired apply
  then quarantined on development Guacamole web readiness timeout.
- Read-only doctor confirmed the candidate runtime and protected lease
  authority are ready, while the required presentation provider is not ready.
  No presentation capacity exists.
- Per the zero-retry boundary, no second apply, fixture launch, controller
  acquisition, pointer movement, or checkbox click is authorized in this
  packet. Resume requires a successor diagnosis and a new explicit effect
  budget.

## 2026-09-14 Successor Provider Diagnosis

The operator resumed the lane after the blocked checkpoint. This successor may
inspect the retained quarantine evidence, make at most one source-backed
provider repair, run provider-free validation, and perform at most one new
development-provider apply. It must not rebuild or reinstall the already
qualified candidate unless diagnosis proves a candidate source defect that
cannot be repaired in the provider bundle alone.

If the provider becomes ready with non-null presentation capacity, the original
one-click fixture boundary resumes. Credentials must be loaded directly from
the operator-managed private environment file without copying, printing, or
persisting their values. A second provider quarantine, missing capacity, or any
visual, audio, or accessibility challenge is an immediate stop. The cumulative
goal ceiling remains unchanged; reassess after 30 minutes or two checkpoints
without acceptance progress.

The retained container log identified the quarantine cause: Guacamole could
not read the staged defaults extension because the JAR was mode `0600` under a
hardened process umask. The atomic writer requested `0644` but did not enforce
the final mode after creation. The repair applies the requested mode explicitly,
and the provider fixture now stages under umask `0077` to cover this failure.
Provider fixture and development-runtime fixture validation pass; fresh staging
produces a mode `0644` JAR and provider preflight passes.

The successor apply then completed as `provider_ready_ingress_pending` with
production unchanged. Its receipt is
`/home/ecochran76/.local/share/agent-browser-dev/presentation-provider/receipts/apply-1789390799829-704539.json`.
Provider-required doctor passes. Development Service Status reports three
`warm_idle` presentation slots, no binding warnings, and a configured hard
maximum of six. The operator-supplied credential path did not exist at the
literal location provided; no alternative credential copy was used and the
fixture remains unstarted with zero input emitted.

## 2026-09-14 Live Fixture Hard Stop

The operator corrected the credential locator to
`~/credentials/API-keys.env`. The repository lab read the two hCaptcha values
directly from that file and served the fixture at the local approved origin.
No credential value entered a command argument, source file, receipt, or plan.

The first installed observation returned `not_found` even though a fresh
desktop capture visibly contained one unchecked widget. Read-only Tesseract
evidence showed two bounded mismatches with the source-only profile: the real
widget rendered a 30-pixel checkbox with a one-pixel border, and OCR returned
`Tam human` plus `hhCaptcha`. Two test-first implementation tracers reproduced
those mismatches. Commit `8d5b3bb6` updates profile `p181-v2` to accept only
those observed OCR variants and exact geometry while retaining prompt, brand,
visibility, score, and duplicate-widget guards. All seven focused hCaptcha
tests, Rust formatting, and strict workspace Clippy pass.

The final optimized development generation is
`0.28.0-b80054511bdc`, SHA-256
`b80054511bdcd71c35202714b8e80bdad679c729200416ceb1041f0137aef922`.
Installation reported the development runtime and presentation provider ready
and production unchanged. A stale pre-install browser identity caused one
fully compensated `presentation_bound_slot_missing` launch failure in job
`r103018`. A fresh one-time identity then opened successfully in job `r525024`
on `development-route-2`, `development-display-2`, with browser
`session:p181-hcaptcha-final`, profile
`managed-ephemeral-p181-hcaptcha-final`, target
`65A6718816889DFD44EEF5A48825475C`, and operator-visible state `ready`.
The preserved durable operator handoff is
`https://agent-browser-dev.ecochran.dyndns.org/remote-view/r525024`.

Installed observation job `r641899` matched exactly one candidate,
`desktop-candidate-aab43ebdbae9c925e34dfedd`, at `(294,331)` with size
`30x30`, score `9498`, and prompt, hCaptcha brand, visible-checkbox, route,
display, profile, and fresh-frame agreement. Controller request
`http-service-request-service_controller_lease_takeover-2501f60b-edf4-4673-99da-59645390c339`
then established controller epoch `1` and exact lease
`viewer:development-route-2:codex-p181-hcaptcha:2026-09-14T13-44-42-88320314Z`.

The sole interaction call, job `r858591`, stopped at
`desktop_interaction_authority_required` because the lease viewer identity
`codex-p181-hcaptcha` did not equal the action agent identity `codex`. Source
ordering and the existing authority regression prove this rejection occurs
before input events. The fresh terminal desktop capture also shows the checkbox
still unchecked and no visual, audio, or accessibility challenge. Therefore
the measured fixture result is zero pointer events and zero clicks, despite the
generic Service wrapper conservatively reporting `effect_uncertain`.

The plan is blocked at its no-retry hard stop. The final browser, fixture, route,
controller record, and durable handoff remain open. Resumption requires a new
explicit interaction attempt budget and a fresh controller lease whose
`viewerId` exactly matches the interaction `agentName`. No solver, reset,
second click, image selection, audio action, accessibility action, production
mutation, or release is authorized.

## 2026-09-14 Effect Classification And Acceptance Resumption

The operator explicitly resumed the plan after reviewing the distinction
between the test-orchestration identity mismatch and the product's generic
failure classification. The zero-input authority rejection remains valid
evidence and is not retried or erased. This successor adds one new interaction
attempt after repairing the product's failure projection.

The consolidated implementation batch now includes an exact Service recourse
mapping for `desktop_interaction_authority_required`. Because the native guard
returns before any input event, the public failure must retain the native code
and report `effectState=no_effect`, `phase=child_admission`, and an
inspect-before-retry disposition with controller-authority recourse. It must
not fall through to `service_operation_failed` and `effect_uncertain`.

The critical path is serialized:

1. Add one public-interface regression that demonstrates the current generic
   classification, then implement the smallest exact classifier branch.
2. Synchronize the required CLI help, README, agent skill, docs site, and inline
   documentation for the corrected recourse behavior.
3. Run focused classifier and Service-envelope checks, selected changed-surface
   validation, Rust formatting, and strict workspace Clippy.
4. Build and install at most one additional optimized development candidate,
   verify its exact generation and that production remains unchanged, then run
   development runtime and provider readiness checks.
5. If the preserved fixture and browser remain ready, acquire one fresh
   controller lease and submit one desktop interaction with both `viewerId`
   and `agentName` set to `codex-p181-hcaptcha`.
6. Capture the terminal state and stop. A passed checkbox, an image, audio, or
   accessibility challenge, an inconclusive result, readiness loss, or any
   authority failure is terminal for this successor.

Successor bounds are one classifier implementation attempt, one completed
candidate build and development installation, one controller takeover, and one
desktop interaction call. No additional provider apply, widget reset, second
click, challenge-solving input, production mutation, or release is authorized.
The successor active-work ceiling is 90 minutes. Reassess after two checkpoints
or 30 minutes without outcome progress. The primary agent owns the critical
path; no worker assignment is needed for this tightly coupled repair and live
acceptance sequence.

Acceptance requires both axes to remain separate: provider-free checks prove
the corrected no-effect recourse, while the installed fixture attempt proves
the original one-click hCaptcha behavior. The classifier repair does not itself
authorize a retry, and a successful interaction does not replace the required
source and Service-envelope validation.

## Non-Goals

- Solving image, audio, accessibility, or multi-round challenges
- Treating hCaptcha as Cloudflare Turnstile
- Retrying or resetting the widget
- Testing a production or protected third-party site
- Persisting or publishing the supplied credentials
- General CAPTCHA loops, identity rotation, or anti-detection behavior
- Formal release or production installation
