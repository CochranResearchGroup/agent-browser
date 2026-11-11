// Schema 2 marks durable idle-retirement obligations. A schema-1-only reader
// must not silently ignore those obligations. Existing schema 1 is read without
// migration; the first custody claim promotes it atomically with its record.
pub(crate) const BROWSER_RUNTIME_DATABASE_MAX_SCHEMA: i64 = 5;
