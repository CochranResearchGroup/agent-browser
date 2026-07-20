# Policy | Website Surface Targeting

## Policy

- Before changing code, content artifacts, migrations, or validation targets, state which website surface the work is meant to affect.
- Treat multiple surfaces as distinct until proven otherwise, for example:
  - canonical public site
  - staging or preview site
  - local review mirror
  - deprecated legacy surface
  - secondary or nested site
- Do not assume a change to one tracked code path affects the canonical public experience.
- Verify which runtime, hostname, path root, or deploy target a proposed change actually reaches before editing or releasing.
- Keep the canonical public surface explicit in repo docs so validation and release checks do not drift to the wrong environment.
- Mark deprecated or non-production surfaces explicitly and treat them as opt-in targets, not default release paths.
- When a repo maintains both legacy and current surfaces, document the boundary between them and keep routine workflows pointed at the current surface by default.
- Validation should target the same surface the slice intends to affect; do not treat a passing check on one surface as sufficient evidence for another.
## Adoption Notes

Use this module when a repo manages website work across more than one meaningful surface, especially when local mirrors, preview hosts, nested sites, legacy domains, or deprecated installations coexist.
