# P232: Existing profile enrollment reuse

Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#244
Branch: fix/profile-enrollment-reuse
Baseline: 624c22e8df7f7f79a18b8d4b09d859f20473d817
State: SOURCE QUALIFIED; workflow recovery remains open

Restore ordinary opening of an existing exact profile without caller revision choreography. Test the protected enrollment seam across persisted state with a new request key. Preserve operator, physical identity, active capability and explicit revision fencing. Investigate recovery absence failures without modifying real profiles or production.

P231 remains in its original dirty checkout; this lane owns Lease Authority and recovery tests only. No shared help or upgrade files are edited. No production publication, provider mutation, QBO navigation or credential effects are authorized by source qualification.

Validation: red/green enrollment regression, Lease Authority suite, meaningful recovery fixtures, format and strict workspace Clippy. Standards and Spec self-review against the baseline and this plan. Installed acceptance remains separate.

## Enrollment checkpoint

The restart/new-request regression failed with the exact stale-revision code before the repair and passed afterward. All 117 Lease Authority tests, workspace strict Clippy and format passed. Reuse authenticates the existing active capability, operator and physical identity and retains resource and principal authority. Explicit nonzero revision mismatch remains fenced. Synthetic negative requests leave protected state unchanged.

Standards review against the baseline found no accepted violations. Spec review qualifies exact active enrollment reuse only. This restores existing acquisition behavior without a new public flag or request shape; existing user guidance remains accurate. New or changed capability rotation is outside this repair.

## Remaining workflow packet

Ordinary recovery still needs to reconcile a live profile browser against stale session inventory before attempting a replacement launch. Do not infer absence from the saved PID, clear a live singleton lock or close an operator-used browser. Preserve peer tabs and authentication. Reproduce through ordinary acquisition and the public recovery seam, then add an exact live-browser reuse transition and qualify no duplicate process or tab on replay and partial failure. Installed publication and acceptance remain unperformed.

Runtime observations and incident identifiers remain private, outside tracked product sources. Memory disposition: forbidden; no explicit user memory-write request.
