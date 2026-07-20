---
id: preview-artifact-review
title: Preview Artifact Review
summary: Publish important review artifacts through an available preview service so human review and approval happen in a browser instead of through scattered local paths.
tags:
  - artifacts
  - previews
  - review
  - approval
---

## Policy

- When a turn produces artifacts that materially need human inspection, publish them through the available preview or approval service before asking the user to review them.
- Treat preview publishing as especially important for:
  - rendered documents
  - HTML or website builds
  - PDFs, Office documents, images, and galleries
  - reports, review packets, benchmark outputs, or generated summaries
  - any artifact family where one browser URL is clearer than a list of local file paths
- Use the preview skill or service only when it is available in the current environment. If it is unavailable, fall back to clear local paths and say that preview publishing was not available.
- Group related outputs into one preview session and return the session URL rather than sending many artifact links.
- If the next action depends on human approval, publish the preview, state the decision needed, and stop before making the gated change.
- Before continuing approval-sensitive work, read the preview feedback through the service when available; if feedback is absent or ambiguous, ask the user directly instead of assuming approval.
- Do not publish secrets, raw credentials, private tokens, or sensitive raw logs to preview sessions.
- Do not persist raw share-link tokens in repo docs, memory systems, logs, or committed artifacts.
- Keep preview publishing focused on human-review value. Do not create preview sessions for trivial outputs that are already clear in the terminal or final response.

## Adoption Notes

Use this module when a repo regularly creates local artifacts that should be inspected or approved by a human, especially when browser rendering, visual layout, generated packets, or multi-file output families matter.
