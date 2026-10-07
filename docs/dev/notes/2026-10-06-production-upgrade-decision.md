# Production upgrade decision and qualification

Date: 2026-10-06
Product lane: PL-PLATFORM
Disposition: accepted-evidence
Owning plan or work item: [Agent Browser issue 218](https://github.com/CochranResearchGroup/agent-browser/issues/218)
Related lanes: PL-AUTH

## Agreed outcome

Make the landed browser workflow improvements available in everyday production use after focused development validation. The operator confirmed this outcome, a short planned service interruption, separate application-authentication qualification, and restoration of previous compatible runtime versions on essential workflow failure. This interview records advice and shared understanding, not a runtime deployment or formal release.

## Initial source baseline

Agent Browser PR 212 merged at `85c6fa22e67f9767578110b83584f5573b3c4cfb`. PR 213 records bounded P223 completion; canonical main at the interview was `3a330ac0f9a66753c8a51bb6babad6e610b48e0e`. Remote View PR 310 merged at `8b11f7f84d36e4d5284454be1fad5a845a6aefbc`; it supplies native viewing metadata for the joined current desktop generation and exact stopped-slot reclamation.

Canonical RUNBOOK records isolated Agent Browser acceptance with the previously installed development provider, not acceptance of the newly built canonical provider. It preserves the original expired-link failure, one initial safe recovery refusal with an unproven transient cause, and unverified application authentication. The local dirty checkout's paused planning text is older than canonical completion and must not reopen P223.

## Recommended sequence

1. Read current production and development executable identities, provider capabilities, service ownership, profile occupancy and active handoffs. Bind evidence and previous compatible versions to each environment. Current live runtime state has not been refreshed by this interview.
2. Build and qualify the landed Remote View candidate in the isolated development environment, then check it with the landed Agent Browser candidate. Reuse unaffected source tests. A passing service doctor alone is insufficient.
3. Require browser launch, ordinary automation, native handoff and visible keyboard control, continuation after natural idle cleanup without reopening the viewer, actionable terminal links, and task-owned cleanup preserving peers. A recurring recovery refusal blocks promotion pending bounded diagnosis; do not turn retries into acceptance.
4. Once the exact pair passes, schedule the agreed short interruption. Prefer provider-first promotion, verify provider readiness, then promote Agent Browser and verify the same essential workflows. Before effects, prove the existing consumer can safely coexist with the candidate provider during the interval. If it cannot, use a bounded stopped-service cutover with both rollback versions retained. An atomic upgrade requirement has not been established.
5. On essential failure, stop promotion and restore the previously compatible runtime versions through their supported transition. Preserve profiles, existing work and attributable failure evidence. Verify state/schema backward compatibility before promising rollback; restoring binaries alone may be insufficient.

## Boundaries and remaining facts

Application authentication is a separate qualification, not a prerequisite for this platform promotion. Do not claim authenticated ChatGPT continuity from the Cloudflare interstitial. Preserve existing authentication and handoff expiry.

No formal release, broad platform matrix, authentication reset or broad cleanup is part of this decision. The deployment sequence, exact supported rollback procedure and current installed identities remain execution-preflight facts. This note is deferred input for a future bounded rollout, not an active execution plan or a second backlog.

The existing CONTEXT.md remains the domain-language authority. This decision introduces no new domain terms or architectural ADR.

## Evidence

- [Agent Browser implementation PR 212](https://github.com/CochranResearchGroup/agent-browser/pull/212)
- [Agent Browser closeout PR 213](https://github.com/CochranResearchGroup/agent-browser/pull/213)
- [Remote View dependency PR 310](https://github.com/CochranResearchGroup/remote-view/pull/310)
- Canonical RUNBOOK at `3a330ac0f9a66753c8a51bb6babad6e610b48e0e`
- Operator interview in this session: “yes all” and “sure” confirmed the recommendations.

## First execution result

The operator subsequently requested execution with “do it”. Production promotion stopped at the agreed development gate. No production binary or configuration was changed.

The exact canonical Remote View release build and verified bundle extraction passed. Its binary was installed into p221 and all four owned software units adopted it. Existing X, VNC, window-manager and audio process identities survived. The Agent Browser development doctor passed, but the actual native browser handoff failed with `remote_view_focus_window_readback_required`.

Read-only assignment observation and window inspection proved that the current desktop generation and foreground Chrome window were available. The canonical provider's window response includes `dialog`, `modal` and `transientFor`. Agent Browser's `RemoteViewApplicationWindow` uses `deny_unknown_fields` and declares none of those fields, so its window response cannot deserialize. This is a concrete consumer/provider compatibility failure, not a provider readiness or application-authentication result. Preserve outer-envelope, process and generation checks when repairing the consumer's treatment of additive window metadata.

The initial harness request separately omitted `session-name` and failed with `browser_session_field_missing:sessionName`. Fresh state comparison showed no browser was created by that request. The corrected request launched a task-owned disposable browser before failing window readback. Exact ordinary close succeeded and its recorded PID was absent afterward.

The previous p221 provider was restored through normal install and software adoption. Its binary hash and the unchanged Agent Browser hash match the initial rollback records; the development install doctor passes. Production Agent Browser and Remote View hashes match the initial readback. The production-shaped Agent Browser build was intentionally cancelled before sealing or installation after the compatibility gate failed. Its partial build and attributable logs remain private evidence, not a validated artifact.

Both temporary clean build worktrees and the task-owned viewer page were removed. Existing dirty checkouts and borrowed viewer pages remain. Private rollback, build, API, preservation and terminal readback receipts are retained under the operator's user-scoped runtime evidence. Application authentication remains separately unverified.

The boundary after this first attempt was a bounded compatibility repair followed by fresh exact-pair qualification. P223's previous bounded acceptance is not rewritten as acceptance of this canonical provider build.


## Repair and fresh qualification

The operator then approved the necessary repair and continuation. [PR 215](https://github.com/CochranResearchGroup/agent-browser/pull/215) recognizes the three typed optional window metadata fields while retaining malformed-value, unknown-field, exact-process and generation checks. Its regression was red before repair; all 16 focused adapter and record tests pass.

Production preflight then exposed a separate prior-boot migration boundary. Existing migration recognized validated previous Linux boot evidence for unreferenced process identities but not for owner-only or terminal-projection identities. [PR 217](https://github.com/CochranResearchGroup/agent-browser/pull/217) reuses that established absence predicate in the two inert paths. A regression with a currently live reused PID was red before repair. All 28 focused migration tests pass, including live and uncertain evidence rejection and exact commit/rollback bytes. Pending-transfer, principal-binding and retained-projection gates and final cross-record validation remain. Both repair slices pass format and strict workspace Clippy.

The final sealed release candidate is bound to source `02cda5230af139ef033ab1e514a8786fadfd20ff`, which entered canonical main through `250aabef4de24bb538a2c62c4ba6c738cc7aeba9`. Its binary digest is `09f912afe4c5261e71a48c8d696030a905efa54ddd48b14910f4a225e7141c28`. The verified Remote View release digest is `163bbfb3bcc6eb4c39a0419774cd7dbb18284bc8e8aecd08900e0ec3680ba236`. The exact pair passes isolated doctor, three fresh launch/read/close smokes, native handoff readiness, an authenticated connected viewer, a complete paced synthetic keyboard marker, ordinary automation, natural idle cleanup and recovery on the first ordinary command before reopening the viewer. The logical browser/session and visible tab are retained; the durable handoff reconnects afterward. Exact close returns the same logical identities, both recorded physical browser PIDs are absent, and the final OS readback finds no development Chrome or crashpad process residue. The terminal API returns HTTP 410 with an actionable target-closed message. Borrowed viewer pages remain; the task-owned page was closed.

The final candidate's read-only production migration preview passes. Profiles, capabilities, principals, owners and handoffs are unchanged, and no protected record is removed. The supported transaction retains raw backups and materializes inert history rather than granting current authority from historical labels or process numbers.

## Native installer correction and remaining execution

The operator clarified that legacy RDP is unused. The earlier sudo gate came from unconditional workstation provisioning, not from native Remote View. The qualified development workflow succeeded without refreshing that helper. The existing protected lease authority is independently ready. Do not run the prepared RDP helper updater for this native rollout.

The bounded installer repair selects native prerequisites from explicit native settings, rejects partial configuration rather than falling back to RDP, and uses a read-only protected-authority check. Native installation, resume and reconciliation omit legacy helper provisioning, RDP users, XRDP, Docker membership, Guacamole startup and the Guacamole backup timer. Production install doctor uses native inventory readiness. The native provider continues to own desktop operation and its authenticated viewer.

Production promotion is still pending. The previously qualified sealed bytes do not contain this installer repair. Issue 218 owns validation and integration of the repair, a newly sealed candidate, fresh exact-pair qualification, target and transaction readback, identity-preserving provider promotion, supported candidate installation, essential production checks and the agreed rollback boundary. This note is a source locator, not effect authority. Application authentication remains separate. No formal release, authentication reset, expiry change, manual state rewrite or broad cleanup is claimed.

## Native configuration loading correction

PR220 passed native installer regression and exact-pair development qualification. The first production attempt then proved that the user-scoped dotenv loader omitted the four native Remote View binding keys. Development supplied those keys directly. The installer failed at the host precondition and durably preserved the old Agent Browser generation. Remote View configuration and software were restored through supported apply, install and software-adoption commands. All 97 protected file hashes and existing native peer start identities matched the pre-cutover baseline. No RDP helper or authority was updated.

The successor adds those four keys to the existing dotenv allowlist, including partial bindings so they reach native validation. A red/green regression covers the persisted configuration path. A new sealed candidate and complete exact-pair development qualification are required before another production attempt.
