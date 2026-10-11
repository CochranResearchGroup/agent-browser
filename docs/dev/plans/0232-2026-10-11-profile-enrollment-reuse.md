# P232: Existing profile enrollment reuse

Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#244
Branch: fix/profile-enrollment-reuse
Baseline: 624c22e8df7f7f79a18b8d4b09d859f20473d817
State: OPEN; source qualified; merge and installed acceptance unperformed

Restore ordinary opening of an existing exact profile without caller revision choreography. Test the protected enrollment seam across persisted state with a new request key. Preserve operator, physical identity, active capability and explicit revision fencing. Investigate recovery absence failures without modifying real profiles or production.

P231, P233, P234 and P235 are integrated. Merge 3028ea34 brings their source into this lane without rewriting enrollment custody. This lane owns Lease Authority enrollment, exact-profile session reconciliation and the corresponding user guidance. No production publication, provider mutation, QBO navigation or credential effects are authorized by source qualification.

Validation: red/green enrollment regression, Lease Authority suite, meaningful recovery fixtures, format and strict workspace Clippy. Standards and Spec self-review against the baseline and this plan. Installed acceptance remains separate.

## Enrollment checkpoint

The restart/new-request regression failed with the exact stale-revision code before the repair and passed afterward. All 117 Lease Authority tests, workspace strict Clippy and format passed. Reuse authenticates the existing active capability, operator and physical identity and retains resource and principal authority. Explicit nonzero revision mismatch remains fenced. Synthetic negative requests leave protected state unchanged.

Standards review against the baseline found no accepted violations. Spec review qualifies exact active enrollment reuse only. This restores existing acquisition behavior without a new public flag or request shape; existing user guidance remains accurate. New or changed capability rotation is outside this repair.

## Workflow packet scope

Ordinary recovery must reconcile a live profile browser against stale session inventory before attempting a replacement launch. Do not infer absence from the saved PID, clear a live singleton lock or close an operator-used browser. Preserve peer tabs and authentication. Reproduce through ordinary acquisition and the public recovery seam, then add an exact live-browser reuse transition and qualify no duplicate process or tab on replay and partial failure. Installed publication and acceptance remain unperformed.

Runtime observations and incident identifiers remain private, outside tracked product sources. Memory disposition: forbidden; no explicit user memory-write request.

## Alice/Bob reconciliation repair

The existing G05, G23, G43 and G44 requirements govern this packet. An ordinary exact-profile acquisition must reconcile a currently live browser before capacity placement or replacement launch. Stale or absent saved inventory must not become a refusal. Reconciliation must preserve peer tabs, authentication, session and handoff identities, and must not take lifecycle custody of a browser used by another caller.

Seams under test: native BrowserRuntimeDriver acquisition with a disposable real Chrome profile; BrowserSessionManager ordinary open and retained continuation; persistent session-state reload and addressed-session close. Start with missing saved inventory, then stale physical identity and peer preservation. Positive current process/profile/endpoint identity is required for attachment; current concrete attachment or capacity failures retain actionable diagnostics.

No production runtime effect is part of this repair. Synthetic browser processes remain bound to their test owner for cleanup.

## Publication replay

Initial task-tab acquisition is part of ordinary session continuity. Qualify a tab created before a failed session publication, then reload the old baseline in a fresh requester and repeat the same named task. Reuse its observed target without creating another tab or browser. The seam must receive the exact catalog profile and task identity rather than infer ownership from an unrelated bootstrap page. Any private effect evidence belongs with profile runtime state, outside tracked product files; malformed unrelated history must remain diagnostic and cannot veto acquisition.

## Historical workflow source checkpoint

The source now reconciles an exact current Linux Chrome profile before replacement, acquires separate background task tabs, preserves borrowed browser processes on task close, and rejoins current native desktop inventory when it can prove placement. An exited borrowed browser can acquire fresh owned native launch custody. Fresh owned launches clear the worker's prior borrowed lifecycle state. Configured executable selection applies to subsequent launches and does not veto current-browser reuse.

Task-specific private acquisition evidence supports initial publication replay and retained-target replacement replay. Initial task names and retained logical tab identities use separate claim namespaces. Existing targets are reused without replaying navigation or changing subsequent page work. Unavailable journal persistence is diagnostic; it does not block acquisition. Transport failure during restoration navigation and loss of the private journal are not qualified by this checkpoint.

Earlier real-Chrome fixture checkpoints passed missing-inventory reuse, task-tab separation, peer-preserving close, stale inventory continuation across requester restart, initial lost-publication replay, and free-profile absence despite unrelated stale browser metadata. Provider-free fixtures passed current native placement, local retained recovery and exited borrowed-browser recovery. The earlier browser-session selection passed 95 tests with five ignored browser tests. These receipts precede the latest identity and retained-publication changes and do not qualify the final source.

The latest full Service Model suite passed its 230 unit tests, integration binaries and three doc tests. Client contracts, Remote View handoff documentation and documentation links passed. Workspace formatting passed; latest source is also rustfmt-formatted. Final strict workspace Clippy, the updated ordinary retained-publication fixture, and the final browser-session/native selection remain pending. Native compilation admission remained unavailable with full host swap and insufficient available memory for the configured reserve and bounded test build. No reserve bypass, production publication, provider mutation, real-profile cleanup or browser navigation was attempted.

The optional policy-wiring check fails because it expects an older AGENTS.md policy-routing layout. Both its script and AGENTS.md are unchanged from integrated origin/main. This is baseline debt, not a passing gate.

Standards review corrected executable-selection drift, stale borrowed lifecycle state and imprecise descendant-directory profile evidence. Spec review remains open until the updated retained-publication fixture proves target reuse and preservation of Alice's subsequent page work while Bob survives. This is an intermediate custody checkpoint, not merge readiness or installed acceptance.

## Resumed qualification under the operator waiver

The operator authorized a temporary memory waiver for this repair. Validation uses one Cargo invocation at a time, four build jobs, a 10 GiB host reserve, and cgroup caps of 5 GiB for native tests and 3 GiB for Clippy. The repository defaults and aggregate cgroup limits are unchanged. This waiver authorizes local validation only.

The retained-target publication regression was observed red with `retained recovery publication retry duplicated Alice's target`. The first fixed run reused the target but exposed a fixture error: the internal action is `evaluate`, not `eval`. The fixture now verifies its successful page change before discarding the session publication. Both native selections then passed all six fixtures; the final selection took 17.60 seconds. Alice's replacement target and subsequent page title survive a fresh requester and old-baseline replay, and Bob's target survives Alice's close.

The final provider-free session selection passed 96 tests, with the six real-browser fixtures explicitly ignored and separately executed. Its first run failed one queued-handoff fixture: the mock did not answer the newly required CDP process-identity probe. The mock now returns its actual server process ID. Retained endpoint qualification requires a current executable, stable kernel start token and matching CDP browser PID; executable-name classification does not confer custody. The corrected full selection passed in 23.31 seconds. The first failures remain attributable fixture and product evidence, not erased retries.

Final workspace format and strict Clippy passed. Lease Authority source and Cargo.lock are unchanged since the qualified enrollment checkpoint, so its 117-test receipt remains applicable; its architecture check passed again. The latest Service Model receipt remains applicable because subsequent edits are confined to native adapters and fixtures. Exact-code CLI build and all seven pinned no-launch service/installation checks subsequently passed. Production installation and actual operator-viewer acceptance remain excluded.

## Final source qualification

The CLI built from code commit `7a2a8bd458884c55da044794cd69fd140aeb3811` has SHA256 `4519582f502321d2f6b7cef618f3672458cfe2877d9cb0822b5591337cc43eac`. A private immutable candidate copy passed service-contract metadata, MCP resource reads, profile-source and site-policy provenance, HTTP and MCP incident summaries, and workstation installation. No production binary was replaced. The installation fixture proved two owned hosts and zero residue. Temporary no-launch smoke hosts were retired using exact candidate and runtime-home ownership; this cleanup does not touch production processes. Fresh OS census found zero remaining candidate hosts.

### Standards review

Serial review against freshly fetched origin/main corrected configured-executable drift, borrowed lifecycle state, physical profile evidence and retained endpoint identity qualification. Final format and strict workspace Clippy passed. No blocking finding remains in this bounded source packet. Prior Lease Authority, Service Model, client, documentation and unchanged workstation receipts are reused only for unchanged inputs. The optional policy-wiring failure remains unrelated baseline debt.

### Spec review

G05 and G23 are covered by stale/absent inventory reuse, current physical profile evidence and free-profile recovery despite unrelated stale metadata. G43 is covered by one browser with separate background task targets and publication replay without duplicate targets. G44 is covered by peer-preserving borrowed close and final owned-browser release across requester restart. The retained-publication fixture additionally preserves subsequent Alice page work and Bob's target. No blocking finding remains for these tested transitions.

This qualifies Linux synthetic Chrome source behavior. It does not prove the original historical absence failure's root cause, the entire failure matrix, other platforms, transport failure plus journal loss, or actual native operator-viewer acceptance. Production installation, provider effects, real-profile cleanup and merge remain unperformed. The plan stays OPEN and its worktree is retained for integration. Memory disposition: forbidden because no explicit memory-write request was given.

## Authorized installed validation and publication

After source qualification, the operator explicitly directed development runtime validation, integration and qualified production publication, followed by ordinary exact-profile open verification. This supersedes the earlier source-only effect boundary for that sequence. Preserve peer tabs and existing browsers; no accounting transactions, credential reset, broad cleanup or reboot. The operator also explicitly required memory recording; the qualified source closeout was queued in agent_browser_main. Earlier forbidden dispositions remain historical receipts.

Fresh origin/main includes P236 upgrade progress. It is integrated before sealing the production-shaped candidate. The first build from the previous source was stopped; partial output is not a qualified artifact. Temporary memory waiver continues only for bounded local compilation, with one invocation, four jobs and a 10 GiB host reserve.
