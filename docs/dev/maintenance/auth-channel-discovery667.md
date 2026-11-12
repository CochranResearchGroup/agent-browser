# Verified Auth channel discovery in ordinary browser login

Issue: #667, depending on Auth354. Owner: ecochran76 / Codex primary.

Outcome: the existing hosted/enrolled Auth helper forwards read-only IM/Mail destination proposals through Auth's verified SDK. The agent follows one discoverable orchestration contract before asking for retrievable facts. No input, policy, review or receiver method is added. Old SDKs return a typed repair path with no alternate client.

Scope: client JS/types/tests plus five required guidance surfaces. One existing primary checkout is reused; prior retirement650 branch/history and all peer work remain intact. Browser runtime/control/retirement code is unchanged. Help text changes are static guidance only.

Validation: actual RED missing method then GREEN for hosted/enrolled forwarding and old-SDK refusal, existing helper/follow/enrollment tests plus TypeScript. Before source integration run client/guide/type checks, policy/docs checks, formatter, and an actual freshly published Auth SDK bridge. Qualify any changed artifact/install separately. Preserve exact source, logs and remaining gates; no paid hosted CI.

Non-goals: choosing a factor by policy, owner approval, SMS/email request, OTP extraction, a fresh login, browser mutation, profile reset or a new first-release claim. Full cross-product skill propagation and installed same-task use remain open until demonstrated.

A service-context run exposed a fixture setup defect: permissive umask0002 made
SDK fixture directories/files group writable, so the unchanged production SDK
privacy check correctly refused them. The original hosted fixture reproduces
`auth_sdk_invalid` under that umask. The four helper fixtures now create owned
0700 directories/0600 files explicitly; all helper/follow/enrolled/type checks
pass under the same umask without changing the production check. Preserve the
failed run; qualification continues on the corrected fixture source.

The extra actual-source owner-playbook probe reached a separate historical parent
contract mismatch: reviewed Receipts checkpoint d6e0722 refuses null pending
review deadlines required by current Auth. The unchanged Browser472d264 baseline
reproduces the same pending request, zero delivered prompts and approval-receive
refusal, with zero private reads/inputs. Track Receipts52 with Auth354. This is an
unwaived owner-review integration gap, not a failure caused by the read-only SDK
forwarder. Do not report that parent journey as passed or the whole installed
orchestration complete. Component helper/guidance/packaging checks and actual
read-only SDK bridge remain independently verifiable; the combined owner journey
requires a qualified current parent contract.

An actual SDK producer bug was also exposed: source publication inherited group
write permission for module directories under umask0002. Auth's producer now makes
0700 subdirectories and its verifier rejects writable directories, matching the
unchanged Browser guard. Two regression tests went red then green;32 Auth focused
checks pass. The actual newly published Auth0995 SDK and this Browser helper pass
SMS and Mail-alias discovery under that same umask, with zero Auth input/approval
calls. Final changed-source Auth full validation and remaining component gates
are pending; source and installed acceptance stay distinct.
