# Plan 0172: Retained Tab And Profile Path Launch Repair

Date: 2026-09-12

State: READY_FOR_INTEGRATION

Consolidation: required

Lane: P172

Branch: fix/plan0172-retained-profile-path

Target: main

Integration: merge

## Objective and authority

Review and act on the Fresh Roof field report in
`docs/dev/notes/0168-2026-09-12-retained-tab-session-identity-conflict-and-profile-path-launch.md`.
Restore one executable retained-tab route across an exact owner alias transition
and make an access-plan profile path safe for native launch without converting
an internal service profile ID into a runtime-profile name.

The operator authorized review and ordinary in-repository repair. This packet
does not authorize another fieldwork request, a production install, a browser
launch, process termination, profile reset, tenant mutation, or broad runtime
cleanup.

## Current state

Field evidence records a retained `default` browser whose physical browser ID
and current owner session differed. Navigation passed pre-effect handle checks,
then failed during post-effect persistence with
`service_navigation_tab_identity_conflict`. The same report records native
launch by the access plan's absolute profile path failing three times with an
invalid colon-bearing `custom:` runtime-profile name. Live job `r695390`
confirms the repeated deterministic error and generic `effect_uncertain`
recourse. Job `r203744` likewise records a transfer admission denial as generic
`effect_uncertain`, even though the admission gate rejected the command before
the requested browser action.

Current source keeps custom service profile IDs distinct for ordinary retained
selection, but authenticated cold and terminal admission assigns the service
profile ID directly to `LaunchOptions.runtime_profile`. Chrome launch also
repeats failures that occur while writing invalid runtime-profile state. Tab
persistence compares the retained physical browser ID and current logical owner
ID as raw strings after navigation instead of proving their exact shared owner.

## Consolidated batch

The batch contains three repairs at one launch and retained-control boundary:

1. Preserve path-backed custom service profiles as path identities during
   authenticated cold and terminal launch selection.
2. Validate any named runtime profile before Chrome process creation, attempt a
   deterministic validation failure once, and classify it as confirmed
   pre-effect with no blind retry.
3. Classify the active runtime-admission drain as confirmed pre-effect, require
   transaction inspection before retry, and retain blind retry as a hard stop.
4. During navigation persistence, accept differing browser aliases only when
   current owner, process, profile, daemon route, session, browser, and target
   evidence resolve the retained tab to one exact physical browser. Preserve
   that physical browser ID and reject every ambiguous or foreign alias.

The post-upgrade projection loss remains an acceptance case for the same exact
owner graph. It is included only as provider-free regression coverage unless
the source investigation proves an additional independent defect.

## Delivery sequence and budget

1. Add focused regressions at the launch-selection, launch-loop, failure
   recourse, and navigation-persistence seams; prove the decisive cases fail on
   the baseline where practical.
2. Implement the smallest source repair and run the focused green checks.
3. Run formatting, strict workspace Clippy, and every changed-surface check
   selected against the batch baseline.
4. Integrate one reviewed source candidate and record the remaining production
   install and field acceptance gates without executing them.

The active-work ceiling is 45 minutes with one implementation pass and one
bounded repair pass after validation. A test that requires a live browser,
production Service State, or profile mutation is deferred to a separately
authorized installed acceptance packet.

## Worker assignments

The primary agent owns diagnosis, implementation, validation, integration, and
closeout. No worker is assigned because the profile launch and retained alias
changes share one Rust ownership and validation boundary.

## Evidence and exit

| Requirement | Evidence | Exit condition |
| --- | --- | --- |
| Path identity | Focused launch-selection test | A custom path-backed service profile produces a profile path and no named runtime profile |
| Prelaunch rejection | Focused Chrome launch test | An invalid named runtime profile fails before any process attempt and appears once |
| Failure semantics | Focused service-failure tests | Invalid runtime-profile input and an active runtime-admission drain report no effect and do not authorize blind retry |
| Retained tab route | Focused persistence regression | Exact owner aliases preserve the physical browser record and navigation metadata |
| Fail closed | Alias mismatch regressions | Foreign, ambiguous, or weakly evidenced identities still return the typed conflict |
| Source qualification | Selector output plus required Rust gates | All checks selected for the complete batch pass at one frozen commit |
| Installed acceptance | Exact binary, transaction, doctor, and field readback | Deferred and explicitly not claimed by this source packet |

## Qualification checkpoint

The focused regressions proved the baseline failures before implementation and
now pass for path-backed custom launch selection, prelaunch validation, failure
recourse, and exact retained-browser alias persistence. Existing retained-child
custody, terminal custom-profile reopening, authenticated cold admission, and
route-confusion gates also pass. Strict workspace Clippy passed. The first
route-confusion run encountered host thread exhaustion while Clippy was active;
its isolated rerun passed every gate and is retained as infrastructure evidence,
not a product failure.

Current read-only production evidence shows the previously missing browser and
South Carolina target projected again under owner generation 102 after upgrade
transaction `upgrade-55a6f48d-5988-4be8-a626-1d234c6e0a1b` reached accepted
revision 14 with no outstanding obligations. That convergence closes the field
report's projection observation as a transient transfer state; it does not prove
an additional persistence defect or authorize installed acceptance.

`RUNBOOK.md` remains the sole current execution status.
