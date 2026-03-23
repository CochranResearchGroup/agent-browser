---
"agent-browser": minor
---

### New Features

- **Cross-origin iframe support** - Added support for snapshots and interactions within cross-origin iframes via `Target.setAutoAttach` (#949)
- **Network request detail and filtering** - Added `network request <requestId>` command to view full request/response detail, and new filtering options for `network requests` including `--type` (e.g. `xhr,fetch`), `--method` (e.g. `POST`), and `--status` (e.g. `2xx`, `400-499`) (#935)

### Improvements

- **Snapshot usability** - Reduced AI cognitive load by filtering semantic noise from snapshot output; cursor-interactive elements are now included by default, making the `-C` flag unnecessary (#968)
- **Upgrade command** - Improved robustness of installation method detection in the upgrade command (#960)
- **Target tracking** - Enhanced target tracking and page information handling for more reliable browser session management (#969)

### Bug Fixes

- Fixed **viewport dimensions** being reported incorrectly in streaming status messages and screencast (#952)
- Fixed **`find` command** flags such as `--exact` and `--name` leaking into fill values when used with fill actions (#955)
- Fixed **state commands** incorrectly starting the daemon when no `session_name` is provided (#677, #964)
- Fixed **auto-connect** triggering when the daemon is already running, preventing duplicate connections (#971)
- Fixed **Enter key press** not working by adding a text field to `keyDown` events (#972)
- Fixed **download command** to properly handle absolute paths and correctly click target elements (#970)

### Breaking Changes

- The `-C` / `--cursor` flag for `snapshot` is deprecated; cursor-interactive elements are now included by default and the flag has no additional effect (#968)

### Documentation

- Updated `README.md` with new `network requests` filtering options and `network request <requestId>` command usage
- Removed references to the deprecated `-C` / `--cursor` snapshot flag from docs and command reference
