# P288: Workstation writer custody

State: OPEN
Product lane: PL-BUGFIX
Owner: ecochran76
Work item: CochranResearchGroup/agent-browser#414
Branch: fix/workstation-writer-custody
Target: main
Baseline: dc5d1f3ee19146bb1ab51b6d9b1d0771e00e7075
Consolidation: required

## Consolidated batch

Repair PR406 review F1 stale-PID refusal, F2 shutdown-preview contention and S1 stale guidance. The operator controls valid requests; real concurrent writers still serialize. Keep the existing CONTEXT.md domain authority. Do not introduce another browser lifecycle or marker permission gate. No production publication, foreign cleanup, browser shutdown or formal release.

## Delivery sequence and budget

Use one red/green vertical slice at a time through existing public CLI fixtures. First prove a retained lock naming an unrelated live PID cannot block takeover, then prove a real executing writer serializes competing effects and appears in the full-shutdown preview. Reuse the qualified archived baseline binary for red evidence. At most three implementation cycles and two qualification builds before reassessment; checkpoint after each semantic green. Run focused workstation tests, required fmt and strict workspace Clippy, affected installer/takeover fixtures and documentation checks before integration through a linked PR. Reuse unchanged evidence under policy0042.

## Worker assignments

One primary, no delegation. Own workstation writer coordination and takeover/replacement consumers, their existing CLI fixtures, installer-help paragraphs and bounded planning/evidence. P283 owns mode help and P285 owns dashboard routing/help. Authentication retains its dirty source and credential guidance. Reconciled inventory: canonical authentication, active dashboard/mode/duplicate peers and clean retained enrollment/profile-proof checkouts. Preserve every peer. This dedicated primary is bound to issue414 and removed after clean integration; retained directories do not become active task ownership.

## Evidence and exit

Seam 1: public recover-host dry-run/apply proves stale metadata is advisory and the exact selected host can transfer; it does not qualify browser continuity or production.
Seam 2: public installer and full-shutdown dry-run prove real writer exclusion and truthful contention without shutdown effects; isolated fixture systemctl stands in for the OS service boundary.
Record exact red and green commands, source/binary identity and independent fixture process readback in the owning evidence note. Exit when both defects and stale guidance are repaired, affected gates pass, PR is integrated and own checkout has a governed closeout. Production installation remains separate.
