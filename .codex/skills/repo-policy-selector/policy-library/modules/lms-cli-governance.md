---
id: lms-cli-governance
title: LMS CLI Governance
summary: Keep LMS-backed command-line operations scoped, read-before-write, and validated when CLI actions can affect live courses.
tags:
  - course
  - lms
  - canvas
  - cli
---

## Policy

- Treat LMS CLI actions as live course operations when they target a production course.
- Confirm the active course, environment, and folder-scoped configuration before any live write.
- Prefer read-only inspection before mutation, for example:
  - list
  - show
  - export
  - doctor
  - planner
  - dry-run import
- Use explicit apply or write flags only after inspecting the planned change or after the user clearly requests the live write.
- Prefer the course workspace's configured default course over ad hoc repeated course ids, unless the task is verifying or overriding scope.
- Treat spreadsheet import/export, ObjectView-style sync, bulk assignment updates, messaging, file publication, page edits, module edits, quiz edits, and grade-related commands as live course operations.
- Keep local LMS exports, audit outputs, and temporary machine-readable products in a documented generated-artifact area unless a repo-local policy names a more specific location.
- After changing LMS content or settings, validate with a matching read-only command or export.
- Before applying a live LMS write, confirm:
  - active course target
  - target environment
  - input artifact source
  - intended audience
  - whether student data, grades, feedback, or answer keys could be exposed or modified

## Adoption Notes

Use this module when a repo or folder drives Canvas, Moodle, Blackboard, Google Classroom, or another LMS through a CLI or automation layer.

Canvas CLI is a concrete instance of this pattern, but the policy is intentionally LMS-generic.
