# Auth approval transport recourse771

Product lane: PL-BUGFIX

Issue: https://github.com/CochranResearchGroup/agent-browser/issues/771

Bounded repair from canonical ef99c1ee8. Auth Receipts446 repairs two masks: parent submission/observation failure became `auth_not_authorized`, and the public broker then hid the corrected code behind `auth_unavailable`. Parent transport failure is operational; only actual policy/owner decisions determine authorization. Already valid permission must continue without another approval merely because transport fails.

The actual SDK with corrected Auth source preserves `auth_parent_unavailable` and original request identity. Its recovery coverage was unknown. Register this exact forwarded owner code in the existing client registry with authenticated same-request inspection and `receive_credential_approval` after connection repair. Missing context stays unbound. The operator notification flow remains owned by Auth and its parent; no additional approval service or automatic private input is introduced.

The existing service-request client regression failed with actual unknown versus registered before the registry change. It checks exact original request arguments, unavailable evidence rather than denied authority, and nonautomatic execution. Auth446 separately tests the real broker, restart, restored parent, actual synthetic owner decision and single private use; current permission makes zero approval calls despite failed transport.

A historical Receipts parent rejects durable pending `expiresAt:null`. That genuine source incompatibility remains visible as `approvalDelivery.causeCode:invalid_request`; no artificial finite expiry is introduced. These historical sources are diagnostic inputs, not current installed acceptance.

Source validation alone does not establish installed Auth or production browser adoption. Subsequent authorized deployment and its qualification limits are recorded below.

The actual SDK probe against corrected Auth source and the historical parent now returns registered `auth_parent_unavailable`, pending original request, `invalid_request` delivery cause and zero private reads/inputs. The strict inventory passes2924 exact contracts,2255 reviewed candidates and all1526 original candidates. Full service-client, docs build, policy wiring, changed-link checks and formatting pass locally. Strict workspace Clippy passes. Auth449 merged at5fbc0d16c014b2466e11a1a2f5af1837c2e3758c: full checkpoint tests pass2038, fail0, skip2 Windows-only checks; combined-main approval/handoff tests pass80. Auth source worktree was removed cleanly; foreign dirty work and installed issue447 custody remain intact.


PR772 merged at c3ace3c3136c5f3be417e28b06f2a5d7d4a8ad89. Both771 and Auth446 are closed. The production-shaped release candidate binds source dd2ad701e86e8f5ab85b264121e1c39fc8a0135c and binary SHA256 b1d0ab184ddb1eb33f61742d8708760b0a3da47e3b164afc7a557a947f8b05f0. Canonical executable inputs match. The supported preserving install accepted those bytes, and installed doctor passes. The existing260-operation acceptance passes against the installed binary with original archival lookup, duplicate protection, refusal journal correlation, live policy recovery and zero owned residue.

Remote View source69a33f7e, integrated by989, was bundled and installed. All27 named software services adopted its binary50fedb5c990bba9bebbfa2c10b71b9076201322cc0b092ca12fe6074cb12f056. Installation/configuration identity checks match and the registered MCP responds. Existing doctor findings are unchanged; this does not claim all old desktop faults repaired. A fresh durable-profile production workflow reports Native operator visibility ready, reads the public page and closes its task. Its synthetic public profile is retained explicitly. Auth449 remains source-only while separate installed447 work continues.

Finite disposable retention exposed two separate conditions: the supervised runtime cannot resolve the default StableTime launcher in its service PATH, and the supported authority currently returns unbounded with cache_age_exceeded. The documented absolute launcher flag is staged in service-specific private configuration; the running host has not adopted it. Reliable time is never replaced with a WSL clock. Ordinary260-operation work and the durable Native workflow pass; finite-lifetime qualification remains pending. Actual failure occurrences are retained in the existing journal.

Attempting a preserving same-generation reapply exposed an older installer bug: it stages before rejecting already-selected bytes and enters recovery. Exact-transaction recovery restored the selected runtime; final doctor and the durable production workflow pass. Issue773 owns the bounded non-mutating/idempotent reapply repair. It is a separate backlog item, not a reopened749 or771 ticket. No direct host restart, browser force-close, private Auth operation or StableTime service mutation was performed. The current generation, recovery history, staged launcher configuration and detailed evidence remain in user-scoped validation artifacts outside Git.
