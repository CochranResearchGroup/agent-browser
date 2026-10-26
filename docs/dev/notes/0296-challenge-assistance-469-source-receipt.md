# Native challenge assistance qualification for 469

Product lane: PL-CHALLENGE
Owning work item: https://github.com/CochranResearchGroup/agent-browser/issues/469
Disposition: active-input

## Seams

The public Challenge Control classification API consumes bounded Native OCR
evidence without authority. The existing Native task adapter adds optional
classifyChallenges to native_capture, asking the owning Remote View OCR endpoint
for one current fenced frame and omitting raw recognized page text. Public
client composition reuses task briefing, opaque handoff, control status and
continuation. Separate application verifiers gate clearance and site admission.

## Source red and green

- Registered hCaptcha public classifier: Unknown versus Checkbox red, then green.
- Declared13-case native corpus: passive-check case red, then all cases green.
- Existing NativeOwner adapter seam: native_capture classifyChallenges returned
  no classification (Null versus checkbox), then same-frame classification green.
- Public client composition: prepare returned no assistance, then exact handoff,
  explicit control, independent site denial and original-task continuation green.
- Actual Native admission exposed a missing control epoch. The adapter regression
  returned Null versus the required Native epoch, then passed after reading the
  existing owner's exact desktop/generation status. Native and cooperative
  browser epochs remain separate.
- Owned manual-login display context returned environ_unreadable despite a
  verified owned launch, then passed using the retained trusted launch display.
  Changed original context and missing owned display still refuse. The adopted
  path retains its original process-environment fallback.

The NativeOwner contract fixture does not establish real viewer delivery or OCR
accuracy. The pure corpus is synthetic English OCR text, not real provider-family
qualification. No classification grants an automated attempt or proves site access.

## Validation

- Challenge Control crate: 60 tests passed, including the declared 13-case corpus.
- Native control integration: 14 tests passed, including same-frame OCR redaction
  and exact Native control admission.
- Manual-login focused unit tests: 2 passed, including debugging-argument refusal
  and unavailable process-environment/context regression.
- Workspace format check and strict Clippy passed through cargo-safe. Builds used
  the reviewed 8 GiB claim/cap without reducing the host reserve.
- Challenge client tests, declaration type check and complete service-client
  suite passed. Fixture JavaScript syntax and Python compilation passed.

## Actual Native qualification

The fresh owned run 015 qualifies both paths through the public workflow and the
actual hosted consumer's Native viewer. It returned DIAGNOSTIC_ACCEPT in 189.15
seconds, with no adapter or cleanup error. This is the bounded challenge/input
lane, not the full P279 matrix or its performance qualification.

| Path | Evidence | Limit |
| --- | --- | --- |
| App-owned attachment | Real Native OCR classifies the unsupported image instructions; actual viewer input and explicit takeover/return continue the original task after independent HTTPS clearance and protected-access checks. | App-owned CDP is explicit fixture staging and independent peer observation. Product perception/control does not use it. |
| Managed manual_login | A distinct actual browser has no debugging switches or opened CDP connection. The same Native classification, viewer input, takeover/return and independent HTTPS checks pass. | The empty synthetic profile is prepared through public blank launch/close. Existing manual-login fixture prior art models its first-run setup marker; this is not actual legal consent, login or authentication evidence. |
| Identity and continuity | Each path reuses its original opaque handoff and exact task/profile/browser/tab target. Exactly one completion request occurs per path. Original app process, draft, storage, cookie and peer target remain intact; the Native peer remains intact. | The larger browser task is not declared complete. |

The managed path uses only the current fixture's Native-provided X11 cookie-file
locator in its private owner environment. This qualifies the one-owned-desktop
Linux fixture, not production multi-desktop focus or another operating system.

The Agent Browser candidate was built from clean
f248c6359028a2612173aebbefdb61266c231e83, SHA-256
792822a0bdce65821c181f21e85a402f7ce8b595feca88a77663f755e31f898a.
The frozen harness is 269f318bdbca82d053338828d9dd4bb8ccf18564.
The Native fixture source is dff90d0cfa8b76edaee7bdc588afc32f090aeea9;
the actual producer binary is
eb0639e7ce7b6271b8d78f105eb1d6b0acc77903f5244e3f6c600d86ddd05ed7.
Producer source/binary equivalence is unclaimed. The pinned consumer is
3d258be0e632668a8e1707dbae646af42d177616. Private Chrome for Testing 153.0.8010.12
is 8c599d43aec53f2460a31ae2f4af6bd863f8258b34ff519564bc5d4726bfaa1e.

Private evidence lives under
`~/.local/state/agent-browser/spec466-469-native/`. Relevant run 015 artifacts are
`agent/challenge469-acceptance.json`,
`agent/challenge469-no-cdp-acceptance.json`,
`agent/challenge469-no-cdp-modeled-first-run.json`, continuity receipts,
`agent/cleanup.json`, the source/workload identity and aggregate receipt.
No auth values, raw OCR text or operator state are copied into this note.

## Cleanup coverage correction

Earlier failed diagnostics remain failed. In particular, run 006 lost child
cleanup during an in-flight viewer-route failure. Its parent Native/container
census did not prove absence of an Agent Browser child daemon. The exact daemon
was later identified by its private fixture home and executable, stopped through
its verified pidfd and independently observed absent. Its original unqualified
child receipt remains intact. Run 013 was aborted after a failed in-place pin copy
and is unqualified; subsequent qualification used an immutable filename with
source, hash and version readback before startup.

The final independent `all-namespaces-resource-census.json` covers 001 through 015:
348 positively owned recorded process generations absent, no unknown recorded
generation, no fresh private HOME/profile/argument/cgroup/executable matches,
and no owned Docker containers, networks, volumes or Native units/unit files.
Verified foreign baseline/denied ancestry is preserved and excluded from owned
custody. The initial audit with those foreign references mixed in is retained
separately before provenance reconciliation. The X11 census checks 26 configured
display bindings with no lock, pathname socket or abstract socket residue.
Private receipts, pins and fixture build-image tags remain retained artifacts;
no image-absence claim is made.

Existing P279 fixture code is evidence infrastructure only. Its old authority and
acceptance do not transfer to 469. This work publishes no production runtime,
provider, ingress, authentication or release change. Parent integration owns the
shared help/README/skill/context pointers and the final ticket disposition.

Memory disposition: forbidden. Parent delegated scope excludes Graphiti writes.
