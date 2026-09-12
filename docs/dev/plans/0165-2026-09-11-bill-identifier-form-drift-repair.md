# Plan 0165: BILL Identifier Form Drift Repair

Date: 2026-09-11

State: OPEN

Consolidation: required

Lane: P165

Branch: fix/profile-repair-owner-generation

Target: main

Integration: merge

Parent: Books Receipts Plan 0240 consumer acceptance

## Objective and authority

Update the repository-owned `bill-login-v1` recipe for the observed BILL
identifier form, install one qualified production candidate through the
canonical workstation transaction, and prove one sealed reauthentication on
the retained `bill-soylei` profile. The operator authorized this packet.

The packet must preserve the profile directory, authenticated browser, tabs,
cookies, credentials, and unrelated runtime owners. It does not authorize a
profile reset, transaction mutation, receipt upload, review, QuickBooks sync,
or accounting effect.

## Current state

The recipe and shared-task-lease repairs are integrated at `5c247f9d` and
installed as generation `0.28.0-c0cc081c1c6b-8e9cb639ccc3`. Focused tests,
the full Authentication Run suite, formatting, and strict Clippy pass. The
retained `bill-soylei` browser profile was relaunched without reset or reseed.

The live run now recognizes the visible `loginEmail` plus `Continue` form and
is durably paused at `awaiting_identifier`, transition 1, with one observation,
zero action receipts, and no pending effect. Its exact resume failed before a
credential action because Auth Vault profile `bill-soylei` does not exist. The
provider returned `effect_uncertain` plus hard stop `blind_retry`, so the live
sequence stopped without retry. Read-only metadata proves the preserved Chrome
profile has one BILL password-manager login while no live BILL password exists
in the environment or Auth Vault. The remaining repair must keep that password
inside Chrome rather than extracting or duplicating it.

## Consolidated batch

The batch contains two joined consumer repairs: accept the exact tenant-neutral selector
`input#login-email-input[name='loginEmail']` and the identifier control label
`Continue`, while preserving the existing selector and `Save` variants. A
single classifier regression protects both forms. The normal shared-profile
broker path also returns a `shared` task lease beneath the exclusive profile
owner; Authentication Run accepts that service-serialized exact-tab lease as
well as the legacy `exclusive` task lease, while continuing to reject released,
expired, conflicted, and human-takeover leases. No broader login provider or
website interaction feature is included.

The bounded remediation adds one BILL-recipe password source:
`browser_autofill`. The sealed provider may submit only when the exact closed
password selector is visible, contains a nonempty browser-provided value, and
has exactly one allowed submit control. It returns only boolean/count evidence,
never the value. The Auth Vault entry supplies the username only; an empty vault
password is represented by a fixed non-secret browser-autofill marker that is
never filled or submitted. Existing real vault-password behavior
and every origin, state-instance, effect fence, SMS-watch, and replay guard stay
unchanged. No generic password-manager extraction or cross-site autofill API is
included.

Fieldwork after candidate installation exposed a second bounded runtime defect:
the profile-repair plan sealed the global Service State revision, while the
planning and applying control-plane job receipts themselves advance that global
revision. Consequently every otherwise exact plan/apply sequence failed closed
as `profile_recovery_plan_stale`. Recovery now seals and revalidates the exact
profile identity, runtime-owner revision and generation, lifecycle, process,
lock, browser, and conflicting-lease graph; unrelated control-plane envelope
history no longer invalidates the plan. Relevant graph drift remains a hard
pre-effect failure.

The next consumer replay exposed a third exact alias defect: Authentication Run
target selection used the current-session-only handle validator, rejecting the
durable browser ID after a runtime handoff even though the generic daemon-aware
validator already authorizes that ID from the current runtime-owner binding.
Authentication Run now uses the same daemon-aware route validator; unrelated
browser IDs and mismatched session routes remain rejected before effects.

After installation of the lock repair identified by SHA-256 prefix
`842bb1be` and suffix `a48974356`, the next preserving repair launched Chrome
but failed lifecycle registration with
`runtime_lifecycle_bound_browser_identity_inconsistent`. Request and job
`mcp-service_profile_repair_apply-3b1c4a7d-9742-4118-858c-e654eba3e4a0`
cleanly terminated launched PID 93502. The retained profile data and BILL
accounting state were unchanged. Source diagnosis proved the apply callback
entered the replacement launch with the exact terminal-owner binding still in
daemon memory. Registration therefore compared the new process with the old
binding before it could commit the replacement generation.

The source repair is integrated through PR 34 at merge commit `9e544710`. It
retires only a daemon binding that matches all five sealed owner joins
immediately before the single authorized recovery launch. A different owner
ID, profile digest, generation, durable browser ID, or daemon route remains
present and fails closed. The lower-level process-identity validator is
unchanged. The focused regression failed before the repair, then the exact and
mismatch cases plus all 35 profile-recovery module tests passed. Formatting
and strict Clippy also pass. Production installation and another BILL repair
attempt remain pending separate governed gates.

Commit `9f14cb05` is integrated on `origin/main`, and its release candidate is
built at SHA-256 `61a50901a56c3403aeb0ac49cef5c0416cddd64cf4fa5482e31b670c3cd4eb40`.
Installation is blocked before transaction creation by repeated
`service_state_lock_timeout` failures, including with the dashboard service
fully stopped. The dashboard was restored and the preserved BILL browser
remains current at PID 37560. Installed SHA-256 is still
`cfd2e4dc749adcbbee026becd7f9f3b23944f95ee7605273fc4f6c4fad4ca188`.
The next action is lock-owner diagnosis, not another install or authentication
retry.

Production installation and preserving runtime repair are now complete. The
first accepted upgrade exposed that profile-repair launch had advanced the
runtime owner without refreshing its existing same-capability principal
binding. A sealed lease reconciliation restored exact custody without launching
or replacing Chrome. A second accepted upgrade then proved the workstation
supersession path had the same atomicity gap. PR 36 fixes both boundaries:
profile repair binds the acquired daemon session before its postcondition, and
terminal replacement plus observed-owner supersession advance an existing
principal binding atomically with the owner generation.

The final installed candidate has SHA-256
`9598cb89565fdcd1af3bcf0d496ebae3d919a7d4c9b1cd725a3393d430d83e20`,
generation `0.28.0-9598cb89565f-492f49580f7e`, and accepted transaction
`upgrade-50b884da-b2dc-4f0d-96e7-54b45e1770c0`. Post-install doctor proves one
dashboard, one executable generation, zero legacy daemons, one runtime host,
and one ready session supervisor. The retained BILL browser remains PID 49619
with its current singleton lock and eight tabs. Owner and registered-capability
binding both remain generation 62 after installation; the lease is active with
no blocking identity axes, profile diagnosis is ready, and the no-launch access
plan selects exact browser reuse. Cookies, credentials, extensions, and
authenticated site state were preserved. No BILL credential, tenant,
transaction, or accounting effect occurred. The plan remains OPEN only for the
separate governed authentication and read-only consumer acceptance gates.

## Git custody review

Plan 0168 verified that branch tip `5ee95d670e52c1157f0f94b90d176e4066d1cd2f`
is an ancestor of `origin/main`, has zero unique commits, is published at the
same local and remote tip, and has a clean assigned worktree. Git custody is
therefore integrated and eligible for exact worktree and ref closure.

This does not close Plan 0165. The retained authentication run, sealed Auth
Vault entry, and consumer acceptance remain governed operational gates. A
future execution resumes from current runtime evidence on a new exact branch
only if source work is required.

## Delivery sequence and budget

1. Produce focused red and green classifier evidence.
2. Run authentication-focused tests, formatting, the changed-surface selector,
   and diff checks against the frozen source.
3. Commit and integrate the repair, build one production candidate, and apply
   it once through `install workstation`.
4. Verify installed identity and retained profile/browser continuity, seed the
   identifier-only `bill-soylei` Auth Vault entry without a password value,
   then inspect the paused run before one sealed resume.
5. On authentication success, perform the read-only consumer inspection. On
   any uncertain or changed state, stop without retry.

Overall active-work ceiling is 45 minutes. The batch permits one source-frozen
production build, one workstation install transaction, one sealed
reauthentication run, and one repair cycle if a source defect is demonstrated.
Any profile, owner, generation, or transaction mismatch stops live effects.

## Worker assignments

The primary owns source, validation, integration, installation, and live
read-only acceptance. No worker is assigned because the patch and retained
runtime custody are one serialized critical path.

## Evidence and exit

| Requirement | Evidence | Exit condition |
| --- | --- | --- |
| Current form support | Focused red then green classifier test | Exact `loginEmail` plus `Continue` classifies as identifier form |
| Compatibility | Focused form and task-lease tests | Existing email-selector plus `Save` remains accepted; shared or exclusive exact-tab task leases work and inactive leases fail closed |
| Source qualification | Authentication-focused tests, format, validation selector, diff check | All required touched-surface checks pass |
| Installed repair | Workstation transaction and exact binary/generation readback | Candidate is selected without profile or browser replacement |
| Consumer outcome | Sealed run receipt and retained URL/title read | Exact BILL organization authenticates in `bill-soylei` |
| Downstream boundary | Read-only Books Receipts case inspection | Result is recorded without BILL or accounting mutation |

`RUNBOOK.md` remains the sole current execution status.
