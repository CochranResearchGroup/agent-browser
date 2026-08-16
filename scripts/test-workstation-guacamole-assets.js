#!/usr/bin/env node

import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import vm from 'node:vm'

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const assetRoot = join(repoRoot, 'cli/assets/workstation/guacamole')
const manifestPath = join(assetRoot, 'manifest.json')
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'))

function sha256(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex')
}

function imageRef(serviceName) {
  const service = manifest.images.find((candidate) => candidate.service === serviceName)
  assert(service, `manifest image entry missing for ${serviceName}`)
  return `${service.repository}:${service.tag}@sha256:${service.digest}`
}

assert.equal(manifest.schemaVersion, 1)
assert.equal(manifest.bundle, 'agent-browser-guacamole-workstation')
assert.equal(manifest.schema.generatorImage, imageRef('guacamole'))

for (const file of manifest.files) {
  const path = join(assetRoot, file.path)
  assert(existsSync(path), `manifest file missing: ${file.path}`)
  assert.equal(sha256(path), file.sha256, `hash mismatch: ${file.path}`)
}

const compose = readFileSync(join(assetRoot, 'compose.yml'), 'utf8')
for (const serviceName of ['postgres', 'guacd', 'guacamole']) {
  assert(
    compose.includes(`image: ${imageRef(serviceName)}`),
    `compose image is not pinned to manifest digest: ${serviceName}`,
  )
  assert.equal(
    manifest.images.find((candidate) => candidate.service === serviceName).platform,
    'linux/amd64',
  )
}
assert.equal(
  (compose.match(/^\s+platform: linux\/amd64$/gm) || []).length,
  3,
  'all services must declare the validated image platform',
)
assert.match(
  compose,
  /guacd:\n[\s\S]*?healthcheck:\n[\s\S]*?interval: 5s\n[\s\S]*?start_period: 5s/,
  'guacd must override the image five-minute health interval',
)

assert.match(
  compose,
  /127\.0\.0\.1:\$\{AGENT_BROWSER_GUACAMOLE_HTTP_PORT:-8092\}:8080/,
)
assert.doesNotMatch(compose, /AGENT_BROWSER_GUACAMOLE_BIND_ADDRESS/)
assert.equal((compose.match(/^\s+ports:/gm) || []).length, 1, 'only the web service may publish ports')
assert.match(compose, /agent-browser-guacamole-postgres-data:\/var\/lib\/postgresql\/data/)
assert.match(compose, /\.\/init:\/docker-entrypoint-initdb\.d:ro/)
assert.match(compose, /\.\/extensions:\/etc\/guacamole\/extensions:ro/)
assert.match(compose, /\.\/start-guacamole\.sh:\/opt\/agent-browser\/start-guacamole\.sh:ro/)
assert.match(compose, /entrypoint: \["\/bin\/bash", "\/opt\/agent-browser\/start-guacamole\.sh"\]/)
assert.match(compose, /^\s+GUACAMOLE_HOME: \/etc\/guacamole$/m)
assert.match(compose, /agent-browser-guacamole-postgres-data:\n\s+name: agent-browser-guacamole-postgres-data\n\s+external: true/)
assert.doesNotMatch(compose, /POSTGRES_PASSWORD:\s+[^\s$]/)
assert.equal(
  compose.match(
    /^\s+(?:POSTGRES|POSTGRESQL)_PASSWORD: \$\{POSTGRES_PASSWORD:\?[^}]+\}/gm,
  )?.length,
  2,
  'both database clients must require the injected secret',
)

const schema = readFileSync(join(assetRoot, manifest.schema.path), 'utf8')
for (const relation of [
  'guacamole_entity',
  'guacamole_user',
  'guacamole_connection',
  'guacamole_connection_parameter',
  'guacamole_connection_permission',
]) {
  assert(schema.includes(`CREATE TABLE ${relation}`), `schema relation missing: ${relation}`)
}
assert.equal(sha256(join(assetRoot, manifest.schema.path)), manifest.schema.sha256)

const defaultsManifestPath = join(assetRoot, 'extensions/guac-manifest.json')
const defaultsScriptPath = join(assetRoot, 'extensions/agent-browser-defaults.js')
const defaultsManifest = JSON.parse(readFileSync(defaultsManifestPath, 'utf8'))
const defaultsScript = readFileSync(defaultsScriptPath, 'utf8')
const guacamoleStart = readFileSync(join(assetRoot, 'start-guacamole.sh'), 'utf8')

assert.match(guacamoleStart, /cp -R "\$template_source\/\." "\$writable_template\/"/)
assert.match(guacamoleStart, /chmod -R u\+rwX "\$writable_template"/)
assert.match(guacamoleStart, /exec \/opt\/guacamole\/bin\/start\.sh/)

assert.equal(defaultsManifest.guacamoleVersion, '1.5.5')
assert.equal(defaultsManifest.namespace, 'agent-browser-defaults')
assert.deepEqual(defaultsManifest.js, ['agent-browser-defaults.js'])

function runDefaultsMigration(initialEntries = {}) {
  const entries = new Map(Object.entries(initialEntries))
  const localStorage = {
    getItem(key) {
      return entries.has(key) ? entries.get(key) : null
    },
    setItem(key, value) {
      entries.set(key, String(value))
    },
  }
  vm.runInNewContext(defaultsScript, { window: { localStorage } })
  return entries
}

const emptyOrigin = runDefaultsMigration()
assert.equal(JSON.parse(emptyOrigin.get('GUAC_PREFERENCES')).inputMethod, 'text')
assert.equal(emptyOrigin.get('AGENT_BROWSER_GUAC_DEFAULTS_VERSION'), '1')

const priorDefault = runDefaultsMigration({
  GUAC_PREFERENCES: JSON.stringify({ inputMethod: 'none', emulateAbsoluteMouse: false }),
})
assert.deepEqual(JSON.parse(priorDefault.get('GUAC_PREFERENCES')), {
  inputMethod: 'text',
  emulateAbsoluteMouse: false,
})

const migratedOverride = runDefaultsMigration({
  GUAC_PREFERENCES: JSON.stringify({ inputMethod: 'none' }),
  AGENT_BROWSER_GUAC_DEFAULTS_VERSION: '1',
})
assert.equal(JSON.parse(migratedOverride.get('GUAC_PREFERENCES')).inputMethod, 'none')

const generator = readFileSync(join(assetRoot, 'generate-initdb.sh'), 'utf8')
assert(generator.includes(`readonly GUACAMOLE_IMAGE='${manifest.schema.generatorImage}'`))
assert(generator.includes(`readonly EXPECTED_SHA256='${manifest.schema.sha256}'`))

let resolvedCompose
try {
  resolvedCompose = JSON.parse(
    execFileSync(
      'docker',
      [
        'compose',
        '--env-file',
        join(assetRoot, 'environment.example'),
        '-f',
        join(assetRoot, 'compose.yml'),
        'config',
        '--format',
        'json',
      ],
      {
        cwd: assetRoot,
        env: { ...process.env, POSTGRES_PASSWORD: 'static-validation-placeholder' },
        stdio: 'pipe',
      },
    ).toString(),
  )
} catch (error) {
  const stderr = error.stderr?.toString().trim()
  assert.fail(`docker compose static validation failed${stderr ? `: ${stderr}` : ''}`)
}

assert.deepEqual(Object.keys(resolvedCompose.services).sort(), ['guacamole', 'guacd', 'postgres'])
assert.equal(resolvedCompose.services.postgres.ports, undefined)
assert.equal(resolvedCompose.services.guacd.ports, undefined)
assert.equal(resolvedCompose.services.guacamole.ports.length, 1)
assert.equal(resolvedCompose.services.guacamole.ports[0].host_ip, '127.0.0.1')
assert.equal(resolvedCompose.services.guacamole.ports[0].target, 8080)
assert.equal(resolvedCompose.volumes['agent-browser-guacamole-postgres-data'].external, true)
assert.equal(
  resolvedCompose.volumes['agent-browser-guacamole-postgres-data'].name,
  'agent-browser-guacamole-postgres-data',
)

console.log('workstation Guacamole asset validation passed')
