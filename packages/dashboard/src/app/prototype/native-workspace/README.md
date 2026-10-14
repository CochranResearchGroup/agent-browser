# Compact native workspace study

The operator rejected the original A/B/C handoff placements on 2026-10-14.
Keep the left rail direction, remove green, minimize wasted space and open
handoffs only on demand. This prototype supersedes those layout proposals.

The work surface fills the space beside a collapsible/resizable 224 px rail
and one 36 px toolbar. A small rail SVG button opens a compact handoff drawer.
The drawer overlays the desktop temporarily and restores focus on dismissal.
Properties, separate link/tab lifetime controls and link-only deletion use
shadcn dialogs. The document, original context, URL, inventory and actions are
synthetic. No service calls, authentication, provider effects or persistence
are implemented. The sample document gives the viewer meaningful visual scale;
it is not a replacement native viewer.

Read-only reference patterns:

- AuraCall ux/operator: compact header, SVG actions, collapsible/resizable panes.
- grok-bots/codex-bots web: restrained dark palette, context actions and dialogs.
- Remote View web/home: desktop tree, retained viewer, on-demand details drawer.

Use neutral charcoal surfaces, muted type and restrained blue focus/link cues.
No green theme, permanent handoff table, oversized framing or variant switcher.
The drawer is one proposed interpretation of the operator's modal-or-drawer
request. Review it before production UI replacement.

Run the prototype package script and open /prototype/native-workspace locally,
or bundle preview-entry.tsx for publication as self-contained HTML. Inline its
JavaScript so Previews source-view handling cannot break relative script assets.
Include desktop, open-drawer and mobile screenshots in one preview session.

This throwaway branch must not merge into main. Rewrite the reviewed design
against the existing native Remote View integration in P237 and qualify actual
browser workflows independently of this study.
