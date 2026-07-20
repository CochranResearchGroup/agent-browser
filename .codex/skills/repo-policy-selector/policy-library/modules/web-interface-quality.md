---
id: web-interface-quality
title: Web Interface Quality
summary: Set reusable quality expectations for website interaction, accessibility, responsive behavior, states, and performance without hard-coding one brand's visual style.
tags:
  - website
  - accessibility
  - interaction
  - performance
---

## Policy

- Keyboard and focus behavior should work on every meaningful interactive path.
- Prefer native semantics before custom interaction patterns, and ensure visible focus indicators remain intact.
- Design and verify responsive behavior across small, typical, and wide screens rather than assuming one viewport is representative.
- Treat empty, error, sparse, dense, and loading states as part of the interface contract, not as afterthoughts.
- Forms should keep labels, validation feedback, and submission behavior clear and accessible.
- Do not rely on color alone to communicate status, errors, or success.
- Use motion only when it clarifies cause and effect or adds deliberate value, and provide a reduced-motion path.
- Avoid unnecessary layout shift, missing image dimensions, or interaction jank that degrades the live experience.
- Prefer inline guidance and recoverable flows over dead ends or opaque failures.
- Validate meaningful interface changes with a combination of browser review, accessibility awareness, and basic performance checks appropriate to the surface.

## Adoption Notes

Use this module when repos own public-facing website or web-application interfaces and need a reusable baseline for interaction quality.

This module should stay framework-agnostic and avoid encoding one company's brand-specific design language, typography, or copywriting style as universal policy.
