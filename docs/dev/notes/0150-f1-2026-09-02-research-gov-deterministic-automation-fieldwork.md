# P150 Fieldwork 1 | Research.gov Deterministic Automation Discovery

Date: 2026-09-02

Status: READ-ONLY FIELDWORK CHECKPOINT

Classification: REFACTOR BEFORE KEEP

## Goal

Resume the retained Research.gov browser without a cold launch, learn which
service identities and browser evidence make automation deterministic, and
collect bounded product requirements for future Agent Browser work.

## Starting State

- Source branch: `main`
- Source commit: `74883c6c3655744f04a2ffeb7894d23ba2c75aa2`
- Fieldwork branch: `fieldwork/research-gov-deterministic-automation`
- Governing accepted lane: P150
- Durable operator handoff: `r580584`
- Expected product area: service access planning, durable handoff adoption,
  service tab handles, profile lease attestation, and provider-neutral probes

The authoritative browser and profile records remain in user-scoped runtime
state. This note records only bounded product observations. It does not copy
credentials, cookies, screenshots, provider URLs, raw browser state, or private
site data into the product repository.

## Systems Touched

- installed user-scoped Agent Browser service control plane
- retained Research.gov browser and existing Research.gov tab
- durable remote-view handoff resolution
- no-launch profile access planning and profile lookup
- provider-neutral `diagnostics`, `evaluate`, and `probe` reads
- profile lease inspection and doctor readback

No browser was launched, closed, replaced, or navigated. No page input,
authentication attempt, freshness update, profile mutation, route switch,
runtime cleanup, or tenant write occurred.

## Evidence

### Durable handoff recovery

Resolving opaque handoff `r580584` reused the retained browser and original
Research.gov target. It returned `status=ready`, a new presentation receipt,
and a valid service tab handle. Operator presentation remained ready and the
browser process was not relaunched.

This supports a deterministic resume rule:

1. resolve the durable handoff;
2. adopt the returned browser, session, target, and service tab handle;
3. do not infer those identities from URL, profile path, route occupancy, or
   process inventory.

### Caller identity is part of profile routing

An access plan using the new caller label `ResearchGovFieldwork` selected the
generic `stealthcdp-default` profile even when the Research.gov target identity
and URL were supplied. The profile lookup surface independently found
`research-gov-nsf` because it searches by target and hostname.

Reusing the original caller tuple from the handoff record changed the access
plan deterministically:

- service: `research-gov-operator`
- agent: `codex`
- task: `prepare-nsf-proposal`
- target service: `research-gov`

The resulting plan selected `research-gov-nsf`, found one compatible live
browser, returned `reuse_existing_browser`, and supplied the retained browser
and session route hints. The behavior is consistent with the profile's
`per_service` allocation and shared-service allowlist.

Future automation must preserve the canonical caller tuple as routing input.
A URL or hostname is not a sufficient resume identity for a per-service
profile.

### Read authority and input authority are distinct

Handle-bound diagnostics and evaluation succeeded against the existing tab.
The page was complete at the public Research.gov landing page and exposed a
visible `Sign In` control. Diagnostics returned no console or page errors.

The same diagnostic returned `controlPlaneAttestation.complete=false` with
`profile_lease` as the missing proof. The protected lease record for
`research-gov-nsf` is observation-only with
`blockingIdentityAxes=[legacy_principal_unproven]` and recourse
`reconcile_principal_identity`.

The fieldwork therefore stopped before navigation or page input. A ready
handoff, healthy process, valid target, and successful CDP read do not prove
effect authority when the profile lease is unproven.

### Selector normalization

A provider-neutral probe identified the page by URL and title. Its first exact
attribute selector used the browser-resolved link URL with a trailing slash:

```css
a[href="https://external.nsf.gov/"]
```

That selector did not match because the raw DOM attribute omits the slash even
though the resolved `HTMLAnchorElement.href` includes it. Both of these bounded
selectors matched the visible `Sign In` link:

```css
a[href="https://external.nsf.gov"]
a[href^="https://external.nsf.gov"]
```

A semantic read confirmed:

- raw attribute: `https://external.nsf.gov`
- resolved property: `https://external.nsf.gov/`
- text: `Sign In`
- visibility: true

Deterministic recipes should distinguish raw attribute identity from resolved
URL identity. Provider-neutral probe receipts should make that distinction
explicit when link identity is part of an automation invariant.

## Productization Candidates

### 1. Handoff-derived access intent

Add a no-launch client helper that resolves a durable handoff and derives the
canonical access-plan intent from the retained tab handle and caller trace. It
should preserve service, agent, task, target, browser, session, profile, and
target identity without asking the consumer to reconstruct them.

### 2. Planner and lookup explanation parity

When profile lookup finds a target match but access planning does not select it
because of per-service allocation, access-plan output should name the excluded profile
and the exact mismatched identity axis. This would replace a generic-profile
selection surprise with deterministic routing evidence.

### 3. Explicit read-only retained-tab mode

Expose a compact field on diagnostics and probe results that says whether the
current handle is authorized for read-only observation, browser effects, or
both. Consumers should not need to infer this from
`controlPlaneAttestation.complete` plus `missingProofs`.

### 4. Legacy lease reconciliation handoff

For a live retained browser with `legacy_principal_unproven`, provide a typed,
no-effect reconciliation plan or operator handoff. The current lease explains
the recourse but authorizes only inspection. There is no copyable bounded path
from a ready durable handoff to effect-capable principal binding.

### 5. Raw versus resolved link evidence

Extend generic link probes or UI find evidence to return both the raw `href`
attribute and the resolved URL. Deterministic recipes can then choose exact raw
matching, normalized origin matching, or semantic text matching intentionally.

## Current Gate

Browser acquisition is already satisfied through the retained browser.
Operator presentation is ready through the durable handoff. Runtime
maintenance is not the request blocker.

Automated navigation and input are blocked by the exact profile lease axis:
`legacy_principal_unproven`. Continue read-only observation only until a
separate bounded productization slice supplies effect-capable principal proof
or an explicit operator workflow reconciles that identity.

## Next Recommendation

Open one bounded productization plan for handoff-derived access intent and
legacy retained-profile authority reconciliation. Its first test should prove
that a resolved handoff can produce the exact canonical caller tuple and a
truthful read-only versus effect-capable disposition without launching or
navigating a browser.
