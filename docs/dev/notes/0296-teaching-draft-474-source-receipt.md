# Teaching draft source checkpoint for issue 474

Product lane: PL-RECIPES
Owning work item: [Issue 474](https://github.com/CochranResearchGroup/agent-browser/issues/474)
Disposition: active-input

The existing SiteGuide candidate JSON model now carries inert teaching knowledge.
The ordinary agent reviews video and selected narration, authors a substantive
first draft, stages a new owned candidate file and inspects it with the existing
public `browser_command` playbook `inspect_guide` operation. No procedure registry,
model provider, heuristic website generator, publisher or execution engine is
added. `AGENT_BROWSER_SITEGUIDE_DIRECTORY` retains its existing meaning.

## Fixed evidence and actual agent extraction

The owned synthetic Daily sales video and four source frames live in
`scripts/fixtures/teaching-report/`. `source.json` preserves the video/frame
hashes, explicit synthetic provenance and an independent known-result oracle.
The video SHA-256 is
`6d4faea1e56304b835ad00dea20b7aac4c1d0ffbdc6daeed30c5665c52c8bfc1`.
FFprobe reports VP8 video, Opus audio and12.008seconds. Audio is a reference tone,
not spoken narration or an ASR transcript. Visible captions supply teaching intent.

The implementing agent actually viewed all four rendered frames and a decoded
video frame at4seconds, then prepared `draft.json` with ordinary multimodal
reasoning. It extracted the Daily sales choice, May1–May31 date examples, checked
refunds option, result title/range/USD, boundary rows and export marker. Four
routines compose selection, parameter setup, generation/verification and artifact
export/verification. Later dates are parameters. The visible checked refunds box
is observed; the business policy for future refunds choices remains unresolved.
Account/company authentication and executable locator validity are not invented.

The concrete draft includes source-linked preconditions, branches, expected
outcomes, independent verification, lost-reply recovery and human-assistance needs.
Its unresolved questions do not ask the operator to repeat visible example dates
or the checkbox state. Suggested interpretations remain unaccepted proposals.

## Public seams and validation

- The JSON-RPC `tools/call` seam reads the existing configured candidate directory.
  The source red was `siteguide_invalid` for the teaching extension. Its meaningful
  assertions cover date/refunds parameters, four routines/composition, attribution,
  unresolved policy and unchanged peer/task/store/files without a transport socket.
  The first loader-success attempt failed fixture cleanup on read-only SQLite
  migration archives; the existing owner-writable fixture cleanup pattern fixed
  that failure before accepting green. A further red returned the unrelated
  login-contract error on execution; explicit `teaching_draft_not_executable`
  refusal now preserves the draft boundary. Invented evidence references are
  rejected, and ordinary BILL SiteGuide inspection remains compatible.
- `node scripts/test-teaching-draft.js` independently checks fixed media hashes,
  known literal dates/choices/CSV rows, parameterization, attribution and public
  client command parity. Staging refuses an existing file and requires explicit
  candidate-only intent. The first red was the absent candidate helper.
- Required Rust focused tests, formatting and strict Clippy results are recorded
  with exact commands in the coordinator's execution ledger. An early compile
  failed on a shared sccache connection reset before tests; the bounded retry used
  the documented cache opt-out. No memory reserve or hard cap was relaxed.

Final owned-source checks pass:

```sh
scripts/ci/rust-tests.sh --focused teaching_draft_public_inspection
scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings
scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check
node scripts/test-teaching-draft.js
node scripts/test-teaching-evidence.js
pnpm --dir docs exec tsc --allowJs --checkJs --noEmit --module NodeNext --moduleResolution NodeNext --target ES2022 --types node ../packages/client/src/playbooks.js ../packages/client/src/teaching-draft.d.ts
```

The compiling checks used `AGENT_BROWSER_CARGO_CACHE=off`,
`AGENT_BROWSER_CARGO_MEMORY_CLAIM_KIB=8388608`,
`AGENT_BROWSER_CARGO_MEMORY_MAX=8G` and
`AGENT_BROWSER_CARGO_MEMORY_HIGH=7G`, retaining the ordinary16GiB host reserve.
Formatting used a matching1GiB claim/hard cap and768MiB high watermark.
Patch hygiene passes. Integration-wide documentation/shared guidance checks
remain with the coordinator after applying the supplied pointers.

The fixture uses only owned synthetic browser rendering and local media encoding.
Its first generation failed because the fixture page used conflicting global
names; that attempt is not acceptance evidence. The corrected fixture completed,
closed its browser and passed a fresh process readback without a fixture Chrome
or FFmpeg tree. No account, microphone, private media, installed service or
production runtime was touched.

## Qualification boundary

Schema validation establishes shape/reference integrity, not the truth of model
claims. Public inspection does not establish source access, review approval,
authentication, successful execution or publication. Drafts cannot invoke
executable playbook routines. Real spoken narration, microphone/ASR quality,
operator review and a fresh authenticated portal repeat remain separate gates.
The fixed caption-led synthetic case does not substitute for those qualifications.

Memory disposition: forbidden. Parent explicitly excluded Graphiti writes for
this worker. No memory write was attempted.
