---
id: writing-document-edit-safety
title: Writing Workspace Document Edit Safety
summary: Protect fragile document structures and prefer safe editing patterns for DOCX, citation fields, tracked changes, and review artifacts.
tags:
  - writing
  - docx
  - citations
  - safety
---

## Policy

- Treat complex office documents as fragile structured artifacts, not plain text blobs.
- Prefer safe editing methods that preserve:
  - live citation fields
  - tracked changes state
  - embedded drawings or anchors
  - formatting that carries workflow meaning
- If a tradeoff remains, preserve semantic document structures first and defer cosmetic cleanup to a later safe pass.
- When generating review snapshots such as PDFs, verify they correspond to the intended source document version.
- Document risky editing seams so future sessions do not rediscover them by accident.

## Adoption Notes

Use this module for writing-project repos that operate heavily on DOCX, PDF, citation-managed manuscripts, or similar structured deliverables.
