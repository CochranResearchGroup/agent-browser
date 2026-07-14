#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const ensureSource = readFileSync('scripts/ensure-rdp-guac-postgres.sh', 'utf8');

for (const table of [
  'guacamole_user',
  'guacamole_entity',
  'guacamole_connection',
  'guacamole_connection_parameter',
  'guacamole_connection_permission',
]) {
  assert.match(
    ensureSource,
    new RegExp(`\\b${table}\\b`),
    `ensure-rdp-guac-postgres.sh must check required table ${table}`,
  );
}

assert.match(
  ensureSource,
  /pg_isready -U "\$POSTGRES_USER" -d "\$POSTGRES_DB"/,
  'ensure-rdp-guac-postgres.sh must wait for Postgres readiness',
);
assert.match(
  ensureSource,
  /Guacamole schema is partial; refusing automatic full schema import\./,
  'ensure-rdp-guac-postgres.sh must refuse automatic import into partial schemas',
);
assert.match(
  ensureSource,
  /Existing guacamole_\* relation count:/,
  'ensure-rdp-guac-postgres.sh must report partial schema relation count',
);
assert.match(
  ensureSource,
  /-v ON_ERROR_STOP=1 < "\$INIT_SQL"/,
  'ensure-rdp-guac-postgres.sh must import schema with ON_ERROR_STOP',
);
assert.match(
  ensureSource,
  /-c "CHECKPOINT;"/,
  'ensure-rdp-guac-postgres.sh must checkpoint after schema readiness or import',
);

for (const file of [
  'scripts/setup-rdp-guac-route-pool.sh',
  'scripts/sync-rdp-guac-existing-user-route-pool.sh',
  'scripts/setup-rdp-autologin-user.sh',
]) {
  const source = readFileSync(file, 'utf8');
  assert.match(
    source,
    /ensure-rdp-guac-postgres\.sh" --apply/,
    `${file} must run the guarded Guacamole Postgres setup before route writes`,
  );
  assert.match(
    source,
    /psql -U guacamole_user -d guacamole_db -v ON_ERROR_STOP=1/,
    `${file} must write Guacamole route records with ON_ERROR_STOP`,
  );
  assert.match(
    source,
    /-c "CHECKPOINT;"/,
    `${file} must checkpoint after Guacamole route writes`,
  );
  assert.match(
    source,
    /route writes checkpoint completed/,
    `${file} must report route-write checkpoint completion`,
  );
}

console.log('RDP Guacamole Postgres hardening guard passed');
