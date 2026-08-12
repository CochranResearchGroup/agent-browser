# P110 PoC 4 Browser-External Prompt Perception Source Acceptance

Date: 2026-08-12

Status: SOURCE ACCEPTED

Implementation: `54cbc9e3`

Remediation: `7391409b`

## Outcome

The canonical read-only `desktop_prompt_observe` action proves one exact
repository fixture claim: a synthetic prompt matches the bound desktop
composite while the independently rendered page image and normalized DOM
manifest contain no prompt match. Configured production dispatch has no
provider and fails before action-dispatch effects.

The observation keeps detection, page visibility, external classification,
handling outcome, and no-effect operator intervention separate. It does not
create or resolve service challenge records and never authorizes input.

## Audit And Remediation

The single fresh audit found six blockers. The remediation:

1. makes `sessionName` the sole MCP daemon selector;
2. resolves provider absence before broadcast, CDP draining, policy reload,
   confirmation, or command dispatch;
3. derives page-viewport bytes from the decoded desktop frame and binds the
   derived hash plus browser and viewport geometry;
4. pins all manifest projections, the corpus projection, and canonical page,
   DOM, desktop, viewport, detector, observation, visualization, and paired
   receipt hashes, with malformed and adversarial tests;
5. derives shared MCP schema field counts from the canonical role ledger;
6. removes unreachable successful `indeterminate` page visibility.

No second broad audit was run.

## Verification

- canonical golden evidence: 1 passed;
- focused `desktop_prompt`: 27 passed;
- focused locator: 10 passed;
- focused capture: 26 passed;
- focused interaction: 17 passed;
- formatting and strict Clippy: passed;
- full service client: passed;
- API/MCP parity: passed for 100 service-request actions;
- no-launch service contracts: passed;
- docs production build and diff checks: passed.

## Boundary

No live browser, CDP screenshot, display, RDP, Guacamole, extension, native
prompt, credential manager, account, authentication flow, challenge, external
process, network provider, or input was exercised. This is fixture-only source
acceptance, not installed, live, real-prompt, challenge, authentication, or
release acceptance.

The next action is to write Plan 0110-5 before PoC 5 implementation.
