# Prepare a teaching draft

Use this when an operator supplies a teaching video or a selected teaching
evidence bundle. Produce a useful first draft before asking clarification
questions. Use your normal video/image and language reasoning. There is no
website-specific generator, default procedure template or new model endpoint.

## Read the evidence

1. Use the teaching-evidence helper to inspect the exact authorized recording,
   selected isolated narration, context revision and original Speech operations.
   Recheck Speech Saved provenance before transcription and text release. Refuse
   a stale, revoked, relabeled or substituted source. Keep private artifacts and
   transcripts in the authorized workspace; do not copy real media into Git.
2. Inspect representative video frames and transitions, not only a transcript.
   Read visible labels, selected options, date/number/unit values, result states
   and any visible output verification. Record each frame's artifact hash and
   recording-local time interval. Watch additional frames where a transition is
   ambiguous. Never treat a transport input event as application acknowledgement.
3. Read the explicitly selected narration transcript. Retain its original source
   revision and absolute normalized-range times. If video/transcript alignment is
   unverified, keep the two time domains separate; do not fabricate an offset.
   For narration evidence, use the original Speech operation ID as its artifact
   reference and the bound source revision as its hash. Its start/end remain
   absolute Speech times. Retain the original teaching capsule separately for
   selected-track/context bindings; do not replace it with a compact summary.
   A source label does not establish speaker identity. Captions are observed
   video evidence, not speech. Missing audio is not an invented transcript.
4. Mark capture gaps, source changes and uncertain critical dates/numbers/units.
   Stop the owning recorder before protected entry; confirm its actual shutdown
   and finalization, then use a distinct resumed recording with an explicit gap.
   A consumer privacy hold does not suppress independent recordings. Never put
   credentials, codes, raw protected input or private credential capabilities in
   evidence summaries, guide parameters or routines.

## Synthesize the first candidate

Use the existing `agent-browser.siteguide.v1` object. Add its inert `teaching`
section with schema `agent-browser.teaching-draft.v1` and state `draft`. The
candidate's guide ID must be new; preserve existing canonical and accepted files.

Prepare the following from what the evidence actually shows:

- A SiteGuide for recognizing meaningful page states and named controls.
  Visible labels are observed facts. A role/accessible-name or CSS locator
  inferred from a picture remains inferred until checked on the actual page.
- Parameters separating the demonstration's examples from later caller inputs.
  Include a type, example, meaning and evidence references. For a date-range
  report, preserve the recorded dates while identifying the future start/end
  parameters; do not silently hardcode the example period.
- Reusable routines with inputs, preconditions, meaningful steps, conditional
  branches, consequential expected outcomes, verification and recovery. Use
  existing Auth, cooperative control and original-operation recovery when needed.
  A filled field, click, download event or plausible filename does not prove the
  requested report was produced correctly.
- A composed playbook naming the routines in useful order and stating what
  independently establishes completion. Do not execute it while drafting.
- Claims explicitly tagged `observed`, `inferred` or `unknown`, with links to
  pertinent video/narration/input/result evidence. Do not promote a convenient
  inference into a demonstrated fact.
- A short unresolved list only for material remaining ambiguities. Give the
  evidence, question and recommended interpretation; a recommendation is not an
  operator answer. Do not ask again for clearly visible labels, dates or choices.

The fixed synthetic Daily sales example in
`scripts/fixtures/teaching-report/draft.json` illustrates a complete draft. It is
not a general report template. Its visible checked refunds option is settled
observation, while the policy for future runs remains unknown. Its caption-led
A/V contains a reference tone, not spoken narration or an ASR result. Real
operator narration/recognition is independently qualified.

## Stage and inspect

Use `writeTeachingDraftCandidate` only with an explicitly owned candidate
directory and `candidateOnly: true`. It creates a new file without overwriting
an existing guide. Configure an ordinary candidate client with that same
`AGENT_BROWSER_SITEGUIDE_DIRECTORY`, as in the existing SiteGuide authoring
workflow. This is the existing candidate file model, not a second registry.

Inspect through `createPlaybookClient(...).inspectTeachingDraft(taskId, guideId)`
or the existing public MCP command:

```json
{"name":"browser_command","arguments":{"taskId":"<exact-task>","action":"playbook","params":{"operation":"inspect_guide","guideId":"<candidate-guide-id>"}}}
```

Confirm the readback contains the substantive states, parameters, routines,
playbook, evidence and unresolved items you prepared. A successful readback is
not source truth, execution approval, publication, authentication or an outcome
receipt. Teaching drafts cannot invoke executable playbook routines. The next
work is focused clarification and explicit review/versioning; preserve the
accepted version until a new candidate is actually reviewed.

For operator corrections and immutable revision/review handling, continue with
[teaching review](teaching-review.md). Initial staging accepts fresh draft
knowledge; reserved lineage/review/validation metadata uses the review workflow.
