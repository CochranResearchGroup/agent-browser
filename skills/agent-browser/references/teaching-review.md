# Review and revise teaching candidates

Use the existing private SiteGuide candidate directory, public playbook client
and `@agent-browser/client/teaching-review`. The receiving application binds its
existing authenticated operator decision reader in trusted backend configuration.
Do not supply an approval callback, actor label or approved boolean as model
request data. No stock Dashboard/Native review wiring is implied.

1. Open the exact guide ID with `review.open({taskId, guideId})`. Inspect each
   step's action, explanation, expected result and source references. A result
   inherited from the routine is explicitly marked `scope: routine`. Preserve
   capture gaps, uncertainty, missing media and the original time domains.
2. Prepare direct corrections, removals or parameterization in a copy of
   `view.guide`. Keep original recording/task/hash/evidence records immutable;
   corrections belong in claims, steps, parameters and unresolved knowledge.
   Recorded May example rows are examples, not invariants for a future period.
3. Save with the exact `expectedSha256` and a durable new `operationId`. The
   client validates the edited inline SiteGuide through the public read-only
   `validate_teaching_candidate` operation, then creates a new immutable JSON
   version. It does not overwrite the accepted procedure or peer candidates.
4. Reopen the returned guide ID. Inspect the changed knowledge and parent/source
   lineage. A stale/concurrent edit is a typed conflict; never replace a peer
   merely to make your revision current. Changed content requires fresh review
   and repeat validation.
5. A proposed interpretation is not an answer. Opening the review or saving an
   edit is not approval. Only an existing deliberate operator decision, verified
   through the configured authenticated reader against task/root/version and
   content digests, can be recorded with `recordDecision`.
6. Keep `ordinaryReuseAllowed: false` until separately established review and
   fresh-repeat validation. This client never promotes or executes a procedure.
   An approved candidate still leaves proposed answers unadopted until the
   operator actually clarifies/corrects the knowledge.

Persist edit/review operation IDs before dispatch. After a lost reply, use
`lookupOperation` and reopen the original returned version; do not create a new
operation merely to discover whether the first save happened. A same-ID,
same-intent retry recovers the original immutable version. `not_observed` does
not prove an earlier caller stopped or grant retry authority. Never recreate an
operator approval silently.

Source access and procedure custody are separate. A source-owner check must
match exact recording ID and hash and independently check each evidence ID,
artifact and hash with its owner. Recording availability alone cannot qualify a
Speech transcript or derived frame; missing per-reference checks stay unchecked. Unavailable/expired/deleted media remains
non-replayable without deleting the retained accepted procedure. Explicit
operator acknowledgement is required for a new review without source playback.
Do not describe a stored hash or text summary as proof the source was watched.

The public validator checks shape/reference integrity only and always reports
`operatorReviewQualified: false`, `executionAllowed: false`. Static review labels
are not authenticated decisions. Reopening a recorded review without its trusted
authority reader reports it as unverified while preserving the original record.
