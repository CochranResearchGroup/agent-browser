# P257 retirement lock diagnostics

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: [P257](../plans/0257-2026-10-20-retirement-lock.md), CochranResearchGroup/agent-browser#307
Related lanes: P249, P256

## Result

Retirement previously held the exclusive Service State file lock across an
external effect. A minimal acquired-guard fixture reproduced the reported
diagnostic timeout twice: a 20 ms read timed out while the sidecar still
reported only 1 ms of hold time.

The repository now serializes retirements with a separate exclusive fence and
retains shared Service State admission during the effect. Snapshot diagnostics
can read while controller and handoff commits remain excluded. Interrupted
transactions recover under exclusive state admission before shared admission
is retained. All acquisition and recovery attempts share one lock deadline.

A blocked handoff writer returns bounded refusal with the executable read-only
inspection command `agent-browser service jobs --json`. Its diagnostic names
the retirement fence separately from shared state admission. After retirement
releases, the same handoff authority admits the candidate while retaining the
browser, profile, process and target-set identity. This is a bounded recovery
path, not permission to cancel a retirement, replay an uncertain effect, force
unlock or start another profile lane.

Unix holders record a same-boot monotonic timestamp. Contenders calculate the
current hold duration after validating the exact PID/start identity and lock
token. Legacy holders and unsupported clocks report unknown rather than the
last frozen sidecar sample. Recorded build generation remains independent of
the installed executable path; the incident's historical discrepancy is not
normalized away.

## Acceptance and review

Implementation commit: `85de4b483`; baseline: `bca063738`.
Review command: `git diff bca063738...85de4b483`.

The final Service State selection passes all 49 tests in 9.16 seconds after
compilation. It covers cross-process read availability and controller write
exclusion, retirement serialization, actual job inspection under a retained
guard, bounded handoff refusal followed by candidate admission, preserved
identity, interrupted four-file transaction recovery, fresh duration, and
unknown legacy clock evidence. Three browserless native-handler, stale
transport and legacy CLI no-op regressions also pass in 0.35 seconds.

The final wider retirement selection passes 58 with one explicit
disposable-Chrome case ignored in 86.90 seconds. The first wider
selection passed 57 and ignored that case; its new
handoff fixture failed on a non-hex synthetic profile digest before reaching
lock contention. The fixture now uses valid evidence and passes in the final
49-test selection. The earlier compiler field-name error and Clippy needless
return were corrected; they are not claimed as passing attempts.

Formatting, strict workspace Clippy, docs build, Remote View handoff docs,
API/MCP and compact skill parity, policy wiring with 11 cases, selected P257
planning completeness, and changed-link checks pass. The selector's broad
installer recommendations come from the CLI help file; installer behavior,
payloads, assets and provisioning are unchanged, so their effectful fixtures
are outside this source repair.

An optional historical archive test fails on the pre-existing 278-line RUNBOOK
limit at the baseline. Policy 0043 permits preserving historical clutter and
scoping migration separately. A direct optional policy-context Python test
could not import its installed engine in the current Python environment;
current governance packets and the repository policy-wiring gate pass. Neither
optional check is represented as successful.

## Standards

No blocking findings in the implementation diff. Existing ownership seams and
write exclusion remain explicit; all five required user-facing documentation
surfaces and the configured domain authority are updated. No speculative
workflow or provider layer was introduced. The historical archive finding is
pre-existing and outside this repair, not an erased validation failure.

## Spec

No blocking findings against #307's supported bounded-recovery option. Reads
remain available during the actual retirement guard, handoff writes return
actionable bounded refusal, and the unchanged browserless success/no-op
contract remains covered. Pending external effects retain custody. This does
not establish successful handoff during an unfinished effect, account login
continuity in a real browser, cross-platform runtime qualification, or repair
of an already-running older executable.

Accepted review findings: Standards zero; Spec zero. Highest accepted severity:
none on either axis.

## Boundary

Source qualification only. Production executables, services, accounts,
profiles, cookies, browser processes and installed shared guidance are
unchanged. No GitHub CI was enabled or dispatched. The clean dashboard primary
was reused; foreign root and resource-bounds worktrees and pinned candidates
remain preserved. Production publication and real-account continuation are
separate from these provider-free checks.

[PR335](https://github.com/CochranResearchGroup/agent-browser/pull/335) owns
canonical integration. Its later documentation commits do not change the
qualified executable inputs.
