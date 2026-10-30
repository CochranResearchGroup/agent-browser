# Dashboard left rail scrolling

Product lane: PL-BUGFIX. Work item: #495. Owner: ecochran76.

A long browser inventory enlarged the native workspace's implicit grid row past
the viewport. The inventory already had overflow scrolling, but its parent rail
could grow with its contents. An explicit zero-minimum grid row and rail minimum
height constrain the list while preserving the search, header, viewer and rail
resize handle.

The mounted fixture appends 80 synthetic entries and exercises 1280 by 480 and
390 by 360 viewports. It checks actual wheel scrolling, keyboard focus revealing
the last entry, stationary header and viewer bounds, and no document overflow.
Before the fix it fails at the viewport containment assertion. The focused lane
is `pnpm test:compact-native-workspace --rail-scroll-only`; default invocation
retains all pre-existing checks. The focused lane skips the unrelated mode
inspection prerequisite and stops after the scrolling regression.

The first repaired full run passes the scroll regression and then fails the
pre-existing assertion "No dead mutation controls before their implementation"
because a matching browser-mode menu item exists. Running the original committed
script independently also encounters a browser-mode prerequisite timeout waiting
for "Relaunch without CDP"; another run encounters "Choose browser build". These
broader fixture problems are retained and are not passing evidence for this fix.

Dashboard production build including TypeScript and patch hygiene pass. Source
and disposable synthetic-browser validation do not establish installed behavior.
No production service, profile or installation is changed.
