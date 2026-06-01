#!/usr/bin/env node

import fs from 'node:fs';

function read(path) {
  return fs.readFileSync(path, 'utf8');
}

function assert(condition, message) {
  if (!condition) {
    console.error(message);
    process.exit(1);
  }
}

const chatPanel = read('packages/dashboard/src/components/chat-panel.tsx');
const dashboardApi = read('packages/dashboard/src/lib/dashboard-api.ts');
const packet = read('packages/dashboard/src/lib/selected-workspace-chat-packet.ts');
const appIntelligence = read('cli/src/native/stream/app_intelligence.rs');
const appIntelligenceSupervisor = read('cli/src/native/stream/app_intelligence_supervisor.rs');
const appIntelligenceSchema = read('cli/src/native/stream/app_intelligence_schema.rs');
const http = read('cli/src/native/stream/http.rs');
const dashboard = read('cli/src/native/stream/dashboard.rs');

assert(
  packet.includes('CONTEXTUAL_CHAT_PROVIDER_ID = "codex-app-server"'),
  'selected workspace packet must hard-code the Codex app server provider',
);
assert(
  dashboardApi.includes('APP_INTELLIGENCE_INSPECT_API_URL = "/api/app-intelligence/inspect-workspace"'),
  'dashboard API must expose the app intelligence inspect endpoint',
);
assert(
  chatPanel.includes('buildSelectedWorkspaceChatPacket') &&
    chatPanel.includes('validateSelectedWorkspaceChatPacket') &&
    chatPanel.includes('APP_INTELLIGENCE_INSPECT_API_URL'),
  'Chat panel must build and submit selected-workspace packets to app intelligence',
);
assert(
  chatPanel.includes('data-codex-app-server-contextual-chat="ready"') &&
    chatPanel.includes('data-contextual-chat-provider={CONTEXTUAL_CHAT_PROVIDER_ID}'),
  'Chat panel must expose stable Codex app-server UX markers',
);
assert(
  chatPanel.includes('Codex app server') && chatPanel.includes('read-only'),
  'Chat panel must identify the Codex app server read-only provider',
);
assert(!chatPanel.includes('<ModelSelector'), 'Contextual Chat must not render the model selector');
assert(!chatPanel.includes('MODELS_API_URL'), 'Contextual Chat must not use the models endpoint');
assert(!chatPanel.includes('ImagePlus'), 'Contextual Chat must not expose screenshot/image attachment input');
assert(
  appIntelligence.includes('APP_INTELLIGENCE_INSPECT_HTTP_ROUTE') &&
    appIntelligence.includes('"/api/app-intelligence/inspect-workspace"'),
  'Rust app intelligence module must define the inspect route',
);
assert(
  appIntelligence.includes('Contextual Chat currently exposes only the Codex app server provider') &&
    appIntelligence.includes('reject_mutating_request'),
  'Rust app intelligence endpoint must reject alternate providers and mutating requests',
);
assert(
  appIntelligence.includes('inspect_with_supervisor') &&
    appIntelligence.includes('dashboard_ledger'),
  'Rust app intelligence endpoint must delegate to the supervised app-server adapter and expose ledger metadata',
);
assert(
  appIntelligenceSupervisor.includes('codex') &&
    appIntelligenceSupervisor.includes('app-server') &&
    appIntelligenceSupervisor.includes('stdio://') &&
    appIntelligenceSupervisor.includes('thread/start') &&
    appIntelligenceSupervisor.includes('turn/start') &&
    appIntelligenceSupervisor.includes('outputSchema') &&
    appIntelligenceSupervisor.includes('codex-events.jsonl') &&
    appIntelligenceSupervisor.includes('events.jsonl') &&
    appIntelligenceSupervisor.includes('observation.json'),
  'Supervisor must use codex app-server stdio and persist replayable run artifacts',
);
assert(
  appIntelligenceSchema.includes('validate_observation') &&
    appIntelligenceSchema.includes('observation_output_schema') &&
    appIntelligenceSchema.includes('codex-workspace-observation.v1'),
  'Rust schema module must validate Codex workspace observations before returning them',
);
assert(
  chatPanel.includes('Inspection failure') &&
    chatPanel.includes('event log') &&
    chatPanel.includes('thread {ledger.threadId.slice(0, 8)}') &&
    chatPanel.includes('turn {ledger.turnId.slice(0, 8)}'),
  'Chat panel must render structured app-server failures and ledger metadata',
);
assert(
  http.includes('APP_INTELLIGENCE_INSPECT_HTTP_ROUTE') &&
    dashboard.includes('APP_INTELLIGENCE_INSPECT_HTTP_ROUTE'),
  'Both HTTP surfaces must route app intelligence inspection requests',
);

console.log('dashboard contextual chat tests passed');
