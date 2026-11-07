# Scoped settled retirement publication

Product lane: PL-BUGFIX
Work item: 570
Branch: fix/p321-retirement-publication
Base: 0b9575e035f06535e8e3ab9ee9c5a699709ad998
State: SOURCE_QUALIFIED

Continue in the existing owning checkout. #567/PR569 qualified historical owner
inventory independently. One source writer; no competing production candidate.
The shared-profile retention interview stays paused with its patch preserved
privately. Mailroot retains all provider and private-input work.

A requested profile can remain fenced after its physical close was positively
settled, publication failed, and its logical scope subsequently ended. Recover
only the requested profile's original transaction, never unrelated profile
retirements. Require the frozen scope to qualify unchanged-state publication,
exact original Native fence readback and atomic store publication before Native
completion. Unknown effects, changed identities and unavailable provider proof
remain unresolved. Never repeat a physical close, erase a record or grant a new
owner from historical evidence.

First reproduce through the actual host and SQLite with a settled unpublished
retirement and ended logical rows. Show same-profile continuation with exactly
one original close per browser and untouched unrelated retirement. Validate
negative effect/identity controls, affected CLI/model regressions, format and
strict workspace Clippy locally. No hosted CI. A later preserving installation
and actual reviewed workflow acceptance are separate coordinated gates; no
runtime reset, manual state edit, lock removal or private-input replay.

Validation: exact launch-readback RED before repair; 72 CLI retirement tests
passed (one existing ignored), all 416 service-model tests passed, format and
strict workspace Clippy passed without warnings. Policy wiring (11 cases),
changed documentation links, handoff documentation and docs build (44 pages)
passed. The existing docs lockfile-root warning remains. See the adjacent
validation receipt. Source qualification does not establish installation or
provider workflow acceptance.
