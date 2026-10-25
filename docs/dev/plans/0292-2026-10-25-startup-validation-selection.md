# P292: Select mounted startup validation

Status: CLOSED
Product lane: PL-BUGFIX
Owner: ecochran76
Work item: https://github.com/CochranResearchGroup/agent-browser/issues/437
Primary: agent-browser-dashboard, fix/validation-startup-437
Baseline: ba5cda65cc47073e0b53bd6af3b4d61fecf4bf51

## Outcome and scope

User-approved implementation of P291 retro recommendation1. The public selector CLI must recommend the existing coordinator, mounted workspace-startup and selected-context checks for polling/cache/hook and related test/fixture changes, including script-only diffs. Reuse the retained primary serially; preserve all foreign checkouts and pinned candidates. Own only selector recommendations, existing selector CLI regressions and local-validation guidance. Preserve classifier tiers and unrelated recommendations. No CI activation, runtime/build publication, CodeGraph changes or acceptance-helper work.

## Seams and qualification

The public selector CLI, exercised through temporary Git repositories, proves the selected commands for actual changed paths; it does not prove dashboard rendering. The existing mounted startup suite verifies real hook/cache behavior separately. Use one red-green slice, then expand coverage for source/test/fixture paths, deduplication and an unrelated negative control. Run test:validation-selection and the existing startup suite, review Standards and Spec against this baseline, then integrate through a linked PR. No native build is needed for recommendation tooling.

## Qualification and review

Red: `node scripts/test-validation-selector-cli.js` failed because a polling-coordinator diff did not recommend the mounted startup suite. Green: the same command passes after the additive mapping. The public-CLI matrix covers four source paths, three focused test scripts and the startup fixture, combined-change deduplication, check ordering before dashboard builds, and an unrelated dashboard negative control. Classifier tiers and existing recommendations remain unchanged.

`pnpm test:validation-selection` passes all six component checks. The mounted startup suite passes ready/unknown/failure/empty cases;18 observed fixture generations are absent. Coordinator and selected-context tests also pass. The Chrome executable is explicitly pinned for the mounted fixture. Standards review found no blocking documented-rule defect; Spec review found the approved source/test/fixture and no-runtime bounds satisfied. Exact packet audit, documentation links and patch hygiene pass after correcting this plan's status/owner metadata.

Integrate through a PR linked to437, then close the tracker item. Retain the primary and pinned candidates. This changes local validation recommendations only; no native build, installation or CI activation was performed.
