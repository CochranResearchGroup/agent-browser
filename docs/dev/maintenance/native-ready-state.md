# Native public ready-state compatibility

Product lane: PL-BUGFIX
Work item: 575
Branch: fix/native-ready-state
Base: 371acd19e50afaaf3364a5203d92ae31b3416aa7
State: ACTIVE

Reuse the existing P321 owning checkout and preserve the paused interview patch
privately. Restore guarded Native input for schema-1 public desktop inspection
whose exact lifecycle target is `ready` with enabled selective-input policy.
The installed client accepts only `local_ready`, incorrectly rejecting the
provider public lifecycle spelling before any guarded input request.

Own `cli/src/native/browser_native_readiness.rs` and adjacent existing readiness
guidance. PR573 owns human-handoff behavior; shared documentation edits are
limited to separate paragraphs and integration must reconcile that overlap.
Mailroot/T0417 retains the existing conversion and all provider inputs. This
lane creates no second conversion, close, input, reset or mail runtime change.

Prove exact HTTP-fixture RED for public `ready` then GREEN for public and legacy
ready spellings while retaining changed identity/schema, disabled/unknown policy
and non-ready-state refusals. Run affected Native tests, format and strict
workspace Clippy, documentation/policy checks locally. Do not use hosted CI.
Source qualification, preserving installation, original conversion continuation,
Native input authority and Google account acceptance remain distinct gates.
