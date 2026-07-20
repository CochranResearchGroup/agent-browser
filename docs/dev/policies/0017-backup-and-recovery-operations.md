# Policy | Backup And Recovery Operations

## Policy

- Define one durable recovery path for live website state and document it clearly.
- Be explicit about what the backup system is meant to recover, such as:
  - databases
  - uploaded assets
  - runtime configuration
  - owned code snapshots
  - environment inventories
- Distinguish durable recovery artifacts from disposable local staging or pull directories.
- After meaningful live changes, refresh the backup cycle when the repo's recovery contract depends on capturing newly changed live state.
- Recovery procedures must describe both extraction and validation, not only archive creation.
- Restore workflows should identify the minimum artifacts that must exist for a recovery to be considered viable.
- If backup exclusions exist, document them explicitly so maintainers do not assume those surfaces are recoverable.
- Keep backup and restore commands deterministic enough that maintainers can run them without reconstructing missing context from chat history.
## Adoption Notes

Use this module when a repo manages live website or CMS state where recovery depends on a documented archive, backup, or snapshot workflow rather than source control alone.
