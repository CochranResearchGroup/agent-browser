# Notes Policy Review

Date: 2026-04-16

## Scope

This note records what the current repo-local policy says about durable notes,
especially the `docs/dev/notes/` surface, and what remains implicit or
ambiguous.

## Sources Reviewed

- `AGENTS.md`
- `docs/dev/policies/0001-policy-management.md`
- `docs/dev/policies/0002-policy-upgrade-management.md`
- `docs/dev/policies/0003-policy-adoption-feedback-loop.md`
- `docs/dev/policies/0009-turn-closeout.md`
- `docs/dev/policies/0010-validation-and-handoff.md`

## Findings

### 1. `docs/dev/notes/` is an approved durable continuity surface

The strongest direct guidance appears in
`docs/dev/policies/0003-policy-adoption-feedback-loop.md`. That policy says
dated adoption feedback should preferably be stored in the repo's normal
durable continuity surface, and explicitly lists `docs/dev/notes/` as one of
the preferred locations.

Practical implication: storing important workflow findings in
`docs/dev/notes/` is policy-aligned and should not be treated as an ad hoc
exception.

### 2. Notes are intended to preserve rationale that should not live only in chat

`docs/dev/policies/0002-policy-upgrade-management.md` and
`docs/dev/policies/0003-policy-adoption-feedback-loop.md` both require durable
recording when rationale, friction, or upgrade decisions would otherwise be
lost. `0003` explicitly says not to leave important lessons only in chat
history, commit messages, or maintainer memory.

Practical implication: if an agent discovers a workflow constraint, policy
friction, or adoption lesson that matters later, a dated note is the expected
repo behavior.

### 3. The repo has note usage guidance, but not a dedicated local notes format policy

There is no standalone local module in `docs/dev/policies/` that defines note
naming, lifecycle, required sections, retention rules, or authority ordering
between notes and other documentation surfaces. The current guidance is enough
to justify creating notes, but not enough to standardize them tightly.

Practical implication: note structure currently depends on local precedent and
operator judgment rather than a canonical repo rule.

### 4. `AGENTS.md` does not call out `0002` and `0003` in the policy entry list

`AGENTS.md` tells agents to read all files under `docs/dev/policies/`, but the
explicit bullet list in the policy entry section omits
`0002-policy-upgrade-management.md` and
`0003-policy-adoption-feedback-loop.md` even though both files exist and are
relevant to note handling.

Practical implication: an agent that follows only the visible bullet list, and
not the directory itself, could miss the repo's clearest note-related policy.

### 5. Validation policy applies to notes that capture live operational findings

`docs/dev/policies/0010-validation-and-handoff.md` requires concrete pass/fail
evidence and says to record whether manual smoke or live validation was run and
what it proved. For operational notes, that means a useful note should include
the commands run, outcomes observed, and any residual risk.

Practical implication: a note that records live browser behavior should include
evidence, not just conclusions.

## Recommended Working Rule

Until the repo adds a dedicated notes policy, treat `docs/dev/notes/` as the
default place for dated, durable workflow findings that:

- would be costly to rediscover
- are grounded in live validation, upgrade review, or policy friction
- are not yet appropriate to promote into canonical user-facing documentation

Each note should at minimum include:

- date
- scope
- source material reviewed or commands run
- concrete finding
- evidence
- practical guidance or implication

## Open Gaps

- No local rule defines when a note should be promoted into README, docs site,
  or policy.
- No local convention defines note filenames beyond existing precedent.
- No local rule defines whether notes are canonical, advisory, or temporary.
- No local archival or cleanup rule exists for superseded notes.

## Conclusion

The repo clearly permits and encourages durable notes in `docs/dev/notes/` for
continuity and policy feedback. The main gap is not whether notes are allowed,
but that the repo does not yet define a precise contract for note format,
authority, and lifecycle.
