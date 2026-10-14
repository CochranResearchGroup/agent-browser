# Compact native workspace review

Product lane: PL-PLATFORM
Disposition: active-input
Owning plan or work item: P237 version 2 / #261 / #260
Related lanes: P238, P240

## Operator correction

The operator rejected all three initial layouts on 2026-10-14. The left rail
is a useful direction. Remove the green palette and prioritize no wasted space.
Handoffs must not remain visible: a small SVG control in the rail opens their
management surface as a modal or animated drawer. Earlier requests to choose
A/B/C are superseded. The independent link/tab lifetime and explicit link-only
deletion rules remain unchanged.

## Reference patterns

Read-only inspection of AuraCall's ux/operator styles and component, Codex Bots'
web/index.html and style.css in its plan1 checkout, and Remote View's web/home
assets found compact SVG toolbars, restrained dark surfaces, collapsible or
resizable navigation and details shown only on demand. No reference repository
or runtime was modified. These are UI references, not provider acceptance.

## Proposed revision

Prototype source is `d0686b9afc3910d8068094cc426d6c74c6b42c15` on
`prototype/native-dashboard-261`. Preserve it separately; it must not merge.
The original `4ca606ac8` study remains in history.

A 224 px collapsible/resizable browser-tab rail and one 36 px toolbar frame the
edge-to-edge desktop. Neutral charcoal replaces green. A small chain-link SVG
opens the compact handoff table in a content-sized drawer anchored beside the
rail. The drawer uses temporary overlay space, closes through its SVG or Escape,
and restores focus. Keyboard opening and reduced-motion mode omit animation.
Narrow screens show target/origin and independent lifetime summaries with visible
action controls rather than an overflowing desktop table. There is no permanent
handoff panel or bottom variant switcher.

The document shown inside the viewer is synthetic, giving the layout useful
scale rather than a large empty placeholder. It is not a new native viewer.
Properties, lifetime controls and link deletion operate only on in-memory
synthetic inventory. The drawer is one interpretation of the requested
modal-or-drawer direction, not a new settled choice by the operator.

[Review session](https://previews.ecochran.dyndns.org/s/4bac1a44b748) contains
self-contained interactive HTML and desktop/open-drawer/mobile screenshots.
Inline JavaScript avoids the source-view response that made the earlier
relative-asset preview blank. Share tokens are excluded from repository notes.

## Qualification and limits

Dashboard TypeScript passes. A disposable headless browser verifies default
hidden handoffs, full desktop bounds, Escape/focus return, keyboard opening,
draft preservation, separate link/tab lifetime edits, link deletion preserving
rail tabs, rail collapse/restore/keyboard resizing, narrow-screen action access
and reduced motion. No page errors occur. The first label lookup failed because
a native select's implicit label included option content; explicit accessible
names repair it. Original failure output is retained.

Local verification artifacts live under `/tmp/p237-compact-prototype-20261014`.
The source executable is not rebuilt or installed. The browser fixture closes
its private profile, and a fresh OS readback finds zero profile-marked processes.
A separate disposable task confirms the published iframe renders the 36 px
workspace with handoffs initially hidden and can open the drawer; scoped close
retires that task. This qualifies synthetic layout behavior only.

## Review and next gate

Serial Standards review preserves shadcn dialogs, accessible SVG names, neutral
surfaces and prototype-only custody. Spec review confirms the operator's new
on-demand/density requirements and retains full lifecycle semantics. No blocker
remains for publishing this study. Review of this revised prototype precedes
production UI replacement. The full P237 UI/public managed/provider/recreation,
reload/foreign/post-effect and canonical acceptance gates remain open.
