# Ordinary denial isolation

Product lane: PL-BUGFIX
Disposition: active-input
Owning plan or work item: P244, #288
Related lanes: P237

## Contract and source

Alice/Bob G05/G23/G42 and policy 0054 govern ordinary browser availability. The baseline is canonical a8b3f1a8233d6ed0ce9488543a16fa3de44b84e9 after PR287. This follow-up repairs independently reproduced historical metadata vetoes; it does not assert an exhaustive audit of every error path.

## Reproduction

The installed public synthetic malformed-custody fixture rejects concurrent continuation with browser_session_publication_release_fenced. Its exact fixture-process census is zero afterward. Actual SQLite regressions independently reproduce malformed launch history and an unchanged peer desktop release fence rejecting ordinary task publication. A current shared-local permission fixture reproduces existing_session_profile_identity_ambiguous before current permission can be evaluated. Native startup independently reproduces remote_view_session_custody_unavailable on the same malformed history. A completed-release fixture reproduces the historical launch-publication prerequisite for an otherwise current browser binding.

An initial owner fixture failed to compile because its JSON map consumed string keys; that test-construction failure is retained separately and was corrected before the asserted defect was established.

## Repair

Ambiguous owner lookup now consults existing registered continuity, shared-local permission and authenticated route authorization without selecting or repairing a historical owner. Missing current permission and conflicting profile selectors remain rejected.

Ordinary SQLite publication retains its exact expected-state comparison. It considers only introduced or changed physical browser/desktop bindings, ignores display labels, and checks current exact desktop-generation retirement fences through a read-only projection independent of launch history. Completed releases require no historical launch proof. Whole-ledger process admission, atomic managed launch publication and destructive release protocols remain distinct and unchanged. Startup leaves invalid historical custody untouched for diagnostics instead of refusing ordinary host creation or performing recovery from it.

The first broad session run passed 99 tests and failed one host test that expected ordinary observation to be denied. The baseline store validator already omitted the old unpublished-launch veto; the test expectation conflicted with P243 and the agreed contract. Its replacement verifies that ordinary observation leaves launch custody unpublished and that the managed commit still atomically publishes and acknowledges the intent, including stale-state conflict behavior. This failure is retained, not counted as a passed run.

## Validation

Final qualification is pending. The initial publication regressions turn green, all 110 profile-routing tests pass, and docs, policy wiring, links and handoff guidance pass. The first strict Clippy attempt fails in the compiler-cache wrapper before linting; retained logs are redacted. A sanitized cache-disabled four-job retry passes. Final affected-surface tests, quality and public workflow results will be recorded before integration.

## Boundaries

This slice changes source and private synthetic fixtures only. No production publication, restart, provider mutation, real account, authentication reset or broad process cleanup is performed. Original consumer/account and native viewer acceptance are separate. P237 and P240 checkouts remain untouched.

The [plan](../plans/0244-2026-10-14-ordinary-denial-isolation.md) owns delivery; private diagnostic logs use the p244 prefix outside the product repository.
