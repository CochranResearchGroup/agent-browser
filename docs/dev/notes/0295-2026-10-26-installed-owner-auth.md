# Isolated installed owner-auth qualification

Product lane: PL-AUTH. Ticket: #463. This is synthetic qualification of the client-owned enrolled Chrome route, not production adoption or native AT-SPI acceptance.

The public seams are the installed `connectHostedAuthHelper()` default discovery, owner login and TOTP helpers, actual standalone Auth credential authority, parent request hub and notification relay, and enrolled browser receiving/capture APIs. The synthetic HTTPS account server independently counts password and factor POSTs and returns intended account/company identity. No real provider or private credentials participate.

The test installs copied client/service/parent closures and a producer-built, verified SDK candidate into disposable directories. The client loads `XDG_DATA_HOME/auth-receipts/client/current` without an `installationRoot` override. Its default discovery path under the isolated HOME points only to the fixture's private connection directory. Discovery environment overrides are removed. Current Auth retains the `codex-bots-auth` v2 transport identifier in that metadata; the deprecated Bots auth helper is never invoked.

Physical enrollment uses Auth's supported `inspectAgentBrowserTarget` and `createAgentBrowserRuntime`: actual Chrome PID, `/proc` ownership and birth, exact profile command line, private DevTools marker and actual target lookup determine generation. No supplied generation, static resolver or synthetic privacy booleans authorize receiving. Before each actual CDP fill, a call-through observer requires the real enrolled `capture()` operation to refuse with `BROWSER_CAPTURE_HELD`. The durable private fence remains held on the MFA page and across receiver reopen. Its scope is cooperating enrolled capture/control, not unrestricted same-user Chrome controllers.

The test requires installed Google Chrome at `/usr/bin/google-chrome`, OpenSSL, Playwright, the reviewed Auth/parent source trees and their source dependencies. The owned target must be foreground for the current real CDP screenshot implementation; an unknown screenshot is a test failure, retains its gate, and is never retried within that fixture. The harness closes its CDP clients, exact Chrome process tree and HTTPS server, then checks a fresh OS process census for the disposable profile.

## Red receipt

Command, from the ticket worktree:

```sh
AUTH_RECEIPTS_SOURCE=/home/ecochran76/workspace.local/auth-receipts-browser449 RECEIPTS_SOURCE=/home/ecochran76/workspace.local/receipts-issue106 node scripts/test-installed-owner-auth.js
```

At Agent Browser `a9be44208a3882b1f54b1ef6af34cb10f2c55c08` and Auth `ee85573886a55b931ad732f2dd5a4ec8c0ed8167`, the installed helper reached one real password POST, one source read and two fenced protected inputs. Same-target TOTP failed before any factor input: the enrolled receiver returned `RECEIVER_RESOURCE_HELD`, projected publicly as `auth_unavailable`. The TOTP helper used an observed-authentication request without the existing `continuationOf` resource transfer. An SSO-only test would miss this failure, so the test retains the required same-target chain.

## Qualification limits

Native AT-SPI is outside this selected route. The enrolled receiver does not expose `observeChallenge` or `respondChallenge`, so installed number matching is unsupported here. Its separately qualified source API is not evidence of installed enrolled-route support. No production selector, installed service, runtime custody, real account, Slack route or provider was changed.

## Green receipt

The same command passed with the existing-contract TOTP fix `b150741c9` applied as worktree commit `1bcc554f0a5145ae5616ebd18e86bd9b10311115`. Auth remained `ee85573886a55b931ad732f2dd5a4ec8c0ed8167`; parent interoperation remained `d6e07227e18b4ad9c866c6fe24e64bdcecdde294`. The fix requests the existing `continuationOf` transfer for an original password request, while retaining observed-authentication continuation for SSO. This receipt does not qualify a newly invented provider wire contract.

The installed candidate closure hashes below are SHA-256 over sorted relative-file-path/content-hash pairs. Copied service, parent and browser-client source trees are independently compared against their reviewed source trees before use. Service dependencies are copied with symlinks dereferenced, so the runtime candidate does not depend on an original dependency directory.

| Candidate | Files | SHA-256 |
| --- | ---: | --- |
| Auth service with dependencies | 540 | `77bf911799a3fbd82fa49a85d6f2b7905eac56a6f87a931b8c37c20d372dd958` |
| Parent interoperation | 6 | `82f7e4c0fa41a75d16fbed29682108d58affb4ab73b2b631d681147f5f431c14` |
| Browser client | 23 | `222033f2ebf002eb8de8b3ad45d6c82999324acb2ade9085d92f4fdbe7c63f26` |
| Verified producer SDK | 4 | `ad768e28b555f91aaf15452def8502e97c6a678705402d0349e36c189145f1eb` |

The verified SDK reports product `auth-receipts`, contract version 1, identity `51728137cc9c614a49b0fd3260f008661a8f18dc0b4a9f5c63748d20d74c0465` and the Auth source revision above. Service and parent are isolated Node source candidates hosted in the test process, not registered or adopted installed daemons. Their owner channels, source data and HTTPS account are synthetic.

The completed same-target browser workflow produces exactly one password POST, one factor POST, one protected source read and three protected fills. All three fills require an actual enrolled capture refusal immediately before input. Capture succeeds before private work and after independent intended-account/company verification; it remains held on the MFA page. Authority restart and receiver reopen preserve original request identities and do not repeat input or submission. The original password workflow also independently reaches completed status after TOTP. A separate real target retains its original untouched password form. This installed recovery slice covers durable acknowledgement plus restart; source-level lost browser/transport acknowledgement fixtures remain separate evidence.

The final run uses Chrome `153.0.8010.52`, proves the actual Chrome PID and derived generation, refuses a forged generation and an unenrolled caller, and observes the owned Chrome exit plus zero remaining profile processes and zero remaining processes retaining the task-owned HOME/candidate paths. Every test fixture and candidate directory is removed. The synthetic machine receipt is written outside Git at `/var/tmp/agent-browser-installed-owner-463.json` when `INSTALLED_OWNER_AUTH_EVIDENCE` is set; it includes the per-run PID, target and generation. No raw credential, seed, auth state, screenshot or provider content is retained in this note.

## Bounded post-review repair: factor button activation

The review identified that TOTP used `requestSubmit(button)` while password receiving activates the reviewed button itself. The installed fixture now attaches the factor POST handler to the actual Verify button and makes the form's submit event inert. Before the fix, the same-target run failed waiting for the signed-in page: code input occurred, but the button handler never ran. The owned Chrome PID `689825` exited and the fresh profile census was zero. Exact RED output is preserved outside Git in `/var/tmp/agent-browser-463-factor-button-red.log`; the corresponding existing generic receiver test also failed its expected authenticated result in `/var/tmp/auth-463-factor-button-unit-red.log`.

RED and GREEN use the same installed public seam:

```sh
AUTH_RECEIPTS_SOURCE=/home/ecochran76/workspace.local/auth-receipts-browser449 RECEIPTS_SOURCE=/home/ecochran76/workspace.local/receipts-issue106 node scripts/test-installed-owner-auth.js
```

Auth commit `077f0e3f74420b52001e0120c138e353df8420e9` minimally changes factor submission to call the validated visible unique button's `click()` method and refuse a missing method. The existing no-submitter fallback remains `requestSubmit()`. Existing origin, method, target, form/control and action checks remain in force; no wire contract changes.

The committed-producer installed GREEN is preserved in `/var/tmp/agent-browser-463-factor-button-green.json` and `.log`. It records one password POST, one TOTP POST through the actual reviewed button, one protected source read, three capture-fenced fills, restart without replay, independent account/company completion and the untouched peer target. Chrome PID `698422` exited; both the profile census and task-HOME/candidate-path census were zero. There was no retry of an unknown operation within the fixture.

This latest run used Browser source `ee0cfb127b7c2e70fbfab19a776109836665c230` and parent `d6e07227e18b4ad9c866c6fe24e64bdcecdde294`. Closure hashes were service `67f0662ebb4051b4803784d17f3386b5ed4a079ab444c12f02e2228a68fa429e`, browser client `142a5b8a74fa8ea9b9fbdfd8063224ce2fe61fcb5894a2919bc373cd775d0f0b`, parent `82f7e4c0fa41a75d16fbed29682108d58affb4ab73b2b631d681147f5f431c14`, and SDK `29732cce70fd1454dee4481ccfb66e1729495ad88bed74467a6885672e16fad6`. Verified SDK identity was `e5995c2de61428c45b5ae5527531667f800614dc75d89df2131abb411af95d10`, bound to the committed Auth revision above.

The generic receiver suite passed 21/21. The focused generic, helper and enrolled receiver suites passed 27/27 with:

```sh
node --test test/generic-browser-receiver.test.mjs test/helper-browser-receiver.test.mjs test/enrolled-browser-receiver.test.mjs
```

This is one bounded repair in the coordinator's single post-review TDD pass. Native AT-SPI and enrolled number matching remain outside the qualified route.
