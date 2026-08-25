#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  utimesSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const operatorPath = 'scripts/guacamole-postgres-durability.sh';

assert.equal(
  existsSync(operatorPath),
  true,
  'the Guacamole PostgreSQL durability operator must exist',
);

const source = readFileSync(operatorPath, 'utf8');

for (const contract of [
  'status',
  'backup',
  'restore-drill',
  'record-identity',
  'allow-stale-source',
  'stale_wsl_bind_mount',
  'cluster_identity_mismatch',
  'mount_identity_mismatch',
  'pg_dump',
  'pg_restore --list',
  'sha256sum',
  'mktemp',
  'flock',
  'residual_drill_count',
  'guacamole_connection_permission',
  '.keep',
]) {
  assert.match(source, new RegExp(contract.replaceAll('-', '\\-')));
}

const fixtureRoot = mkdtempSync(join(tmpdir(), 'agent-browser-guac-durability-'));
const fakeDocker = join(fixtureRoot, 'docker');
writeFileSync(
  fakeDocker,
  `#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" == "inspect" && "$2" == "-f" ]]; then
  echo true
elif [[ "$1" == "inspect" ]]; then
  if [[ "$*" == *".Type"* ]]; then
    echo "\${FAKE_DECLARED_TYPE:-bind}"
  else
    echo "\${FAKE_MOUNT_SOURCE:-/fixture/postgres}"
  fi
elif [[ "$1" == "exec" && "$*" == *"pg_isready"* ]]; then
  exit 0
elif [[ "$1" == "exec" && "$*" == *"pg_control_system"* ]]; then
  echo "\${FAKE_SYSTEM_ID:-12345}"
elif [[ "$1" == "exec" && "$*" == *"mountinfo"* ]]; then
  echo "100 90 0:32 /fixture /var/lib/postgresql/data rw - \${FAKE_FSTYPE:-tmpfs} none rw"
elif [[ "$1" == "exec" && "$*" == *"pg_dump"* ]]; then
  printf 'fixture-custom-dump'
elif [[ "$1" == "exec" && "$*" == *"pg_restore --list"* ]]; then
  printf '; fixture catalog\\nTABLE DATA public guacamole_connection\\n'
elif [[ "$1" == "exec" && "$*" == *"pg_restore"* ]]; then
  cat >/dev/null
elif [[ "$1" == "exec" && "$*" == *"createdb"* ]]; then
  exit 0
elif [[ "$1" == "exec" && "$*" == *"dropdb"* ]]; then
  [[ "\${FAKE_DROP_FAIL:-0}" == "1" ]] && exit 7
  exit 0
elif [[ "$1" == "exec" && "$*" == *"information_schema.tables"* ]]; then
  echo 5
elif [[ "$1" == "exec" && "$*" == *"guacamole_connection_permission"* ]]; then
  echo 4
elif [[ "$1" == "exec" && "$*" == *"guacamole_connection"* ]]; then
  echo 2
elif [[ "$1" == "exec" && "$*" == *"pg_database where datname"* ]]; then
  echo 0
else
  echo "unexpected fake docker invocation: $*" >&2
  exit 9
fi
`,
);
chmodSync(fakeDocker, 0o755);

try {
  const stale = spawnSync('bash', [operatorPath, 'status'], {
    cwd: process.cwd(),
    encoding: 'utf8',
    env: {
      ...process.env,
      PATH: `${fixtureRoot}:${process.env.PATH}`,
      AGENT_BROWSER_GUACAMOLE_IDENTITY_FILE: join(fixtureRoot, 'missing.json'),
    },
  });
  assert.equal(stale.status, 1);
  assert.match(stale.stdout, /issue=stale_wsl_bind_mount/);

  const identityPath = join(fixtureRoot, 'identity.json');
  writeFileSync(identityPath, JSON.stringify({ systemIdentifier: 'old-cluster' }));
  const mismatch = spawnSync(
    'bash',
    [operatorPath, 'status', '--require-continuity'],
    {
      cwd: process.cwd(),
      encoding: 'utf8',
      env: {
        ...process.env,
        PATH: `${fixtureRoot}:${process.env.PATH}`,
        AGENT_BROWSER_GUACAMOLE_IDENTITY_FILE: identityPath,
        FAKE_DECLARED_TYPE: 'volume',
        FAKE_FSTYPE: 'ext4',
        FAKE_SYSTEM_ID: 'new-cluster',
      },
    },
  );
  assert.equal(mismatch.status, 1);
  assert.match(mismatch.stdout, /issue=cluster_identity_mismatch/);

  writeFileSync(
    identityPath,
    JSON.stringify({
      systemIdentifier: '12345',
      declaredMountType: 'bind',
      runningMountFilesystem: 'ext4',
      mountSource: '/old/postgres',
    }),
  );
  const mountMismatch = spawnSync(
    'bash',
    [operatorPath, 'status', '--require-continuity'],
    {
      cwd: process.cwd(),
      encoding: 'utf8',
      env: {
        ...process.env,
        PATH: `${fixtureRoot}:${process.env.PATH}`,
        AGENT_BROWSER_GUACAMOLE_IDENTITY_FILE: identityPath,
        FAKE_DECLARED_TYPE: 'volume',
        FAKE_FSTYPE: 'ext4',
        FAKE_SYSTEM_ID: '12345',
        FAKE_MOUNT_SOURCE: '/new/postgres',
      },
    },
  );
  assert.equal(mountMismatch.status, 1);
  assert.match(mountMismatch.stdout, /issue=mount_identity_mismatch/);
  const refusedRebind = spawnSync('bash', [operatorPath, 'record-identity'], {
    cwd: process.cwd(),
    encoding: 'utf8',
    env: {
      ...process.env,
      PATH: `${fixtureRoot}:${process.env.PATH}`,
      AGENT_BROWSER_GUACAMOLE_IDENTITY_FILE: identityPath,
      FAKE_DECLARED_TYPE: 'volume',
      FAKE_FSTYPE: 'ext4',
      FAKE_SYSTEM_ID: '12345',
      FAKE_MOUNT_SOURCE: '/new/postgres',
    },
  });
  assert.notEqual(refusedRebind.status, 0);
  assert.equal(
    JSON.parse(readFileSync(identityPath, 'utf8')).mountSource,
    '/old/postgres',
  );

  writeFileSync(
    identityPath,
    JSON.stringify({
      systemIdentifier: '12345',
      declaredMountType: 'volume',
      runningMountFilesystem: 'ext4',
      mountSource: '/new/postgres',
    }),
  );
  const backupDir = join(fixtureRoot, 'backups');
  mkdirSync(backupDir);
  const protectedBase = join(
    backupDir,
    'guacamole-postgres-20260101T000000-000000000Z',
  );
  const unprotectedBase = join(
    backupDir,
    'guacamole-postgres-20260102T000000-000000000Z',
  );
  for (const base of [protectedBase, unprotectedBase]) {
    writeFileSync(`${base}.dump`, 'old dump');
    writeFileSync(`${base}.json`, '{}');
    const fixtureTime = new Date('2026-01-02T00:00:00.000Z');
    utimesSync(`${base}.dump`, fixtureTime, fixtureTime);
    utimesSync(`${base}.json`, fixtureTime, fixtureTime);
  }
  writeFileSync(`${protectedBase}.keep`, 'protected\n');

  const backup = spawnSync('bash', [operatorPath, 'backup'], {
    cwd: process.cwd(),
    encoding: 'utf8',
    env: {
      ...process.env,
      PATH: `${fixtureRoot}:${process.env.PATH}`,
      AGENT_BROWSER_GUACAMOLE_IDENTITY_FILE: identityPath,
      AGENT_BROWSER_GUACAMOLE_BACKUP_DIR: backupDir,
      AGENT_BROWSER_GUACAMOLE_BACKUP_RETENTION: '1',
      AGENT_BROWSER_GUACAMOLE_BACKUP_WAIT_ATTEMPTS: '1',
      FAKE_DECLARED_TYPE: 'volume',
      FAKE_FSTYPE: 'ext4',
      FAKE_SYSTEM_ID: '12345',
      FAKE_MOUNT_SOURCE: '/new/postgres',
    },
  });
  assert.equal(backup.status, 0, backup.stderr);
  assert.equal(existsSync(`${protectedBase}.dump`), true);
  assert.equal(existsSync(`${unprotectedBase}.dump`), false);
  const published = readdirSync(backupDir).filter((name) =>
    name.endsWith('.dump'),
  );
  const publishedNew = published.find(
    (name) => !name.includes('20260101') && !name.includes('20260102'),
  );
  assert.ok(publishedNew);
  const publishedPath = join(backupDir, publishedNew);
  assert.equal(existsSync(`${publishedPath.slice(0, -5)}.json`), true);

  const orphanPath = join(
    backupDir,
    'guacamole-postgres-20990101T000000-000000000Z.dump',
  );
  writeFileSync(orphanPath, 'orphan');
  const drill = spawnSync('bash', [operatorPath, 'restore-drill'], {
    cwd: process.cwd(),
    encoding: 'utf8',
    env: {
      ...process.env,
      PATH: `${fixtureRoot}:${process.env.PATH}`,
      AGENT_BROWSER_GUACAMOLE_IDENTITY_FILE: identityPath,
      AGENT_BROWSER_GUACAMOLE_BACKUP_DIR: backupDir,
      FAKE_DECLARED_TYPE: 'volume',
      FAKE_FSTYPE: 'ext4',
      FAKE_SYSTEM_ID: '12345',
      FAKE_MOUNT_SOURCE: '/new/postgres',
    },
  });
  assert.equal(drill.status, 0, drill.stderr);
  assert.match(drill.stdout, /restore_drill=passed/);
  assert.doesNotMatch(drill.stdout, /20990101/);

  const cleanupFailure = spawnSync(
    'bash',
    [operatorPath, 'restore-drill', '--backup', publishedPath],
    {
      cwd: process.cwd(),
      encoding: 'utf8',
      env: {
        ...process.env,
        PATH: `${fixtureRoot}:${process.env.PATH}`,
        AGENT_BROWSER_GUACAMOLE_IDENTITY_FILE: identityPath,
        AGENT_BROWSER_GUACAMOLE_BACKUP_DIR: backupDir,
        FAKE_DECLARED_TYPE: 'volume',
        FAKE_FSTYPE: 'ext4',
        FAKE_SYSTEM_ID: '12345',
        FAKE_MOUNT_SOURCE: '/new/postgres',
        FAKE_DROP_FAIL: '1',
      },
    },
  );
  assert.notEqual(cleanupFailure.status, 0);
  assert.doesNotMatch(cleanupFailure.stdout, /restore_drill=passed/);
} finally {
  rmSync(fixtureRoot, { recursive: true, force: true });
}

const ensureSource = readFileSync('scripts/ensure-rdp-guac-postgres.sh', 'utf8');
assert.match(
  ensureSource,
  /guacamole-postgres-durability\.sh" status --require-continuity/,
  'schema assurance must require durability continuity before importing schema',
);
assert.match(
  ensureSource,
  /guacamole-postgres-durability\.sh" record-identity/g,
  'schema assurance must establish identity after first complete schema readiness',
);

const installerSource = readFileSync(
  'scripts/install-dashboard-user-service.sh',
  'utf8',
);
for (const contract of [
  'agent-browser-guacamole-postgres-backup.service',
  'agent-browser-guacamole-postgres-backup.timer',
  'OnCalendar=daily',
  'Persistent=true',
  'Restart=on-failure',
  'POSTGRES_DURABILITY_BIN',
]) {
  assert.match(installerSource, new RegExp(contract));
}

console.log('Guacamole PostgreSQL durability contract passed');
