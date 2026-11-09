# Native saved-login window coverage (#596)

Product lane: PL-AUTH
Work item: 596
Branch: fix/native-login596-coverage
Base: 87035a504c6d2b80622f1c6045b31ccfa61b8b1f
State: OPEN
Owner: ecochran76
Consolidation: required

Scope: SDK saved-login capture coverage, public SDK protocol tests and required documentation.
Boundaries: no Rust behavior change, owner pixel-limit change, input/deadline budget change, runtime installation, live input or original-episode replay.

A 1440 by 900 window defaults to a 1440 by 694 capture under the Native owner's
one-million-pixel limit. A chooser below row 704 cannot be discovered in that
capture. The SDK currently requests no further region. This is separately proven
from the earlier uncertain live username input; its cause remains unknown.

The public SDK callTool seam reproduces a chooser between rows 704 and 820:
`node scripts/test-native-saved-login.js` fails with human_required before repair.
The fix covers the window with overlapping bounded tiles, deduplicates the same
physical target, refuses distinct competing targets, then recaptures the selected
tile before input. Each click retains its own opaque reference, original episode
and latest capture request. Frames are not stitched; submit still requires masked
password and the intended username in the same fresh frame. Unsupported surfaces
stop rather than alter owner limits or authority.

Source qualification passes the [focused validation receipt](native-login596-coverage-validation.json): SDK protocol/type checks, full no-launch service-client gate, docs build, exact-lane planning/policy/links and patch hygiene. Existing SDK deployment pins and completed or
uncertain episodes remain immutable. Adoption requires a successor immutable SDK
path/hash in the consumer's reviewed configuration after terminal old episodes;
source tests do not qualify actual saved-password acceptance.

The sealed Native binary remains unchanged. This successor changes SDK code and help documentation; no binary rebuild or runtime adoption was performed. #596 stays OPEN until actual ordinary saved-password login acceptance.
