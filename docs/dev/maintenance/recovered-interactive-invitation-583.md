# Retained interactive invitation recovery

Product-Lane: PL-BUGFIX
Work item: 583
Branch: fix/recovered-interactive-invitation
Base: a4dce26e421884adee19b21a00b2b61a79d09e83
State: OPEN

## Current State

Source qualification is complete; PR integration, normal optimized candidate and installed acceptance belong to the coordinator. [Consolidated validation](recovered-interactive-invitation-583-validation.json) retains all failures, exact commands, source/binary fingerprints and remaining gates.

#579 qualified physical recovery while preserving the original logical human hold. Its physical target digest necessarily changes; the original invitation stays bound to the old target and Native reports view intent. The existing target guard is correct and remains intact.

## Outcome and bounds

Reconcile the same interactive invitation only through uniquely qualified owned recovery. Preserve invitation ID, human/pauseReads and exact control epoch. Require an exact current session snapshot, original immutable terminal settled-close scope, original and current uniquely published launch custody with kernel identity, unchanged logical browser/session/tab scope, and fresh original process absence. Reject ambiguous, incomplete or unrelated history and concurrent target/control changes. Append an attributable old/new digest transition under SQLite writer and existing control-file locks; no generic target guard weakening, manual state edit, new invitation, foreign control, browser input, website replay or runtime install.

Reuse the previous clean owned source checkout rather than increasing the eight-checkout population. All seven auxiliaries were clean; canonical foreign CONTEXT.md remains untouched. Previous published #579 branch is retained. This checkout now owns only #583; coordinator owns integration/install/live acceptance. Catalog overlap with #579/#580 is source-serial, no concurrent production Rust edits are expected.

## Validation

First demonstrate RED through public retained-browser handoff resolve with original interactive invitation. GREEN must preserve same task/tab/handoff and human epoch while Native intent stays interactive after one qualified relaunch. Negative controls cover missing/ambiguous recovery custody, changed logical scope, foreign/stale control, concurrent state and still-live original identity. Run focused CLI and model checks as appropriate, final format and strict lint; hosted CI skipped by operator policy. No broad review/remediation goal counters consumed. Installation and actual same-viewer acceptance remain separate coordinator gates.


## Qualified design boundary

Actual RED: the existing public host/SQLite recovery succeeds, but the original invitation reports view instead of interactive (3.21s). Initial control-parent setup failures were retained separately. A provisional two-record browser-chain design was superseded before qualification: it proves process ordering but cannot prove a per-tab replacement mapping after same-process reattachment. Its owned compilation was explicitly canceled with exit143; no test verdict and no runtime effect.

The accepted repair records a per-handoff old/new physical digest mapping with the replacement launch and session publication. Only that receipt, freshly requalified against exact completed retirement and original/current launch custody, allows same-epoch control-journal rebinding after owned Native focus. Existing recovered legacy tasks have no such receipt and are intentionally refused, preserving their human hold. The coordinator owns any separately authorized legacy workflow; this ticket adds no discard, cleanup, takeover or generic return operation.


## Source qualification

The public original-invitation-before-recovery case and ten distinct guard scenarios pass (two test groups,29.16s). Related CLI selection covers373 passes and3 existing ignored cases. Its initial two failures were explicit runner-environment preconditions; the same binary passed only those two after the coordinator accepted the correction. The first receipts remain retained. Required workspace Rustfmt and the exact maintained strict workspace Clippy gate pass; the optional all-targets lint run remains failed in unchanged test regions and is not claimed clean. Policy, handoff documentation, changed links and the44-page documentation build pass. Unchanged model/dependency evidence is reused explicitly; no broad review or remediation counter was consumed.

The development test profile overrides only the CLI package to opt0. Installation must use one frozen-source normal optimized CI build without that override. The existing legacy held task is not repaired retroactively. No shared runtime effect occurred in this packet.
