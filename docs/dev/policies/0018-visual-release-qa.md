# Policy | Visual Release QA

## Policy

- Do not treat a passing code deploy as sufficient evidence for design-facing or UX-facing website changes.
- For meaningful visual changes, validate the exact changed page, flow, or component rather than checking only a homepage or generic smoke route.
- Run visual review on the intended review surface before release.
- Capture durable review evidence when the change is visually meaningful, such as:
  - screenshots
  - focused crops
  - concise review notes
  - targeted QA artifacts
- Include basic functional confirmation for the primary call to action or interactive flow affected by the change.
- When automated browser or performance checks exist, use them as supporting evidence rather than a substitute for visual review.
- After release, verify the live surface that users actually see, not only the local or staging surface.
- If the release contract includes recovery or backup refresh after live changes, treat that as part of release completeness rather than optional cleanup.
## Adoption Notes

Use this module when repos ship website changes where layout, styling, copy presentation, or interaction quality matters materially and cannot be validated by code-level tests alone.
