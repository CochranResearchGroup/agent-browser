# Load the MCP handoff origin from standard environment configuration

State: CLOSED
Product lane: PL-BUGFIX
Lane: P266
Owner: primary Codex session
Work item: CochranResearchGroup/agent-browser#358
Branch: fix/public-origin-env
Target: main
Integration: merge

## Current state

Installed MCP browser_open succeeds. Handoff fails when its long-running process
lacks AGENT_BROWSER_PUBLIC_ORIGIN. An otherwise identical fresh MCP request with
the reviewed configured origin returns a ready opaque link for the same task.
The bootstrap omission is repaired. The real loader test fails before repair and all four dotenv tests pass after it. Strict workspace Clippy, formatting, policy wiring, links and handoff guidance pass. An isolated candidate MCP startup with dotenv-only origin advances to task database lookup while the installed baseline rejects the origin. Source is integrated by PR360 at fa9a03253 and installed as generation 0.28.0-76cec5b73183-79b66b79b467. Fresh installed MCP dotenv handoff, authenticated native input/reconnect, production doctor and preservation pass. The cached conversation still requires MCP reconnect; shared refresh lacked host scope approval.

## Bounded outcome

Load the service public origin from the existing dotenv configuration without
changing explicit environment precedence, origin validation, or native ownership.
Own agent_env.rs and additive setup guidance only. P265's hosted workflow/model
changes and other worktrees are preserved; no workflow adapter edits.

## Acceptance and terminal condition

Observe the real loader test fail before the allowlist fix and pass after it;
verify explicit environment precedence, formatting and affected checks. Record
installed same-task handoff qualification separately from source integration.
Authenticated viewer/input/reconnect evidence belongs to Remote View316.
No formal release or shared runtime restart in this source packet. The debug candidate refuses a stale live daemon lane before browser creation; no recovery fence was bypassed. Installed configured-origin native input and reconnect passed separately.
