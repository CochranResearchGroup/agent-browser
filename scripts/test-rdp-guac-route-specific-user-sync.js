#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { delimiter, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const fixtureRoot = mkdtempSync(join(tmpdir(), 'agent-browser-p79-route-sync-'));
const guacDir = join(fixtureRoot, 'guacamole');
const secretFile = join(fixtureRoot, 'guacamole.env');
const stateDir = join(fixtureRoot, 'state');
const binDir = join(fixtureRoot, 'bin');
const sqlPath = join(fixtureRoot, 'route-write.sql');
const dockerLog = join(fixtureRoot, 'docker.jsonl');
const script = resolve('scripts/sync-rdp-guac-route-specific-user-pool.sh');
const legacyScript = resolve('scripts/sync-rdp-guac-existing-user-route-pool.sh');

mkdirSync(join(guacDir, 'init'), { recursive: true });
mkdirSync(binDir, { recursive: true });
writeFileSync(join(guacDir, 'compose.yml'), 'services: {}\n');
writeFileSync(join(guacDir, 'init', '001-initdb.sql'), '-- fixture schema\n');
writeSecrets();

writeFileSync(join(binDir, 'getent'), `#!/usr/bin/env bash
if [[ "$1" == "passwd" && ("$2" == "agent-browser-rdp-a" || "$2" == "agent-browser-rdp-b") ]]; then
  printf '%s:x:2001:2001:fixture:/home/%s:/bin/bash\\n' "$2" "$2"
  exit 0
fi
exit 2
`);
writeFileSync(join(binDir, 'docker'), `#!/usr/bin/env bash
set -euo pipefail
printf '%s\\n' "$*" >> "$P79_DOCKER_LOG"
if [[ "$1" == "inspect" ]]; then
  echo true
  exit 0
fi
if [[ "$*" == *"pg_isready"* ]]; then
  exit 0
fi
if [[ "$*" == *"information_schema.tables"* ]]; then
  printf '%s\\n' guacamole_connection guacamole_connection_parameter guacamole_connection_permission guacamole_entity guacamole_user
  exit 0
fi
if [[ "$*" == *"CHECKPOINT;"* ]]; then
  exit 0
fi
if [[ "$*" == *" psql "* ]]; then
  tee "$P79_SQL_PATH" >/dev/null
  exit 0
fi
echo "unexpected docker fixture command: $*" >&2
exit 3
`);
chmodSync(join(binDir, 'getent'), 0o755);
chmodSync(join(binDir, 'docker'), 0o755);

try {
  const dryRun = run(['--dry-run']);
  assert.equal(dryRun.status, 0, `dry run must succeed: ${dryRun.stdout}${dryRun.stderr}`);
  assert.match(dryRun.stdout, /No Guacamole records were changed\./);
  assert.doesNotMatch(dryRun.stdout, /route-a-secret|route-b-secret/);
  assert.equal(readOptional(sqlPath), '', 'dry run must not execute route SQL');

  const legacyDryRun = run(['--dry-run'], legacyScript);
  assert.equal(
    legacyDryRun.status,
    0,
    `the legacy command must route to the safe migration: ${legacyDryRun.stdout}${legacyDryRun.stderr}`,
  );
  assert.match(legacyDryRun.stderr, /compatibility alias/i);
  assert.match(legacyDryRun.stdout, /route-specific Guacamole route-pool sync dry run/);
  assert.doesNotMatch(legacyDryRun.stdout, /color depth/i);

  const applied = run([]);
  assert.equal(applied.status, 0, `route-specific sync must succeed: ${applied.stdout}${applied.stderr}`);
  assert.match(applied.stdout, /route writes checkpoint completed/);
  assert.doesNotMatch(applied.stdout, /route-a-secret|route-b-secret/);

  const sql = readFileSync(sqlPath, 'utf8');
  assert.match(sql, /BEGIN;/, 'route migration must be transactional');
  assert.match(sql, /COMMIT;/, 'route migration must commit only after postconditions');
  assert.match(sql, /Agent Browser RDP Existing User Route A/);
  assert.match(sql, /Agent Browser RDP Existing User Route B/);
  assert.match(sql, /Agent Browser RDP Route A/);
  assert.match(sql, /Agent Browser RDP Route B/);
  assert.match(sql, /RAISE EXCEPTION[\s\S]*ambiguous/i, 'mixed or duplicate managed rows must fail closed');
  assert.match(sql, /connection_name\s*=/, 'legacy rows must be renamed in place');
  assert.match(sql, /parameter_name\s*=\s*'color-depth'/, 'stale color-depth isolation metadata must be removed');
  assert.match(sql, /agent-browser-rdp-a/);
  assert.match(sql, /agent-browser-rdp-b/);
  assert.match(sql, /postcondition/i, 'the transaction must verify its final topology');

  const dockerCommands = readFileSync(dockerLog, 'utf8');
  assert.match(dockerCommands, /ON_ERROR_STOP=1/);
  assert.match(dockerCommands, /CHECKPOINT;/);
  assert.match(
    dockerCommands,
    /compose --project-name true /,
    'retained Compose project must be inherited by route sync and schema guards',
  );

  const applyFlag = run(['--apply']);
  assert.equal(applyFlag.status, 2, 'sync applies by default and must reject an apply flag');

  writeSecrets({ includeRouteBPassword: false });
  const missingSecret = run(['--dry-run']);
  assert.equal(missingSecret.status, 1, 'missing route-specific secrets must fail closed');
  assert.match(missingSecret.stderr, /route_user_inventory_password_missing/);

  console.log('RDP Guacamole route-specific user sync behavior passed');
} finally {
  rmSync(fixtureRoot, { recursive: true, force: true });
}

function writeSecrets({ includeRouteBPassword = true } = {}) {
  const lines = [
    'XRDP_AGENT_BROWSER_ROUTE_A_USERNAME=agent-browser-rdp-a',
    'XRDP_AGENT_BROWSER_ROUTE_A_PASSWORD=route-a-secret',
    'XRDP_AGENT_BROWSER_ROUTE_B_USERNAME=agent-browser-rdp-b',
  ];
  if (includeRouteBPassword) lines.push('XRDP_AGENT_BROWSER_ROUTE_B_PASSWORD=route-b-secret');
  writeFileSync(secretFile, `${lines.join('\n')}\n`, { mode: 0o600 });
}

function run(args, commandPath = script) {
  return spawnSync('bash', [commandPath, ...args], {
    cwd: process.cwd(),
    encoding: 'utf8',
    env: {
      ...process.env,
      PATH: `${binDir}${delimiter}${process.env.PATH}`,
      AGENT_BROWSER_GUACAMOLE_DIR: guacDir,
      AGENT_BROWSER_GUACAMOLE_SECRET_FILE: secretFile,
      AGENT_BROWSER_GUACAMOLE_STATE_DIR: stateDir,
      AGENT_BROWSER_GUACAMOLE_IDENTITY_FILE: join(stateDir, 'guacamole-postgres-identity.json'),
      P79_DOCKER_LOG: dockerLog,
      P79_SQL_PATH: sqlPath,
    },
  });
}

function readOptional(path) {
  try {
    return readFileSync(path, 'utf8');
  } catch {
    return '';
  }
}
