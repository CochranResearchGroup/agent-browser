# Throwaway native workspace layout study

Question: where should the prominent handoff table sit alongside the native desktop and expandable browser/tab rail?

Run `pnpm prototype:native-dashboard`, then open `/prototype/native-workspace?variant=A` on port 3187. A puts the handoff shelf below the desktop. B puts the handoff table alongside the desktop. C gives the handoff desk the primary pane with the desktop beside it. The bottom switcher and arrow keys change variants. Inputs retain their arrow keys.

This separate route avoids mounting the retired dashboard's live synchronization hooks. All data and actions are synthetic, in memory. No viewer, service request, runtime mutation or persistence is implemented. Properties, sharing, independent per-link/per-tab policy edits and link-only deletion are demonstrable interactions, not backend acceptance. The switcher is excluded from production builds. `preview-entry.tsx` captures the same component for a portable development-only artifact.

Keep this branch out of main. Review the layouts, record the choice, then rewrite the selected design against the existing native Remote View integration and qualify real workflows through P237.
