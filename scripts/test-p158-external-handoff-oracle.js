#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

import {
  auditExternalHandoffSession,
  classifyHost,
  classifyOperatorUrl,
} from './lib/p158-external-handoff-oracle.js';

const root = new URL('..', import.meta.url).pathname;

function readJson(relativePath) {
  return JSON.parse(readFileSync(join(root, relativePath), 'utf8'));
}

const fixtureSet = readJson('docs/dev/fixtures/p158/external-handoff-sessions.v1.json');
const fixtureSchema = readJson('docs/dev/contracts/p158-external-handoff-fixtures.v1.schema.json');
const reportSchema = readJson('docs/dev/contracts/p158-external-handoff-oracle-report.v1.schema.json');
const urlRoles = fixtureSchema.$defs.urlRole.enum;
const findingCodes = fixtureSchema.$defs.findingCode.enum;
const hostClasses = reportSchema.$defs.hostClass.enum;
const identityFields = fixtureSchema.$defs.identity.required;
const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormats(ajv);
const validateFixtureSet = ajv.compile(fixtureSchema);
const validateReport = ajv.compile(reportSchema);

function clone(value) {
  return structuredClone(value);
}

function sorted(values) {
  return [...values].sort();
}

function assertValid(validate, value, label) {
  assert.equal(
    validate(value),
    true,
    `${label} violates its JSON Schema: ${ajv.errorsText(validate.errors, { separator: '; ' })}`,
  );
}

function audit(session) {
  return auditExternalHandoffSession({
    session,
    options: {
      auditId: `p158-handoff-audit:${session.fixtureId}`,
      auditedAt: '2026-09-02T23:00:00.000Z',
    },
  });
}

function runTest(name, body) {
  try {
    body();
    process.stdout.write(`PASS ${name}\n`);
  } catch (error) {
    error.message = `${name}: ${error.message}`;
    throw error;
  }
}

runTest('accepts the frozen external-handoff fixture corpus', () => {
  assertValid(validateFixtureSet, fixtureSet, 'external-handoff-sessions.v1.json');
  assert.equal(fixtureSet.syntheticOnly, true);
  assert.deepEqual(
    sorted(new Set(fixtureSet.sessions.flatMap((session) => session.expectedFindingCodes))),
    sorted(findingCodes),
    'fixture corpus does not cover every external-handoff finding',
  );
  assert.deepEqual(
    sorted(new Set(fixtureSet.sessions.flatMap(
      (session) => session.urlObservations.map((observation) => observation.role),
    ))),
    sorted(urlRoles),
    'fixture corpus does not scan every client-visible and diagnostic URL role',
  );
});

runTest('classifies every forbidden host family without trusting URL spelling', () => {
  const hostMatrix = [
    ['handoff.public.example', 'public'],
    ['localhost', 'localhost_literal'],
    ['app.localhost', 'localhost_literal'],
    ['127.0.0.1', 'ipv4_loopback'],
    ['::1', 'ipv6_loopback'],
    ['10.1.2.3', 'rfc1918'],
    ['172.16.2.3', 'rfc1918'],
    ['192.168.2.3', 'rfc1918'],
    ['fd00::1', 'rfc1918'],
    ['169.254.1.2', 'link_local'],
    ['fe80::1', 'link_local'],
    ['agent-browser.local', 'local_domain'],
  ];
  for (const [hostname, expected] of hostMatrix) assert.equal(classifyHost(hostname), expected, hostname);

  const urlMatrix = [
    ['https://handoff.public.example/remote-view/opaque', 'starting_handoff', 'public'],
    ['https://localhost/internal', 'location_header', 'localhost_literal'],
    ['https://127.0.0.1/internal', 'iframe_src', 'ipv4_loopback'],
    ['https://[::1]/internal', 'form_action', 'ipv6_loopback'],
    ['wss://10.0.0.4/socket', 'websocket_endpoint', 'rfc1918'],
    ['https://169.254.169.254/latest', 'error_action', 'link_local'],
    ['https://agent-browser.local/reconnect', 'reconnect_target', 'local_domain'],
    ['https://handoff.public.example/guacamole/#/client/raw', 'location_header', 'raw_guacamole'],
    ['not a URL', 'starting_handoff', 'invalid'],
  ];
  for (const [url, role, expected] of urlMatrix) {
    assert.equal(classifyOperatorUrl(url, { role }).hostClass, expected, `${role} ${url}`);
  }
  assert.deepEqual(
    sorted(new Set(urlMatrix.map(([url, role]) => classifyOperatorUrl(url, { role }).hostClass))),
    sorted(hostClasses),
  );

  const rebinding = classifyOperatorUrl('https://handoff.public.example/remote-view/opaque', {
    role: 'starting_handoff',
    resolvedAddresses: ['127.0.0.1', '10.0.0.9'],
  });
  assert.ok(rebinding.findingCodes.includes('loopback_url_leak'));
  assert.ok(rebinding.findingCodes.includes('private_network_url_leak'));
});

const originalFixtureSet = clone(fixtureSet);
const reports = fixtureSet.sessions.map((session) => ({ session, report: audit(session) }));

runTest('emits deterministic schema-valid reports without mutation or repair', () => {
  for (const { session, report } of reports) {
    assertValid(validateReport, report, session.fixtureId);
    assert.deepEqual(audit(clone(session)), report, `${session.fixtureId} audit is not deterministic`);
    assert.equal(report.repairAttempted, false);
    assert.ok(report.findings.every((finding) => finding.repairAttempted === false));
    assert.deepEqual(
      report.findings.map((finding) => finding.findingId),
      sorted(report.findings.map((finding) => finding.findingId)),
      `${session.fixtureId} findings are not deterministically ordered`,
    );
  }
  assert.deepEqual(fixtureSet, originalFixtureSet, 'oracle mutated the fixture corpus');
});

for (const { session, report } of reports) {
  runTest(`classifies ${session.fixtureId} exactly`, () => {
    const actualCodes = sorted(new Set(report.findings.map((finding) => finding.code)));
    assert.deepEqual(actualCodes, sorted(session.expectedFindingCodes));
    assert.equal(report.passed, session.expectedFindingCodes.length === 0);
    assert.equal(report.summary.urlObservationCount, report.urlClassifications.length);
    const synthesizedStartingHandoff = session.urlObservations.some(
      (observation) => observation.role === 'starting_handoff',
    ) ? 0 : 1;
    assert.equal(
      report.summary.urlObservationCount,
      session.urlObservations.length + synthesizedStartingHandoff,
    );
    assert.equal(report.summary.ingressCheckCount, session.ingressChecks.length);
    assert.equal(report.summary.reconnectCount, session.reconnects.length);
    assert.equal(report.summary.findingCount, report.findings.length);
    for (const code of findingCodes) {
      assert.equal(
        report.summary.findingCounts[code],
        report.findings.filter((finding) => finding.code === code).length,
        `${session.fixtureId} ${code} count drifted`,
      );
    }
  });
}

runTest('never accepts public or local success as fallback for a loopback handoff', () => {
  const clean = fixtureSet.sessions.find((session) => session.expectedFindingCodes.length === 0);
  assert.ok(clean, 'fixture corpus has no clean external session');
  const loopback = clone(clean);
  loopback.fixtureId = 'handoff-loopback-no-fallback-probe';
  loopback.initialHandoffUrl = `https://127.0.0.1/remote-view/${loopback.initialHandoffId}`;
  const starting = loopback.urlObservations.find((observation) => observation.role === 'starting_handoff');
  starting.url = loopback.initialHandoffUrl;
  const result = audit(loopback);
  assert.equal(result.passed, false);
  assert.ok(result.findings.some((finding) => finding.code === 'loopback_url_leak'));
  assert.ok(loopback.ingressChecks.every((check) => check.state === 'passed'));
});

runTest('requires public HTTPS ingress, ready gating, and exact retained identity', () => {
  const clean = fixtureSet.sessions.find((session) => session.expectedFindingCodes.length === 0);
  assert.ok(clean, 'fixture corpus has no clean external session');
  const handoffUrl = new URL(clean.initialHandoffUrl);
  assert.equal(handoffUrl.protocol, 'https:');
  assert.equal(classifyHost(handoffUrl.hostname), 'public');
  assert.equal(handoffUrl.pathname, `/remote-view/${clean.initialHandoffId}`);
  assert.deepEqual(clean.externalVantage, {
    runnerId: clean.externalVantage.runnerId,
    outsideServiceHost: true,
    outsideServiceNetworkNamespace: true,
    publicEgressObserved: true,
  });
  assert.equal(clean.operatorVisibleState, 'ready');
  assert.ok(Date.parse(clean.readyObservedAt) <= Date.parse(clean.firstUsablePixelsAt));
  assert.deepEqual(
    sorted(clean.ingressChecks.map((check) => check.kind)),
    sorted(['dns', 'tls', 'redirect', 'cookie', 'websocket', 'iframe', 'form_action', 'reconnect']),
  );
  assert.ok(clean.ingressChecks.every((check) => check.state === 'passed'));
  assert.deepEqual(sorted(Object.keys(clean.expectedIdentity)), sorted(identityFields));
  for (const reconnect of clean.reconnects) {
    assert.equal(reconnect.handoffId, clean.initialHandoffId);
    assert.equal(reconnect.handoffUrl, clean.initialHandoffUrl);
    assert.equal(reconnect.state, 'passed');
    assert.equal(reconnect.operatorVisibleState, 'ready');
    assert.equal(reconnect.physicalBrowserLaunchCount, 0);
    assert.deepEqual(reconnect.identity, clean.expectedIdentity);
  }
  assert.equal(audit(clean).passed, true);
});

runTest('detects wrong visible browser identity and duplicate reconnect launch', () => {
  const identityFixture = fixtureSet.sessions.find(
    (session) => session.expectedFindingCodes.includes('visible_identity_mismatch'),
  );
  const launchFixture = fixtureSet.sessions.find(
    (session) => session.expectedFindingCodes.includes('duplicate_cold_launch'),
  );
  assert.ok(identityFixture, 'fixture corpus omits wrong-browser identity');
  assert.ok(launchFixture, 'fixture corpus omits duplicate cold launch');
  assert.ok(audit(identityFixture).findings.some((finding) => finding.code === 'visible_identity_mismatch'));
  assert.ok(audit(launchFixture).findings.some((finding) => finding.code === 'duplicate_cold_launch'));
});

process.stdout.write('P158 external handoff oracle adversarial self-test passed\n');
