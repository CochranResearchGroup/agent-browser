# Client and dashboard failure preservation, ticket 760

Product lane: PL-PLATFORM

Baseline: `ddaf0a69c`. This receipt describes the first source slice, not ticket acceptance.

The Service success assertion now rejects explicit nested `data.ok=false` and `data.success=false`. Its recovery reader accepts owner recourse at outer, nested data and MCP error-data positions. Playbook, Native saved login, image challenge and challenge assistance transport adapters reject nested failures. They preserve the original typed owner failure; the shared client adapter preserves occurrence identity and request identity. Saved login preserves effect uncertainty when returning its intervention result. No adapter automatically retries input.

Dashboard fetch observation detects nested application failure and records only a bounded owner code and original occurrence identity as `observationId`. The server generates the dashboard observation's own occurrence identity; the original correlation is not represented as a new owner occurrence. No raw failure object or body is logged. Screenshot/control adapters retain owner failure metadata when throwing.

The separate recovery registry fragment covers five exact local transport fallback codes and declares real public client exports. Existing owner codes pass through unchanged and belong to the native registry. A transport configuration repair is a caller action with explicit preconditions, not an invented browser command. Missing task/request identity does not grant executable scope.

## Validation

Three regressions were run against exact baseline source temporarily restored in the existing checkout, then the changes were restored: Service nested recourse, task briefing typed refusal, dashboard nested refusal. All failed against baseline. Focused existing harnesses passed with the changes:

- `node scripts/test-service-request-client.js`
- `node scripts/test-task-briefing-client.js`
- `node scripts/test-dashboard-failure-observation.js`
- `node scripts/test-dashboard-view-streams.js`
- `node scripts/test-challenge-assistance.js`
- `node scripts/test-native-saved-login.js`
- JavaScript checkJs for task briefing, playbooks, Native image challenge and Native saved login through the existing docs TypeScript installation.

No compiler, browser launch, installed runtime change, private account input or hosted CI was used.

## Remaining acceptance scope

The exported package also includes Service observability, hosted authentication/receiving, teaching evidence/review/grilling/media, workspace SiteGuides and login orchestration. Their local validation errors and other emitted failure codes still need source-qualified inventory, explicit recovery entries or documented normalization. The five-code fragment is not a full exported-client catalogue. Dashboard workspace recovery string formatting and read coordinator caching also remain inspection targets. Registry validation and integrated dashboard type/build gates are coordinator-owned. This ticket must remain open until that coverage and integrated acceptance are complete.

Memory disposition: forbidden. Parent instructions prohibit memory updates for this worker slice.
