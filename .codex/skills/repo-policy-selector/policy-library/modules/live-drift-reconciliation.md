---
id: live-drift-reconciliation
title: Live Drift Reconciliation
summary: Detect and reconcile changes made outside the repo workflow before release so live drift becomes an explicit merge decision instead of an accidental overwrite.
tags:
  - website
  - drift
  - reconciliation
  - release
---

## Policy

- Before a meaningful release to a live website or CMS-managed surface, check whether live state has drifted from the repo and local review environment.
- Treat live drift as a real reconciliation surface, not as background noise.
- Review file-backed drift first, because tracked code differences can be overwritten silently by deploy.
- Review database-backed or admin-side drift next, such as content, menus, assets, settings, or other runtime-managed changes.
- For each meaningful drift item, make an explicit decision to:
  - accept the live change into repo-managed state
  - overwrite it intentionally with the next release
  - promote it into a reproducible migration or operational artifact
  - defer release until the conflict is resolved
- If both local work and live changes touch the same page, flow, asset set, setting, or configuration surface, resolve that conflict explicitly before deploy.
- Record drift reports or reconciliation notes durably enough that future maintainers can understand why a release did or did not overwrite live state.
- Do not assume the live site still matches the last pulled mirror or the last reviewed backup state.

## Adoption Notes

Use this module when website or CMS changes can be made outside normal repo flow, especially through admin interfaces, vendor dashboards, or other live operational surfaces.
