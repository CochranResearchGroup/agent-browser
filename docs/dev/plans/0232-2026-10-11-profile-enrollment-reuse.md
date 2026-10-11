# P232: Existing profile enrollment reuse

Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#244
Branch: fix/profile-enrollment-reuse
Baseline: 624c22e8df7f7f79a18b8d4b09d859f20473d817
State: OPEN; enrollment qualified, workflow source checkpoint awaiting final qualification

Restore ordinary opening of an existing exact profile without caller revision choreography. Test the protected enrollment seam across persisted state with a new request key. Preserve operator, physical identity, active capability and explicit revision fencing. Investigate recovery absence failures without modifying real profiles or production.

P231, P233, P234 and P235 are integrated. Merge 3028ea34 brings their source into this lane without rewriting enrollment custody. This lane owns Lease Authority enrollment, exact-profile session reconciliation and the corresponding user guidance. No production publication, provider mutation, QBO navigation or credential effects are authorized by source qualification.

Validation: red/green enrollment regression, Lease Authority suite, meaningful recovery fixtures, format and strict workspace Clippy. Standards and Spec self-review against the baseline and this plan. Installed acceptance remains separate.

## Enrollment checkpoint

The restart/new-request regression failed with the exact stale-revision code before the repair and passed afterward. All 117 Lease Authority tests, workspace strict Clippy and format passed. Reuse authenticates the existing active capability, operator and physical identity and retains resource and principal authority. Explicit nonzero revision mismatch remains fenced. Synthetic negative requests leave protected state unchanged.

Standards review against the baseline found no accepted violations. Spec review qualifies exact active enrollment reuse only. This restores existing acquisition behavior without a new public flag or request shape; existing user guidance remains accurate. New or changed capability rotation is outside this repair.

## Remaining workflow packet

Ordinary recovery still needs to reconcile a live profile browser against stale session inventory before attempting a replacement launch. Do not infer absence from the saved PID, clear a live singleton lock or close an operator-used browser. Preserve peer tabs and authentication. Reproduce through ordinary acquisition and the public recovery seam, then add an exact live-browser reuse transition and qualify no duplicate process or tab on replay and partial failure. Installed publication and acceptance remain unperformed.

Runtime observations and incident identifiers remain private, outside tracked product sources. Memory disposition: forbidden; no explicit user memory-write request.

## Alice/Bob reconciliation repair

The existing G05, G23, G43 and G44 requirements govern this packet. An ordinary exact-profile acquisition must reconcile a currently live browser before capacity placement or replacement launch. Stale or absent saved inventory must not become a refusal. Reconciliation must preserve peer tabs, authentication, session and handoff identities, and must not take lifecycle custody of a browser used by another caller.

Seams under test: native BrowserRuntimeDriver acquisition with a disposable real Chrome profile; BrowserSessionManager ordinary open and retained continuation; persistent session-state reload and addressed-session close. Start with missing saved inventory, then stale physical identity and peer preservation. Positive current process/profile/endpoint identity is required for attachment; current concrete attachment or capacity failures retain actionable diagnostics.

No production runtime effect is part of this repair. Synthetic browser processes remain bound to their test owner for cleanup.

## Publication replay

Initial task-tab acquisition is part of ordinary session continuity. Qualify a tab created before a failed session publication, then reload the old baseline in a fresh requester and repeat the same named task. Reuse its observed target without creating another tab or browser. The seam must receive the exact catalog profile and task identity rather than infer ownership from an unrelated bootstrap page. Any private effect evidence belongs with profile runtime state, outside tracked product files; malformed unrelated history must remain diagnostic and cannot veto acquisition.

## Workflow source checkpoint

The source now reconciles an exact current Linux Chrome profile before replacement, acquires separate background task tabs, preserves borrowed browser processes on task close, and rejoins current native desktop inventory when it can prove placement. An exited borrowed browser can acquire fresh owned native launch custody. Fresh owned launches clear the worker's prior borrowed lifecycle state. Configured executable selection applies to subsequent launches and does not veto current-browser reuse.

Task-specific private acquisition evidence supports initial publication replay and retained-target replacement replay. Initial task names and retained logical tab identities use separate claim namespaces. Existing targets are reused without replaying navigation or changing subsequent page work. Unavailable journal persistence is diagnostic; it does not block acquisition. Transport failure during restoration navigation and loss of the private journal are not qualified by this checkpoint.

Earlier real-Chrome fixture checkpoints passed missing-inventory reuse, task-tab separation, peer-preserving close, stale inventory continuation across requester restart, initial lost-publication replay, and free-profile absence despite unrelated stale browser metadata. Provider-free fixtures passed current native placement, local retained recovery and exited borrowed-browser recovery. The earlier browser-session selection passed 95 tests with five ignored browser tests. These receipts precede the latest identity and retained-publication changes and do not qualify the final source.

The latest full Service Model suite passed its 230 unit tests, integration binaries and three doc tests. Client contracts, Remote View handoff documentation and documentation links passed. Workspace formatting passed; latest source is also rustfmt-formatted. Final strict workspace Clippy, the updated ordinary retained-publication fixture, and the final browser-session/native selection remain pending. Native compilation admission remained unavailable with full host swap and insufficient available memory for the configured reserve and bounded test build. No reserve bypass, production publication, provider mutation, real-profile cleanup or browser navigation was attempted.

The optional policy-wiring check fails because it expects an older AGENTS.md policy-routing layout. Both its script and AGENTS.md are unchanged from integrated origin/main. This is baseline debt, not a passing gate.

Standards review corrected executable-selection drift, stale borrowed lifecycle state and imprecise descendant-directory profile evidence. Spec review remains open until the updated retained-publication fixture proves target reuse and preservation of Alice's subsequent page work while Bob survives. This is an intermediate custody checkpoint, not merge readiness or installed acceptance.
