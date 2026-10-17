# Selected planning completeness evidence

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: [Plan 0247](../plans/0247-2026-10-17-planning-completeness.md), CochranResearchGroup/agent-browser#299
Related lanes: P246, P240

## Baseline and behavior

Baseline 0cf8e5bc8d057eb8b5856dcfceb856bf6b19c1a3. The legacy active-only audit
passes while reporting all 22 current plans as unclassified, including P245's
Status: COMPLETED. This does not constitute completeness qualification.
The lane auditor checks Git custody; the archive test has historical assertions.

The existing planning auditor now accepts exact repeatable lane selections.
Selected current/completed plans must be discoverable through startup links;
catalog and plan ownership/product lane agree; matching evidence notes must
have index links and bounded ownership metadata. Closed/cancelled outcomes do
not require current execution links. The legacy audit remains independent.

Initial red: the new disposable accepted/missing-index fixture failed because
no selected-lane check existed. After implementation it passes. Mutation tests
remove catalog ownership, note ownership and note-index links, mismatch status,
remove completed evidence, and move metadata into narrative body. Actual CLI
coverage verifies exit 0 for a complete fixture and exit 1 for a missing link.
The canonical P245 selection passes. The complete audit suite passes 67 tests,
including six selected-lane tests and actual CLI exit checks. Policy wiring and
the docs build pass. The initial independent batch retains a failure from this
packet's missing legacy Lane header; adding Lane: P247 makes the legacy audit
pass. No source logic changed after the 67-test pass; reuse those tests and the
docs-site build because the follow-up changes only planning evidence/status.
Changed-link validation covers seven Markdown files.

## Standards

Serial primary review of implementation facbef2df against base
0cf8e5bc8d057eb8b5856dcfceb856bf6b19c1a3 found no blocking code-standard issue.
The legacy Lane metadata omission was caught by validation and corrected.
The new option belongs to a developer audit script, not product CLI behavior;
its owning validation guide documents the contract and limitations.

## Spec

The issue299 bounded contract is satisfied by explicit selected-lane checks,
status-aware fixtures, existing-parser reuse and actual CLI exit checks.
No legacy migration, custody replacement or CI change was introduced.
No blocking Spec finding. The opt-in mode does not imply automatic enforcement.

Memory disposition will be recorded at publication closeout.

## Limits

Only selected lanes are covered; no historical migration or live forge/runtime
claim. Notes associate by plan serial prefix; links must be inline Markdown.
No product/browser/runtime change, installation or CI activation.
