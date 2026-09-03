#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import Ajv2020 from 'ajv/dist/2020.js';

const recordSchema = JSON.parse(readFileSync('docs/dev/contracts/service-failure-record.v1.schema.json', 'utf8'));
const observationSchema = JSON.parse(readFileSync('docs/dev/contracts/service-failure-observation.v1.schema.json', 'utf8'));
const readbackSchema = JSON.parse(readFileSync('docs/dev/contracts/service-failure-journal-readback.v1.schema.json', 'utf8'));
const rust = readFileSync('cli/src/native/service_failure_journal.rs', 'utf8');
const controlPlane = readFileSync('cli/src/native/control_plane.rs', 'utf8');
const browser = readFileSync('cli/src/native/browser.rs', 'utf8');
const cdpLoop = readFileSync('cli/src/native/stream/cdp_loop.rs', 'utf8');
const dashboardServer = readFileSync('cli/src/native/stream/dashboard.rs', 'utf8');
const dashboard = readFileSync('packages/dashboard/src/lib/failure-observation.ts', 'utf8');
const viewport = readFileSync('packages/dashboard/src/components/workspace-remote-viewport.tsx', 'utf8');
const handoff = readFileSync('packages/dashboard/src/app/page.tsx', 'utf8');

const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
const validateRecord = ajv.compile(recordSchema);
const validateObservation = ajv.compile(observationSchema);
const validateReadback = ajv.compile(readbackSchema);

const record = {
  schemaVersion: 'agent-browser.service-failure-record.v1',
  occurrenceId: 'occurrence-1',
  occurredAt: '2026-09-03T12:00:00Z',
  bootEpoch: 'boot:test',
  runtimeEnvironment: 'development',
  category: 'cdp_stream',
  source: 'authenticated_dashboard_client',
  stage: 'frame_watchdog',
  code: 'cdp_frame_never_received',
  summary: 'Connected without a usable frame.',
  action: 'stream_frame',
  references: { browserId: 'browser-1', handoffIdHash: `sha256:${'a'.repeat(64)}` },
  details: { elapsedMs: 15000 },
};
assert(validateRecord(record), JSON.stringify(validateRecord.errors));
assert(validateReadback({
  schemaVersion: 'agent-browser.service-failure-journal-readback.v1',
  records: [record],
  malformedLineCount: 1,
  writeFailureCount: 0,
}), JSON.stringify(validateReadback.errors));

for (const category of ['guacamole_load', 'handoff_link', 'cdp_stream', 'dashboard_action']) {
  const observation = {
    category,
    stage: 'external_client',
    code: 'observed_failure',
    summary: 'The client observed a typed failure.',
    observationId: `observation-${category}`,
  };
  assert(validateObservation(observation), JSON.stringify(validateObservation.errors));
}

assert(!validateObservation({
  category: 'browser_launch', stage: 'client', code: 'bad', summary: 'bad', observationId: 'bad',
}), 'client observation schema accepted a server-only category');
assert(!validateObservation({
  category: 'handoff_link', stage: 'client', code: 'bad', summary: 'bad', observationId: 'bad',
  handoffUrl: 'https://private.example/remote-view/secret',
}), 'client observation schema accepted a raw URL field');

assert(rust.includes('failure-journal.jsonl'));
assert(rust.includes('file.sync_data()'));
assert(rust.includes('file.lock()'));
assert(rust.includes('file.lock_shared()'));
assert(rust.includes('malformed_line_count'));
assert(rust.includes('Permissions::from_mode(0o600)'));
assert(browser.includes('ServiceFailureCategory::BrowserLaunch'));
assert(cdpLoop.includes('cdp_screencast_start_failed'));
assert(cdpLoop.includes('cdp_frame_never_received'));
assert(cdpLoop.includes('cdp_frame_stream_stalled'));
assert(cdpLoop.includes('append_service_failure_best_effort'));
assert(controlPlane.includes('append_service_failure_best_effort'));
assert(controlPlane.includes('ServiceFailureCategory::DashboardAction'));
assert(dashboardServer.includes('"/api/service/failures"'));
assert(dashboardServer.includes('"/api/service/failure-observation"'));
assert(dashboardServer.includes('dashboard_auth::authenticate_headers'));
assert(dashboard.includes('installDashboardFetchFailureInstrumentation'));
assert(dashboard.includes('FAILURE_OBSERVATION_ROUTE'));
assert(viewport.includes('cdp_frame_never_received'));
assert(viewport.includes('cdp_frame_stream_stalled'));
assert(viewport.includes('guacamole_load'));
assert(handoff.includes('handoff_unusable'));

process.stdout.write('Service failure journal contract checks passed\n');
