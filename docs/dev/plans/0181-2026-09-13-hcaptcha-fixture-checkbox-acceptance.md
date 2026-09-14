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
secret in `/tmp`; those files are untracked, mode `0600`, and may be read only
to start this bounded lab run. They must not enter source, logs, receipts, or
durable runtime state.

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

## Non-Goals

- Solving image, audio, accessibility, or multi-round challenges
- Treating hCaptcha as Cloudflare Turnstile
- Retrying or resetting the widget
- Testing a production or protected third-party site
- Persisting or publishing the supplied credentials
- General CAPTCHA loops, identity rotation, or anti-detection behavior
- Formal release or production installation
