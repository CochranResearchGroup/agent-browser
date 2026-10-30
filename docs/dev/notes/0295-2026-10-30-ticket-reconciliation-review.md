# P295 final ticket reconciliation review

Product lane: PL-AUTH. Owning plan: [P295](../plans/0295-2026-10-26-standalone-auth-449.md).
Disposition: bounded review and fixture remediation; no runtime effects.

## Frozen scope

The operator selected base `99faef3d051732f302ce0c21f2a308b6ef3939fc`.
Frozen head: `063f9ab8965de245354c42dfe5083f70a6213ec7`.
Command: `git diff 99faef3d051732f302ce0c21f2a308b6ef3939fc...HEAD`.
The diff is nonempty. Commit inventory uses
`git log 99faef3d051732f302ce0c21f2a308b6ef3939fc..HEAD --oneline`.
Separate read-only reviewers inspected Auth integration and its successors,
including bootstrap, owner login, recovery, TOTP, number matching, declarations,
fixtures and authentication documentation. Unrelated teaching, challenge,
branding and task TTL work since the base is excluded. This is one additional
review expressly requested by the operator after ticket reconciliation; previous
P295 review history remains intact.

The frozen spec is [the standalone integration contract](../contracts/standalone-auth-integration.v1.md)
and issue449. Later explicit owner direction permits ephemeral caller TOTP under
the same approved login grant and authenticated owner web or Slack decisions.
Those clarifications supersede earlier hidden-only or Slack-only wording.

## Standards

Accepted blocking P2: `scripts/test-owner-staged-factor.js` declares a fixed
45000ms clock and expects that RFC code window, but omits `totpPreparationMs`.
Current Auth defaults preparation to20000ms while only15000ms remain. Waiting
cannot advance the fixed clock, so completion fails before factor input. Policy
0042 requires deterministic fixtures with declared inputs. The coordinator
verified the exact failed command and current source before adjudication.

Accepted nonblocking maintenance observation: repeated receiving/binding/result
shapes across owner continuation modules. This possible duplication was already
recorded by the earlier review. Distinct recovery contracts remain intact; no
cross-module refactor belongs in this bounded pass. It adds no new delivery gate.
No additional hard documented-standard breach was accepted.

## Spec

Accepted blocking P2: the same fixture failure prevents its public continuation
from demonstrating that approval and delivery permit the intended private use.
The contract's primary composition seam requires TOTP account/target/policy and
original-attempt binding. This is one shared finding across the two axes, not two
product defects. No other missing, incorrect or unrequested core behavior was
accepted. Existing installed/provider evidence remains independently qualified.

## Single bounded TDD remediation

Public seam: `createOwnerTotpContinuation.advance()` through real Auth authority,
SDK, encrypted synthetic KeePass source and enrolled native receiver, with
synthetic desktop/provider ports. It checks staged-factor approval, private input,
activation, exact account, lost-ack restart without replay, and explicit ambiguous
factor choice. It does not qualify live provider performance or desktop capture.

The existing test reproduces RED unchanged: waiting instead of completed at
line67, before factor input. Minimal repair explicitly sets
`materials.totpPreparationMs:0` beside its existing `minimumRemainingMs:0`, with
a comment declaring the fixed-window synthetic desktop assumption. Production
Auth defaults, guards, source data and runtime configuration are unchanged.
No implementation-mirroring test or speculative abstraction is added.

From the Agent Browser root, RED and GREEN use the identical command:

```sh
AUTH_RECEIPTS_SOURCE=/home/ecochran76/workspace.local/auth-receipts-browser449 AUTH_RECEIPTS_SDK_SOURCE=/home/ecochran76/workspace.local/auth-receipts-browser449 RECEIPTS_SOURCE=/home/ecochran76/workspace.local/receipts-issue106 node scripts/test-owner-staged-factor.js
```

Reviewed producer source: `b5bd66ac910c3e6a07f79cc8962cb14b67a77290`.
Parent source: `d6e07227e18b4ad9c866c6fe24e64bdcecdde294`.
Red log: `/tmp/p295-final-staged-red.log`. Green log: `/tmp/p295-final-staged-green.log`, exit0; all existing staged-factor
scenarios pass. Full owner-auth source composition passes (exit0):
`AUTH_RECEIPTS_SOURCE=<reviewed Auth source> AUTH_RECEIPTS_SDK_SOURCE=<same source> RECEIPTS_SOURCE=<reviewed parent source> pnpm test:owner-auth-source`.
It covers all five scripts: owner login, recovery, TOTP, number matching and
staged factors. Log: `/tmp/p295-final-owner-source.log`.
No hosted CI is dispatched; local Node24 Linux/WSL checks are the validation
environment. No target-platform or installed-runtime requalification is implied.

Fresh canonical main advanced to `fcb0e33cb5c6ac42041f8cdd3a7d97e9b769d240`
through independent retention PR500 during this review. Its Rust/runtime changes
are not findings remediation and do not alter this fixture or its source closure.
The frozen review head remains unchanged; integration will preserve that merged
work without restarting the broad review.

Accepted findings: Standards1blockingP2 and1nonblocking judgement; Spec1blockingP2
(the same fixture issue). The one bounded remediation is complete; focused and aggregate verification pass.
Policy wiring11/11, changed documentation links, JavaScript syntax and patch
hygiene pass. The prior documentation production build passed with unchanged
docs application source; integration documentation verification remains scoped
separately. No Rust source is changed by this remediation.
