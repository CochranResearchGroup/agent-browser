# Auth approval transport recourse771

Product lane: PL-BUGFIX

Issue: https://github.com/CochranResearchGroup/agent-browser/issues/771

Bounded repair from canonical ef99c1ee8. Auth Receipts446 repairs two masks: parent submission/observation failure became `auth_not_authorized`, and the public broker then hid the corrected code behind `auth_unavailable`. Parent transport failure is operational; only actual policy/owner decisions determine authorization. Already valid permission must continue without another approval merely because transport fails.

The actual SDK with corrected Auth source preserves `auth_parent_unavailable` and original request identity. Its recovery coverage was unknown. Register this exact forwarded owner code in the existing client registry with authenticated same-request inspection and `receive_credential_approval` after connection repair. Missing context stays unbound. The operator notification flow remains owned by Auth and its parent; no additional approval service or automatic private input is introduced.

The existing service-request client regression failed with actual unknown versus registered before the registry change. It checks exact original request arguments, unavailable evidence rather than denied authority, and nonautomatic execution. Auth446 separately tests the real broker, restart, restored parent, actual synthetic owner decision and single private use; current permission makes zero approval calls despite failed transport.

A historical Receipts parent rejects durable pending `expiresAt:null`. That genuine source incompatibility remains visible as `approvalDelivery.causeCode:invalid_request`; no artificial finite expiry is introduced. These historical sources are diagnostic inputs, not current installed acceptance.

Source validation and publication do not establish installed Auth or production browser adoption. Spec749 source acceptance remains complete and its separate deployment request remains pending.

The actual SDK probe against corrected Auth source and the historical parent now returns registered `auth_parent_unavailable`, pending original request, `invalid_request` delivery cause and zero private reads/inputs. The strict inventory passes2924 exact contracts,2255 reviewed candidates and all1526 original candidates. Full service-client, docs build, policy wiring, changed-link checks and formatting pass locally. Strict Clippy and final Auth source qualification remain in progress.
