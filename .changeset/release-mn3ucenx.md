---
"agent-browser": patch
---

### Bug Fixes

- Fixed **modifier key chords** (e.g. `Control+a`, `Shift+Enter`, `Control+Shift+a`) not being handled correctly when using `press`. Modifier keys (`Alt`, `Control`/`Ctrl`, `Meta`/`Cmd`, `Shift`) are now parsed and forwarded as CDP modifier bitmasks rather than treated as part of the key name (#980)
- Fixed **query parameters being dropped** from `--cdp` HTTP URLs (e.g. `http://host:9222?mode=Hello`). Query strings are now preserved and forwarded to the remote CDP endpoint (#982)
