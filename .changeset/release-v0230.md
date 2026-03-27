---
"agent-browser": minor
---

### New Features

- **Observability dashboard** - Added a local web UI (`dashboard`) that shows live browser viewports, command activity feeds, console output, network requests, storage, and extensions for all sessions. Manage it with `dashboard start`, `dashboard stop`, and `dashboard install`. The dashboard runs as a standalone background process and all sessions stream to it automatically (#1034)
- **Runtime stream management** - Added `stream enable`, `stream disable`, and `stream status` commands to control WebSocket streaming at runtime. Streaming is now always enabled by default; `AGENT_BROWSER_STREAM_PORT` overrides the port instead of toggling the feature (#951)
- **Close all sessions** - Added `close --all` flag to close every active browser session at once

### Bug Fixes

- Fixed **Lightpanda engine** compatibility (#1050)
- Fixed **Windows daemon TCP bind** failing when Hyper-V reserves the port by falling back to an OS-assigned port and writing it to a `.port` file (#1041)
- Fixed **Windows dashboard relay** using Unix socket instead of TCP (#1038)
- Fixed **radio/checkbox elements** being dropped from compact snapshot tree because the `ref=` check required a leading `[` that those elements lack (#1008)
