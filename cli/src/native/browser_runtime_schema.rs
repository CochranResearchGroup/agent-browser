// Schema 2 marks durable idle-retirement obligations. A schema-1-only reader
// must not silently ignore those obligations. Existing schema 1 is read without
// migration; the first custody claim promotes it atomically with its record.
pub(crate) const BROWSER_RUNTIME_DATABASE_MAX_SCHEMA: i64 = 8;

// Schema 6 protects archived original task operations from older readers that
// cannot consult their duplicate-execution history. Promoted on first archival.

// Schema7 introduces the refused-no-effect terminal original. Older binaries
// must refuse this database rather than treating failed input as acknowledged
// or forgetting duplicate protection after its terminal record is archived.

// Schema8 protects qualified finite handoff retention. Earlier handoff decoders
// ignored unknown fields and would substitute guest time for retained authority.
// Promote atomically with the first session document carrying retentionTime.
