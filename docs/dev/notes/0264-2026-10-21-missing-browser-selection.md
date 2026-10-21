# Missing explicit browser evidence

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: P264 / #344
Related lanes: #68, P260, P261

## Reproduction and repair

The context regression fails on canonical 59021fe0f: an absent explicit browser
falls through to a live peer selected by weaker URL hints. The inspector can
therefore describe the wrong workspace. Direct and browser-workspace IDs now
resolve exactly or return missing. A missing node cannot supply unrelated page
or daemon evidence. Profile-only navigation and live browser selection remain
covered by the existing fixture.

Ten regression combinations cover both browser-ID forms against independent
profile, service-session, daemon-session, tab and job hints. They require missing
state, null browser/page/daemon/stream, no actions and no resources. The initial
combined-hint regression was red before repair; the expanded matrix is green.

## Validation and limits

Context, contextual chat, view projection and inspector action checks pass.
Dashboard TypeScript and build, policy wiring, documentation links and patch
hygiene qualify the bounded source repair. The initial planning audit rejected
the unrecognized IN_PROGRESS plan-header spelling; final CLOSED metadata is
rechecked. No Rust changed or Cargo gate claimed.

Standards review keeps the change within existing pure context projection. Spec
review verifies that explicit browser identity cannot silently retarget evidence
while weaker-only navigation stays supported. #68 remains open for its broader
inspector outcomes. No native viewer, authentication, polling, production runtime,
profile or provider effect occurs; this is source acceptance, not deployment.
