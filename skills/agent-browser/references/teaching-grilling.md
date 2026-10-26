# Clarify a candidate through focused grilling

Start with a useful inspected SiteGuide draft and its selected media, not a blank
questionnaire. Use normal agent reasoning to compare observed actions and stated
intent against inferred rules, input variability, completion, exceptions and
recovery. Preserve facts already visible or stated. In the Daily sales example,
report name, May dates, USD and checked refunds are known; future refunds policy,
timezone/boundaries, file delivery and unshown recovery may remain unknown.
Recorded example rows never become future-run invariants.

Add genuine material ambiguities to the existing `teaching.unresolved` through
`review.save`. Cite exact evidence IDs/times or conflicting passages and the
pertinent draft step. Explain a recommended interpretation only when justified;
otherwise state what is missing. Recommendations never become decisions by default.

Use `createTeachingGrillingClient({review, readOperatorAnswer})` from
`@agent-browser/client/teaching-grilling`. `prepare` selects existing unresolved
IDs with a category, existing `routineId:index` step references and dependencies
on earlier questions. It persists the plan in the same immutable SiteGuide. Ask
only `open().round`: independent ready questions now, dependent questions after
their prerequisites settle. Do not reask facts or answers already known.

Only the receiving application's trusted authenticated answer reader may supply
an existing actual answer receipt. It must bind question ID/hash, task/root,
current guide/raw/content hashes and attributable actor/source-event references.
Do not pass a model approval callback, script recommendation or actor label as
operator authority. No stock Dashboard/Native interview wiring is implied.

Read the actual answer and prepare useful changes in a copy of the current guide.
Call `answer` with that copy, question ID, answer reference, expected hash and a
saved operation ID. Source observations and unrelated unresolved questions stay
intact; answer history and remaining uncertainty survive the new candidate
revision. Clarification does not approve/promote/execute the procedure.

After a lost reply, recover by operation ID and reopen the original revision.
Never discard an answer or fabricate a second consent event. Reopen verifies
stored receipts with their original authority. Missing/changed authority marks
answers unverified and blocks rounds without erasing or reasking settled answers.
Full receiving-application contract and examples: [teaching grilling](../../../docs/src/app/teaching-grilling/page.mdx).
