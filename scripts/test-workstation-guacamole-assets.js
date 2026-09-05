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

const manifestHashMismatches = []
for (const file of manifest.files) {
  const path = join(assetRoot, file.path)
  assert(existsSync(path), `manifest file missing: ${file.path}`)
  const actualSha256 = sha256(path)
  if (actualSha256 !== file.sha256) {
    manifestHashMismatches.push({ path: file.path, expected: file.sha256, actual: actualSha256 })
  }
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

const textInputTemplateKey = 'app/textInput/templates/guacTextInput.html'
const textInputTemplate = '<div><textarea rows="1" class="target" autocorrect="off" autocapitalize="off" autofocus></textarea></div>'
function runEmbeddedTextInputTemplate({ embedded, migrated = false }) {
  const cache = new Map()
  const calls = []
  const templateCache = { get: (key) => cache.get(key), put: (key, value) => cache.set(key, value) }
  const runBlocks = [() => {
    calls.push('upstream-template-cache')
    templateCache.put(textInputTemplateKey, textInputTemplate)
    templateCache.put('unrelated.html', '<textarea autofocus></textarea>')
  }]
  const window = {
    localStorage: {
      getItem: (key) => migrated && key === 'AGENT_BROWSER_GUAC_DEFAULTS_VERSION' ? '1' : null,
      setItem() {},
    },
    angular: { module(name) {
      if (name === 'textInput') return { config() {} }
      assert.equal(name, 'templates-main')
      return { run(injected) {
        assert.equal(injected[0], '$templateCache')
        runBlocks.push(() => { calls.push('extension-template-hook'); injected[1](templateCache) })
      } }
    } },
  }
  window.parent = embedded ? {} : window
  vm.runInNewContext(defaultsScript, { window })
  assert.equal(cache.size, 0, 'extension registration must precede Angular bootstrap')
  for (const run of runBlocks) run()
  return { cache, calls }
}
for (const migrated of [false, true]) {
  const embedded = runEmbeddedTextInputTemplate({ embedded: true, migrated })
  assert.equal(embedded.cache.get(textInputTemplateKey), textInputTemplate.replace(' autofocus', ''))
  assert.equal(embedded.cache.get('unrelated.html'), '<textarea autofocus></textarea>')
  assert.deepEqual(embedded.calls, ['upstream-template-cache', 'extension-template-hook'])
}
const standalone = runEmbeddedTextInputTemplate({ embedded: false })
assert.equal(standalone.cache.get(textInputTemplateKey), textInputTemplate)
assert.deepEqual(standalone.calls, ['upstream-template-cache'])

// Guacamole 1.5.5 also calls target.focus() synchronously in its text-input
// controller. Removing only the template attribute does not preserve host focus.
for (const embedded of [true, false]) {
  for (const throws of [false, true]) {
    let decorator
    let focusCalls = 0
    const nativeFocus = function () { focusCalls += 1 }
    const target = Object.create({ focus: nativeFocus })
    const scope = {}
    const element = { find: () => [target] }
    const failure = new Error('controller failure')
    const controller = ['$scope', '$element', function ($scope, $element) {
      assert.equal($scope, scope)
      assert.equal($element, element)
      $scope.focusExplicitly = () => target.focus()
      target.focus()
      if (throws) throw failure
      return 'controller result'
    }]
    const directive = { controller }
    const window = { parent: {}, angular: { module(name) {
      if (name === 'templates-main') return { run() {} }
      assert.equal(name, 'textInput')
      return { config(injected) { injected.at(-1)({ decorator(name, injected) {
        assert.equal(name, 'guacTextInputDirective')
        decorator = injected.at(-1)
      } }) } }
    } } }
    if (!embedded) window.parent = window
    const injector = { invoke(injected, receiver, locals) {
      return injected.at(-1).apply(receiver, injected.slice(0, -1).map(name => locals[name]))
    } }
    vm.runInNewContext(defaultsScript, { window })
    if (decorator) decorator([directive], injector)
    const initialize = () => injector.invoke(directive.controller, {}, { $scope: scope, $element: element })
    if (throws) assert.throws(initialize, error => error === failure)
    else assert.equal(initialize(), 'controller result')
    assert.equal(focusCalls, embedded ? 0 : 1, 'embedded controller startup must not steal dashboard focus')
    assert.equal(Object.hasOwn(target, 'focus'), false, 'restore inherited focus even on controller failure')
    assert.equal(target.focus, nativeFocus)
    scope.focusExplicitly()
    assert.equal(focusCalls, embedded ? 1 : 2, 'explicit input focus must remain functional')
  }
}

function createShareAuthSignalHarness({
  frameName = 'agent-browser-guacamole-share:attempt-safe-123',
  href = 'https://agent-browser-dev-share.example.test:8443/guacamole/',
  parentIsSelf = false,
} = {}) {
  const messages = []
  const parent = {
    postMessage(message, targetOrigin) {
      messages.push({ message: JSON.parse(JSON.stringify(message)), targetOrigin })
    },
  }
  class FakeXMLHttpRequest {
    constructor() {
      this.listeners = new Map()
      this.status = 0
    }

    open(method, url) {
      this.method = method
      this.url = url
    }

    addEventListener(type, listener) {
      this.listeners.set(type, listener)
    }

    send() {}

    complete(status) {
      this.status = status
      this.listeners.get('loadend')?.call(this)
    }
  }
  let fetchStatus = 200
  const location = new URL(href)
  const window = {
    XMLHttpRequest: FakeXMLHttpRequest,
    fetch: async () => ({
      status: fetchStatus,
      body: 'private-response-body',
      url: 'https://must-not-cross-frame.example.test/private-response',
    }),
    localStorage: {
      getItem() { return null },
      setItem() {},
    },
    location: {
      href: location.href,
      hostname: location.hostname,
      port: location.port,
      protocol: location.protocol,
    },
    name: frameName,
    parent,
  }
  if (parentIsSelf) window.parent = window
  vm.runInNewContext(defaultsScript, { URL, window })
  return {
    messages,
    requestWithXhr({
      body = 'key=private-share-key&token=private-token&opaque=private-request-body',
      method = 'POST',
      status = 200,
      url = '/guacamole/api/tokens?token=private-token',
    } = {}) {
      const request = new window.XMLHttpRequest()
      request.open(method, url)
      request.send(body)
      request.complete(status)
    },
    async requestWithFetch({
      body = 'key=private-share-key&token=private-token&opaque=private-request-body',
      method = 'POST',
      status = 200,
      url = '/guacamole/api/tokens?token=private-token',
      requestObject = false,
    } = {}) {
      fetchStatus = status
      const input = requestObject ? { method, url } : url
      const init = requestObject ? undefined : { method, body }
      await window.fetch(input, init)
    },
  }
}

function assertPrivacyBoundedShareAuthMessage(actual, outcome) {
  assert.deepEqual(actual, {
    message: {
      type: 'agent-browser-guacamole-share-auth',
      attemptId: 'attempt-safe-123',
      outcome,
    },
    targetOrigin: 'https://agent-browser-dev.example.test:8443',
  })
  const encoded = JSON.stringify(actual)
  for (const forbidden of [
    'private-share-key',
    'private-token',
    'private-request-body',
    'private-response-body',
    'must-not-cross-frame.example.test',
    '/guacamole/api/tokens',
  ]) {
    assert.equal(encoded.includes(forbidden), false, `share auth message leaked ${forbidden}`)
  }
}

for (const transport of ['xhr', 'fetch']) {
  for (const [status, outcome] of [
    [200, 'ready'],
    [400, 'share_key_rejected'],
    [401, 'share_key_rejected'],
    [403, 'share_key_rejected'],
  ]) {
    const harness = createShareAuthSignalHarness()
    if (transport === 'xhr') harness.requestWithXhr({ status })
    else await harness.requestWithFetch({ status })
    assert.equal(harness.messages.length, 1, `${transport} HTTP ${status} signal count`)
    assertPrivacyBoundedShareAuthMessage(harness.messages[0], outcome)
  }
}

for (const scenario of [
  {
    label: 'wrong frame name',
    harness: createShareAuthSignalHarness({ frameName: 'untrusted-frame' }),
    request: { status: 403 },
  },
  {
    label: 'top-level document',
    harness: createShareAuthSignalHarness({ parentIsSelf: true }),
    request: { status: 403 },
  },
  {
    label: 'non-share origin',
    harness: createShareAuthSignalHarness({ href: 'https://agent-browser-dev.example.test/guacamole/' }),
    request: { status: 403 },
  },
  {
    label: 'non-token request',
    harness: createShareAuthSignalHarness(),
    request: { status: 403, url: '/guacamole/api/session/data/postgresql/activeConnections' },
  },
  {
    label: 'non-POST token request',
    harness: createShareAuthSignalHarness(),
    request: { method: 'GET', status: 403 },
  },
  {
    label: 'server failure',
    harness: createShareAuthSignalHarness(),
    request: { status: 500 },
  },
]) {
  scenario.harness.requestWithXhr(scenario.request)
  assert.deepEqual(scenario.harness.messages, [], `${scenario.label} must not signal the parent`)
}

const fetchNegative = createShareAuthSignalHarness()
await fetchNegative.requestWithFetch({ status: 500 })
await fetchNegative.requestWithFetch({ method: 'GET', status: 403 })
await fetchNegative.requestWithFetch({
  status: 403,
  url: '/guacamole/api/session/data/postgresql/activeConnections',
})
await fetchNegative.requestWithFetch({ status: 200, body: 'username=not-a-share-key' })
await fetchNegative.requestWithFetch({ status: 200, requestObject: true })
assert.deepEqual(
  fetchNegative.messages,
  [],
  'fetch must ignore 500, non-POST, non-token, keyless, and uninspectable Request-like traffic',
)

const xhrKeylessSuccess = createShareAuthSignalHarness()
xhrKeylessSuccess.requestWithXhr({ status: 200, body: 'username=not-a-share-key' })
assert.deepEqual(
  xhrKeylessSuccess.messages,
  [],
  'XHR token success without a key-bearing POST body must not signal readiness',
)

// Restricted PostgreSQL share users cannot re-share their tunnel. Exercise
// the same tunnel-service call used by Guacamole's client UI, including auth
// changes after decorator registration and unchanged failures for full users.
async function checkSharedViewerCapabilities({
  frameName = 'agent-browser-guacamole-share:capability-test',
  hostname = 'dashboard-share.example.test',
  embedded = true,
  expectsGuard = true,
} = {}) {
  let decorator
  let dataSource = 'postgresql-shared'
  let anonymous = true
  const calls = []
  const unavailable = new Error('No readable active connection for tunnel.')
  const delegate = {
    getSharingProfiles(tunnel) {
      calls.push({ receiver: this, tunnel })
      return Promise.reject(unavailable)
    },
    getProtocol() { return 'rdp' },
  }
  const window = {
    parent: {}, name: frameName,
    location: { hostname, protocol: 'https:', port: '' },
    angular: { module(name) {
      if (name === 'templates-main') return { run() {} }
      if (name === 'textInput') return { config() {} }
      assert.equal(name, 'rest')
      return { config(injected) {
        assert.equal(injected[0], '$provide')
        injected.at(-1)({ decorator(name, injected) {
          assert.equal(name, 'tunnelService')
          decorator = injected.at(-1)
        } })
      } }
    } },
  }
  if (!embedded) window.parent = window
  vm.runInNewContext(defaultsScript, { window })
  const services = {
    authenticationService: { getDataSource: () => dataSource, isAnonymous: () => anonymous },
    $q: { when: (value) => Promise.resolve(value) },
  }
  const decorated = decorator ? decorator(delegate, { get: (name) => services[name] }) : delegate
  if (!expectsGuard) {
    assert.equal(decorator, undefined)
    await assert.rejects(decorated.getSharingProfiles('unchanged-tunnel'), (error) => error === unavailable)
    return
  }
  assert.deepEqual(JSON.parse(JSON.stringify(await decorated.getSharingProfiles('shared-tunnel'))), {})
  assert.equal(calls.length, 0, 'restricted sharing must not issue the known-unavailable HTTP request')
  assert.equal(decorated.getProtocol(), 'rdp')
  for (const identity of [
    { source: 'postgresql', anonymous: false },
    { source: 'postgresql-shared', anonymous: false },
    { source: 'unknown-shared', anonymous: true },
    { source: null, anonymous: true },
  ]) {
    dataSource = identity.source
    anonymous = identity.anonymous
    await assert.rejects(decorated.getSharingProfiles('full-tunnel'), (error) => error === unavailable)
    assert.equal(calls.at(-1).receiver, delegate)
    assert.equal(calls.at(-1).tunnel, 'full-tunnel')
  }
}
await checkSharedViewerCapabilities()
await checkSharedViewerCapabilities({ frameName: '', expectsGuard: false })
await checkSharedViewerCapabilities({ hostname: 'dashboard.example.test', expectsGuard: false })
await checkSharedViewerCapabilities({ embedded: false, expectsGuard: false })

assert.deepEqual(manifestHashMismatches, [], 'workstation asset hashes must match the manifest')

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
