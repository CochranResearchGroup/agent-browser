# Challenge assistance source checkpoint for469

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

The NativeOwner contract fixture does not establish real viewer delivery or OCR
accuracy. The pure corpus is synthetic English OCR text, not real provider-family
qualification. No classification grants an automated attempt or proves site access.

## Native qualification

Pending the owned HTTPS/Native fixture. Producer source/binary, fixture inputs
and process/resource cleanup must be recorded separately. Existing P279 fixture
code is reused as evidence infrastructure only; its old execution authority and
old acceptance do not transfer to469.

Memory disposition: forbidden. Parent delegated scope excludes Graphiti writes.
