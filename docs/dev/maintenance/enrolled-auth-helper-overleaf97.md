# Enrolled Auth Receipts helper for unattended consumers

State: OPEN
Product lane: PL-AUTH
Consolidation: required
Work item: [Agent Browser580](https://github.com/CochranResearchGroup/agent-browser/issues/580)
Branch: auth/enrolled-helper-overleaf97
Base: bfdf162b896aec462e8341a57bfcec89dc8fefad

Owner: ecochran76

## Current State

The supported Codex helper requires a genuine current caller. Overleaf97's
background worker has none. Auth Receipts290 provides an explicit enrolled
consumer SDK factory; copying a Codex thread ID or falling back to its discovery
would misstate authority. This slice adds only the Agent Browser client adapter.
No installed runtime, enrollment, credential request/use or browser effects.

## Consolidated batch

Freeze `connectEnrolledAuthHelper({installationRoot?,discoveryFile,operationContext})`
with the provider contract: await
`createEnrolledAuthClient({discoveryFile,operationContext})`, returning
`{callerTaskId,call}`. Context contains canonical UUID operationId/recoveryId and
exact six-key taskId/browserId/profileId/tabId/origin/controlGeneration binding.
Auth validates consumer discovery and current authority. Preserve the existing
verified installation boundary and hosted Codex entry point; factor their common
private requestLogin/advanceLogin/followLogin/helper and capability/non-replay
handling. No connection contents or credentials enter this adapter's result.

Source scope is hosted-auth-helper.js/declarations, its public SDK fixtures/types
and narrow API/guidance parity. No CLI feature or auth transport is invented.
Related source: [Auth290](https://github.com/CochranResearchGroup/auth-receipts/issues/290).
Consumer integration and actual acceptance stay with
[Overleaf97](https://github.com/CochranResearchGroup/overleaf-mcp/issues/97).

## Delivery sequence and budget

One missing-export public fixture RED first, implement the configured enrolled
entry point, then cover refusal/unknown-attempt behavior and existing hosted
compatibility. Run affected provider-free SDK/types/guidance checks and the
changed-surface documentation/planning gates locally. No hosted CI or Rust build
unless an executable Rust surface changes; source-only qualification does not
install the Auth provider or enroll a caller. Preserve first failures and report
exact checkpoint/remote custody. One bounded source delivery; no broad review or
findings-remediation counter is consumed.

## Worker assignments

This worker owns Agent Browser580 and its PL-AUTH primary worktree. The canonical
foreign CONTEXT.md remains untouched. Parallel579 owns retained browser recovery;
active-catalog top entries and distinct shared guidance paragraphs integrate
serially by intent. Auth290's worker owns producer SDK/attestation; Overleaf root
owns readonly attestation, consumer integration and live acceptance. Preserve all
retained foreign checkouts; the explicit directed new checkout is not automatic
portfolio cleanup.

## Evidence and exit

Public module fixture must succeed without CODEX_THREAD_ID using an enrolled
provider and exact operation context. It must refuse wrong/missing discovery,
unsafe installation, missing/incompatible factory and owner-policy failures;
unknown private attempts cannot be repeated. Existing hosted helper/follow tests
remain unchanged and green. Declare source readiness only after affected checks,
commit and remote readback. Runtime installation/enrollment and actual private
input remain NOT_RUN and cannot be inferred from synthetic qualification.

## Source checkpoint qualification

The public module fixture failed first because the enrolled export was absent,
then passed against an explicitly enrolled asynchronous provider with no Codex
caller. The common owner helper retains hosted behavior. Local passing checks:
`pnpm test:hosted-auth-helper` (hosted loader, hosted follow, enrolled fixture and
both TypeScript callsites), `pnpm test:validation-selection` (including exact
enrolled fixture classification and documentation links), `pnpm test:policy-wiring`
(11 policy wiring cases), explicit CheckJS for hosted-auth-helper.js and
`git diff --check`. The selector's initially unresolved new fixtures were
registered with exact paths after a regression fixture failed; classifier and
package-script changes still deliberately select broad integration qualification.
A mistakenly named standalone doc-link command was unavailable; the actual
documentation-link test passes in the validation-selection aggregate.

The one Auth SDK CLI help sentence requires combined Rustfmt/strict Clippy gates
on the serially integrated source with579. Those gates, the remaining selector
integration requirements, Auth290's actual producer integration, runtime install,
enrollment and private input are pending; synthetic source checks do not establish
them. No broad review or findings-remediation counter was consumed.
