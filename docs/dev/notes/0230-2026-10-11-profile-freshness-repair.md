# Profile freshness repair

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: P230 / #176
Related lanes: PL-AUTH

Access planning selected a persisted legacy per-service profile with two aliases, but readiness mutation revalidated unrelated allocation and rejected it. The generic response reported uncertain effect although durable profile state was unchanged. Freshness updates now preserve allocation, aliases and access policy without requiring allocation migration. Full profile registration retains its existing policy validation. Input checks emit typed `profile_freshness_invalid` before profile mutation; persistence errors keep their original uncertainty classification. Executable recourse inspects profiles before correcting the request.

Two regression tests went red before fixes and passed afterward. The legacy, valid per-service and shared-service controls preserve every serialized non-readiness field when authentication updates are disabled. Thirty-one configuration tests, six failure-recourse tests, client contracts, format, strict workspace Clippy, docs links, handoff docs and docs build passed.

The original public CLI workflow failed against the prior sealed binary and passed against the repaired debug fixture binary in three disposable homes. Access-plan selected each profile, verify-seeding updated only readiness and expected authentication metadata while retaining allocation and access policy, and invalid HTTP updates returned no-effect with unchanged readiness. Exact fixture hosts exited; fresh OS census found no task process residue. Debug binary SHA256: `7603c3468d0eb8447eb74ab4cb0090fc4d9dcabb847bb1d961958019047448b7`.

Preserve harness limits: the first cleanup assumed close would stop a standalone runtime host and failed; corrected cleanup stopped only the positively identified fixture binary in its disposable home. The first direct dispatcher test lacked response decoration; its corrected repro exercised actual recourse and reproduced uncertain effect. Those failed attempts are retained privately.

No real profile, credential, browser, provider or installed runtime was changed. This is source qualification and public synthetic execution with an unsealed debug artifact, not production promotion or an installed-generation claim. Reboot remains deferred. Issue #84 is next for bounded current qualification.
