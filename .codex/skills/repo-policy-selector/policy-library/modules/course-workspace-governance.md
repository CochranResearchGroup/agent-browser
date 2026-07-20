---
id: course-workspace-governance
title: Course Workspace Governance
summary: Govern live course workspace identity, folder authority, archives, generated artifacts, and instructor workflow boundaries.
tags:
  - course
  - education
  - workspace
  - governance
---

## Policy

- Treat a course workspace as a live operational surface, not as a generic document folder.
- Make the course identity explicit before substantive work:
  - course name or number
  - term or offering
  - canonical workspace path
  - live LMS course target when applicable
  - primary cloud-storage folder or drive identity when applicable
- Keep course-local configuration in the course workspace when it is non-secret and needed for reproducible operations.
- Keep secrets, access tokens, OAuth credentials, and private operator credentials outside synced or broadly shared course folders.
- Preserve existing human-organized course folders unless the user explicitly approves a reorganization.
- Separate active course material from:
  - generated artifacts
  - downloaded exports
  - staged submissions
  - grading work products
  - archives from prior terms
- Treat archives as historical reference surfaces by default; do not mutate archived course material unless the task explicitly concerns archived content.
- Before broad folder cleanup or reorganization, inventory affected paths and state the intended move plan.
- Do not create parallel nested course roots or alternate canonical folders unless the user explicitly requests a migration.
- Keep course-specific operating facts in local policy, notes, or memories rather than relying only on generic shared policy.

## Adoption Notes

Use this module for course folders or repos that function as active instructor workspaces, especially when they combine LMS configuration, course documents, student artifacts, generated exports, and cloud-drive material.
