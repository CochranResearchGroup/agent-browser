# Session Management

Named sessions give concurrent tasks separate tabs. Profile selection determines whether browser data is shared or isolated.
Use runtime profiles for persistent browser identity and long-lived auth state.

**Related**: [authentication.md](authentication.md) for login patterns, [SKILL.md](../SKILL.md) for quick start.

## Sharing a profile between tasks

Use one named session and tab per task. When tasks deliberately need the same login and stored browser data, select the same durable profile so they reuse one browser. Give each task its own session name; address commands through that session rather than a global active tab.

Tab-targeted automation can run without either tab being physically visible. Serialize foreground desktop clicks, keyboard input and human control. Separate tabs share cookies, login/logout state and persistent site storage: coordinate account switches, logout and conflicting application changes. Choose separate profiles when independent state is required.

Close only the task's session. Closing one session must preserve its peers; the final session releases the browser. Do not delete the shared profile to clean up a task. If a site requires foreground execution, serialize that part of the workflow.

## Contents

- [Named Sessions](#named-sessions)
- [Session Isolation Properties](#session-isolation-properties)
- [Session State Persistence](#session-state-persistence)
- [Common Patterns](#common-patterns)
- [Default Session](#default-session)
- [Session Cleanup](#session-cleanup)
- [Best Practices](#best-practices)

## Named Sessions

Use `--session` to isolate concurrent browser sessions:

```bash
# Session 1: Authentication flow
agent-browser --session auth open https://app.example.com/login

# Session 2: Public browsing (separate cookies, storage)
agent-browser --session public open https://example.com

# Commands are isolated by session
agent-browser --session auth fill @e1 "user@example.com"
agent-browser --session public get text body
```

## Session Isolation Properties

Each task has independently addressed tabs. Tasks sharing a profile share cookies,
local storage, IndexedDB, cache and history. Use distinct profiles when those data
must be isolated. Session storage follows the site and tab behavior.

## Session State Persistence

`--session` identifies the task and its tabs. It does not replace runtime profiles.
If you need a browser identity that survives manual sign-in and later agent
reuse, prefer `--runtime-profile`.

### Runtime Profile Plus Session

You can combine a named runtime profile with a named session when you need both
persistent identity and distinct task tabs:

```bash
agent-browser --session reviewer-a --runtime-profile work open https://app.example.com
agent-browser --session reviewer-b --runtime-profile staging open https://staging.example.com
```

### Save Session State

```bash
# Save cookies, storage, and auth state
agent-browser state save /path/to/auth-state.json
```

### Load Session State

```bash
# Restore saved state
agent-browser state load /path/to/auth-state.json

# Continue with authenticated session
agent-browser open https://app.example.com/dashboard
```

### State File Contents

```json
{
  "cookies": [...],
  "localStorage": {...},
  "sessionStorage": {...},
  "origins": [...]
}
```

## Common Patterns

### Authenticated Session Reuse

Prefer managed runtime profiles for recurring authenticated work:

```bash
agent-browser --runtime-profile work runtime login https://app.example.com/login
agent-browser --runtime-profile work open https://app.example.com/dashboard
```

Use state save/load when you specifically need a portable snapshot:

```bash
#!/bin/bash
# Save login state once, reuse many times

STATE_FILE="/tmp/auth-state.json"

# Check if we have saved state
if [[ -f "$STATE_FILE" ]]; then
    agent-browser state load "$STATE_FILE"
    agent-browser open https://app.example.com/dashboard
else
    # Perform login
    agent-browser open https://app.example.com/login
    agent-browser snapshot -i
    agent-browser fill @e1 "$USERNAME"
    agent-browser fill @e2 "$PASSWORD"
    agent-browser click @e3
    agent-browser wait --url "**/dashboard"

    # Save for future use
    agent-browser state save "$STATE_FILE"
fi
```

### Concurrent Scraping

```bash
#!/bin/bash
# Scrape multiple sites concurrently

# Start all sessions
agent-browser --session site1 open https://site1.com &
agent-browser --session site2 open https://site2.com &
agent-browser --session site3 open https://site3.com &
wait

# Extract from each
agent-browser --session site1 get text body > site1.txt
agent-browser --session site2 get text body > site2.txt
agent-browser --session site3 get text body > site3.txt

# Cleanup
agent-browser --session site1 close
agent-browser --session site2 close
agent-browser --session site3 close
```

### A/B Testing Sessions

```bash
# Test different user experiences
agent-browser --session variant-a open "https://app.com?variant=a"
agent-browser --session variant-b open "https://app.com?variant=b"

# Compare
agent-browser --session variant-a screenshot /tmp/variant-a.png
agent-browser --session variant-b screenshot /tmp/variant-b.png
```

## Default Session

When `--session` is omitted, commands use the default session:

```bash
# These use the same default session
agent-browser open https://example.com
agent-browser snapshot -i
agent-browser close  # Closes default session
```

## Session Cleanup

```bash
# Close specific session
agent-browser --session auth close

# List active sessions
agent-browser session list
```

## Best Practices

### 1. Name Sessions Semantically

```bash
# GOOD: Clear purpose
agent-browser --session github-auth open https://github.com
agent-browser --session docs-scrape open https://docs.example.com

# AVOID: Generic names
agent-browser --session s1 open https://github.com
```

### 2. Always Clean Up

```bash
# Close sessions when done
agent-browser --session auth close
agent-browser --session scrape close
```

### 3. Handle State Files Securely

```bash
# Don't commit state files (contain auth tokens!)
echo "*.auth-state.json" >> .gitignore

# Delete after use
rm /tmp/auth-state.json
```

### 4. Timeout Long Sessions

```bash
# Set timeout for automated scripts
timeout 60 agent-browser --session long-task get text body
```

### 5. Prefer Runtime Profiles For Login-Bound Work

```bash
# Good: persistent browser identity for recurring authenticated automation
agent-browser --runtime-profile billing runtime login https://accounts.example.com
agent-browser --runtime-profile billing open https://app.example.com/invoices

# Use --session mainly for concurrency or isolation
agent-browser --session scrape-a open https://docs.example.com
```
