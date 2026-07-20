---
id: cloud-drive-course-governance
title: Cloud Drive Course Governance
summary: Govern course workspaces that rely on cloud-drive mounts, native cloud documents, connector identity, and placeholder files.
tags:
  - course
  - drive
  - cloud
  - documents
---

## Policy

- Treat cloud-drive course folders as synchronized operational surfaces, not plain local filesystems.
- Keep the canonical cloud folder identity explicit when the course depends on a shared drive, folder id, or stable URL.
- Treat provider-native document placeholders, such as Google Drive for Desktop `.gsheet`, `.gdoc`, `.gslides`, and `.gform` files, as shortcuts or metadata stubs rather than reliable local document bodies.
- Use connector or API-aware tools when provider-native identity, sharing state, form responses, comments, or spreadsheet contents matter.
- Prefer stable cloud ids and configured URLs over parsing placeholder files.
- Do not assume local file mtimes, placeholder sizes, or synced shortcut contents fully represent provider-native document state.
- Do not duplicate, move, or rename native cloud documents through local filesystem operations unless the user explicitly asks for filesystem-level reorganization and the effect on cloud identity is understood.
- When a course workflow uses cloud spreadsheets or forms, prefer the course tool's configured integration or the stored provider URL over manual local placeholder reads.
- Before changing sharing, publication, or folder location, verify intended audience and course scope.
- Keep downloaded exports distinct from authoritative provider-native originals.

## Adoption Notes

Use this module when a course workspace lives in Google Drive, OneDrive, Box, Dropbox, or another cloud-sync surface and contains provider-native documents or LMS-linked cloud artifacts.
