# Plan 0169 | Cloudflare Turnstile Desktop Challenge

Date: 2026-09-11

State: OPEN

Consolidation: required

Lane: P169

Branch: `feature/turnstile-desktop-challenge`

Target: `main`

## Objective

Add one service-owned desktop workflow that detects the visible Cloudflare
Turnstile `Verify you are human` checkbox and performs one bounded pointer
hover and left click on an exactly bound X11 browser surface.

## Current State

Desktop capture, deterministic synthetic location, guarded curved pointer
motion, controller fencing, XTEST input, effect journaling, and after-state
verification already exist. The configured locator and interaction provider
remain limited to repository fixtures. The operator-authorized Ohio Secretary
of State browser previously displayed a real Turnstile checkbox that CDP could
not reliably address across the challenge iframe. After restoring that site in
the retained stealth Chromium browser, Cloudflare cleared during the bounded
pre-click observation and the business-search form loaded. No X11 input was
emitted, so the live interaction acceptance gate remains unproven.

A second California Secretary of State fieldwork route exposed a distinct
challenge family. Imperva supplies the blocking interstitial, while a nested
hCaptcha checkbox frame supplies the visible `I am human` control and a
separate hidden challenge frame. This is discovery evidence only. The
`cloudflare-turnstile-v1` locator correctly returns `not_found`, and its
interaction recipe has no authority over this hCaptcha surface.

## hCaptcha Discovery Checkpoint

The bounded live inspection established the following reusable facts:

1. The top-level application embeds a same-origin Imperva resource frame.
2. That frame loads the hCaptcha JavaScript API and embeds a visible checkbox
   frame plus a separate offscreen challenge frame.
3. A successful hCaptcha callback supplies a token to the Imperva wrapper,
   which posts it to the wrapper endpoint before reloading the application.
4. The visible widget exposes an accessibility label identifying an hCaptcha
   security challenge, but cross-origin frame content is not a reliable CDP
   interaction surface for this workflow.
5. Whole-desktop Tesseract OCR does not preserve the visible labels exactly in
   the current frame. It recognized variants approximating `lam human` and
   `Captcha`, so an exact `I am human` text rule would produce a false negative.
6. A later operator-requested X11 probe revalidated the active window PID,
   moved through fixed coordinates, and completed the visible multi-round
   image challenge. The hCaptcha callback ran and dismissed the interstitial,
   but Imperva independently returned block error `15`. The CAPTCHA was solved;
   the protected application was not admitted. No further retry was attempted.

A reusable hCaptcha detector therefore needs a separate provider-owned profile,
not an alias for the Turnstile profile. Its fixture contract should require a
bounded checkbox geometry plus corroborating prompt, hCaptcha brand, and
Imperva context signals. It must reject generic `I am human` text, duplicate
widgets, hidden challenge frames, malformed OCR, and unsupported layouts. The
first product slice should remain observation-only. A later interaction recipe
may perform at most one checkbox click and classify `passed`, `challenge_open`,
or `inconclusive`; it must never solve an image, audio, or accessibility
challenge or retry automatically.

The [official hCaptcha developer guide](https://docs.hcaptcha.com/) publishes
test keys that return a passcode without a question and warns that they provide
no anti-bot protection. A future provider integration should use a synthetic
desktop fixture for default tests and an opt-in local HTTP fixture with those
test keys for browser integration. A production website is live acceptance
evidence only, never the deterministic test harness.

## Authority

The operator explicitly authorized this real challenge integration and the
current single-click live proof. Source changes, provider-free fixtures, a
candidate installation, deterministic observation, one controller-fenced
hover and click, and fresh verification are in scope. The authority does not
permit repeated clicks, challenge farming, fingerprint manipulation, broad
anti-bot evasion, profile replacement, browser closure, or unrelated website
actions.

## Frozen Decisions

1. Keep the workflow behind existing `desktop locate` and `desktop interact`
   ownership rather than expose raw coordinates or a general shell surface.
2. Register `cloudflare-turnstile-v1` as both the locator profile and the
   interaction recipe.
3. Detection requires one fresh bound desktop frame and exactly one normalized
   phrase `verify you are human`. The profile derives only a tightly bounded
   expected checkbox region immediately to its left. A visible checkbox may
   corroborate the initial observation, but phrase-only rendering is a typed
   `hover_required` state rather than a failure or click authority.
4. The production detector may invoke a pinned Tesseract TSV adapter with a
   bounded timeout. Raw OCR text and pixels remain response-only.
5. The X11 provider must prove the active window PID equals the service-owned
   browser PID before every input event.
6. Pointer motion uses the existing deterministic curved trajectory and pauses
   over the server-derived checkbox region. A fresh hover-state capture must
   prove checkbox geometry inside that region before one left-down and one
   left-up pair. `xdotool` may be used only as a tightly controlled X11 adapter
   with fixed arguments, never as a caller-controlled command surface.
7. Verification requires a fresh frame where the original eligible candidate
   is gone or the retained tab has left the challenge state. Disappearance is
   workflow completion, not proof that Cloudflare will accept every challenge.
8. Keep the existing browser process, `default` profile, session, route, and
   durable remote-view handoff open.

## Consolidated Batch

- Add the Turnstile locator profile, ambient OCR adapter, deterministic
  checkbox geometry, redacted receipts, and adversarial fixtures.
- Add the named interaction recipe and exact owned-window X11 probe.
- Align CLI, MCP, service contracts, generated client surfaces, help, skill,
  README, and docs.
- Qualify provider-free behavior, install one candidate, and exercise exactly
  one live click against the current Ohio SOS challenge.

## Delivery Sequence And Budget

1. Freeze detector and interaction fixtures before implementation.
2. Implement observation and provider integration with focused tests.
3. Run formatting, strict Clippy, selected contract/docs checks, and patch
   hygiene once for the completed source batch.
4. Build and install one candidate generation.
5. Observe once, acquire exact controller authority, interact once, and verify
   once. Stop on ambiguity, stale geometry, ownership mismatch, failed input,
   or inconclusive after-state.

Maximum implementation attempts: 2. Maximum live clicks: 1. No automatic
retry is permitted.

## Worker Assignments

The primary agent owns the critical path, all source writes, provider-free
validation, installation, live authority reconciliation, the single live
effect, and closeout. No subagent has live-system authority.

## Acceptance Criteria

1. Visible-checkbox and hover-revealed Turnstile fixtures produce one stable
   bounded candidate; phrase-only evidence requires hover verification.
2. Missing phrase, decoy geometry, duplicate candidates, stale capture,
   malformed OCR, and OCR timeout fail closed without input.
3. The action accepts no caller pixels, OCR text, template, display, PID,
   coordinates, executable, motion tuning, or retry count.
4. Every input event revalidates controller authority, route/display geometry,
   provider generation, and active-window browser PID.
5. Exactly one hover-and-click recipe is emitted and replay emits no new
   effects.
6. Fresh after-state evidence returns `passed`, `failed`, or `inconclusive` and
   never treats input acknowledgement alone as success.
7. Durable projections contain no pixels, OCR transcript, page text, IP
   address, provider stderr, local paths, or command line.
8. All documented public surfaces and focused tests agree on the new IDs.
9. The installed candidate detects the current challenge, emits at most one
   click, and keeps the browser and durable handoff open.

## Evidence And Exit

Record source identity, focused and selected validation, installed generation,
 observation receipt, interaction receipt, fresh title or visual after-state,
 browser PID, route readiness, and zero-repeat disposition. Close only when
all criteria are proven or mark the plan blocked with the exact unmet gate.

## Non-Goals

- Solving image, audio, managed, or interactive CAPTCHAs
- Treating the discovered hCaptcha widget as a Turnstile target
- Retrying after the post-hCaptcha Imperva block
- Spoofing browser fingerprints or hiding automation
- Caller-defined OCR, coordinates, commands, or arbitrary desktop input
- General challenge loops or unattended retries
- Closing, replacing, or creating a browser profile
