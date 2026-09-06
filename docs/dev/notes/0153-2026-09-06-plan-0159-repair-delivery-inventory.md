# Plan 0159 repair and delivery inventory

Date: 2026-09-06. Status: isolated candidate installed; production proposal
preparation incomplete. This inventory does not authorize whole-branch promotion.

Authority: [Plan 0159](../plans/0159-2026-09-05-client-recovery-logging-and-remote-view-remediation.md).
The historical [D01–D15 register](0152-2026-09-05-plan-0158-defect-register.json)
remains unchanged. The plan's checkpoints retain the scope of each proof and
every failed live attempt.

## Exact candidate and installed state

Candidate source: `7d6ac6dc850f4df5401eac9746222f1e1d143f06`.
Optimized `ci` binary SHA-256:
`9e6280975da3e1067d352e4237791b33df7d95ea0f5e11e810cbb69f04a23706`.
The frozen executable and build receipt are private under
`campaigns/p159/restore-bubble-candidate/`. That binary is installed only in
P158. Installer, exact doctor and three launch smokes passed. A newly launched
synthetic browser reports the suppression switch in its live command line and
accepts its original handle. The production binary remains
`4a92c42517e1441f5e30b6fcf52857123efa7eb8273a8b126fc504de966333f7`.

This optimized binary is not a final production release-profile candidate.
The installed production metadata does not identify its source commit, so this
document does not claim an exact source diff from production merely from a
generation label. Current production generation is
`0.28.0-4a92c42517e1-6121fd69672b`; retain its binary, support tree and unit
custody as rollback inputs.

The subsequent trusted-display keyboard-focus repair in Plan checkpoint 36 is
source-verified but is not part of this installed candidate. Its Guacamole
extension JAR and manifest must be included in the eventual delivery bundle;
installing a CLI binary alone does not deliver that repair.

## Repair composition and proof boundaries

All commits below are verified ancestors of the candidate. Ancestry establishes
composition history, not a new behavioral test or production-installation claim.

| Repair group | Source/dependencies | Existing outcome evidence and remaining limit |
| --- | --- | --- |
| D01/D02 original-handle recovery | `894076b4`, owner counterexamples `0c4c1c16` | Plan checkpoints 1 and 11: two original handles after host interruption; exact owner/target rejection before effects. No repeat of the accepted live cycle. |
| D04/D05 truthful denials | `5803203b`, `b6ee48f2`, `abc784c5`, `644c8a48` | Historical selected transport/projection joins remain scoped; rejection correctness is distinct from working-link acceptance. |
| D06/D08 journal and actor custody | `2fae5f5a`, `062727fb` | Two interruption seams and selected actor joins passed. Power loss and universal log coverage are unproven. |
| D07/D09 policy drain and advice | `54d1a8f6`, `31a5a348` | Historical closed-tab drain, retained peer, revoked grant and authenticated advice checks passed. |
| D10 host socket startup | `8164eac9` | Disposable startup proof and production compatibility restoration are distinct from source-fix delivery. |
| Repeated stream enable | `d46ddc71` | Checkpoints 10 and 14: real-listener identity plus two installed compatible requests; manifest unchanged. |
| Presentation frame, dispatch and client custody | `c1434e57`, `4e71eb03`, `6efa26f1` | Focused red/green and isolated checks. Retained frame behavior must be delivered with its backend contracts. |
| Backend primary ownership | `8f830c5c`, `40023d88`, `9b975aec` | Backend ownership requires the matching dashboard client, provider authentication/data-source split and ingress deadline. Do not deploy the frontend or backend half alone. |
| Primary terminal logging | `e0bccc38`, `b5faee54`, `3428a7a7` | Owner-to-endpoint occurrence linkage and typed guard causes verified at selected seams. External HAR does not prove response-body occurrence custody. |
| Route revalidation continuity | `13706923`, `3ad29c44`, `d0b5f298` | Exact owner/route guards, pending acquisition overlay and retained capacity belong together. Local combined transition passed; historical external primary failure is not erased. |
| Restore-bubble obstruction | `7d6ac6dc` | Focused launch regression and disposable native desktop comparison passed; installed process flag verified. External ordinary input/reopen remains a separate gate. |

The external producer also requires the input acknowledgment, geometry and
resized-sample corrections (`d75d79ca`, `9d825b1c`, `846ec620`, `80ef60ab`). They
are evidence tools, not substitutes for shipping runtime repairs. The independent
historical PNG oracle remains fixed. The client-helper takeover envelope finding
in checkpoint 29 remains a delivery risk; the verified local probe uses the valid
params-only form. It is not silently included as a completed repair.

## Production decision still to make concrete

Before requesting approval, finish ordinary external input/reopen acceptance,
review the complete proposed runtime/support composition and compatibility,
build the exact final release-profile artifact, and seal its support manifest
and selected validation evidence. A passing isolated binary does not identify
the full production update bundle by itself.

The production update must use the supported guarded installer transaction,
with its fresh runtime census, exact candidate generation and rollback custody.
Read-only `install transactions inspect` supplies the transaction ID, revision
and census digest required by guarded rollback. Do not invent those values now
or replace the supported transaction with a manual selector flip. Preserve the
current generation until candidate outcome checks and rollback retention are
accepted. No production client eviction is authorized by this plan.

The current-generation compatibility drop-in
`50-current-generation-socket-directory.conf` pins the old host socket directory.
Its current SHA-256 is
`aba367de7198cc1f284caee7bf93c9712ff019e0d9c7d517a6a0a02e26c29509`.
The concrete update must explicitly retire or supersede that workaround only
after proving the source startup fix for the new generation, and retain its
exact prior content for rollback. Do not remove it merely because source tests
pass. Deployment, override mutation and production outcome checks are pending.

Post-install outcome checks must cover selected-generation identity and doctor,
authorized client/original-handle usability, denial/actor/journal joins, correct
ordinary remote-view pixels and trusted input, same-URL reopen, and stable
rollback custody. Existing private or active production targets require the
appropriate authority and privacy boundary; synthetic proof does not authorize
capturing them. Separately finish or explicitly retain each fixture process
and Service-resource obligation.
