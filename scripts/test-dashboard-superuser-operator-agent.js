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

const auth = read('cli/src/native/stream/dashboard_auth.rs');
const appIntelligence = read('cli/src/native/stream/app_intelligence.rs');
const dashboard = read('cli/src/native/stream/dashboard.rs');
const http = read('cli/src/native/stream/http.rs');
const api = read('packages/dashboard/src/lib/dashboard-api.ts');
const page = read('packages/dashboard/src/app/page.tsx');
const chatPanel = read('packages/dashboard/src/components/chat-panel.tsx');

assert(
  auth.includes('DASHBOARD_ROLE_SUPERUSER') &&
    auth.includes('DASHBOARD_ROLE_OBSERVER') &&
    auth.includes('require_superuser') &&
    auth.includes('Superuser role required'),
  'Dashboard auth must expose explicit superuser/observer roles and a superuser gate.',
);

assert(
  auth.includes('"admin",') &&
    auth.includes('DASHBOARD_ROLE_SUPERUSER') &&
    auth.includes('"codex",') &&
    auth.includes('DASHBOARD_ROLE_OBSERVER'),
  'Bootstrap role construction must keep admin superuser and codex observer distinct.',
);

for (const route of [
  '/api/app-intelligence/operator/status',
  '/api/app-intelligence/operator/turn',
  '/api/app-intelligence/operator/confirm',
]) {
  assert(appIntelligence.includes(route), `App Intelligence must define operator route ${route}`);
  assert(api.includes(route), `Dashboard API constants must expose operator route ${route}`);
}

assert(
  dashboard.includes('require_superuser') &&
    dashboard.includes('operator_status_json') &&
    dashboard.includes('operator_turn_response') &&
    dashboard.includes('operator_confirm_response'),
  'Dashboard server must gate operator status, turn, and confirm routes behind superuser auth.',
);

assert(
  http.includes('require_superuser') &&
    http.includes('operator_status_json') &&
    http.includes('operator_turn_response') &&
    http.includes('operator_confirm_response'),
  'HTTP dashboard runtime must gate operator status, turn, and confirm routes behind superuser auth.',
);

assert(
  appIntelligence.includes('superuser-operator') &&
    appIntelligence.includes('operator_tool_manifest') &&
    appIntelligence.includes('write_operator_ledger') &&
    appIntelligence.includes('operator_read_tool_calls') &&
    appIntelligence.includes('operator_dashboard_actions') &&
    appIntelligence.includes('read-tools-completed') &&
    appIntelligence.includes('Service-mediated operator contracts pending'),
  'Operator surface must expose audited read tools, dashboard actions, and a ledger before mutation tools are enabled.',
);

assert(
  page.includes('authenticatedUser={user}') &&
    chatPanel.includes('authenticatedUser?: DashboardAuthUser | null') &&
    chatPanel.includes('data-superuser-operator-agent="ready"') &&
    chatPanel.includes('chatMode === "operate"') &&
    chatPanel.includes('isSuperuser') &&
    chatPanel.includes('Plan operator action') &&
    chatPanel.includes('updateDashboardWorkspaceUrlSelection') &&
    chatPanel.includes('Tool calls') &&
    chatPanel.includes('Dashboard actions'),
  'Chat panel must expose Operate only from authenticated superuser context.',
);

assert(
  !chatPanel.includes('data-superuser-operator-agent="ready"\\n    >') ||
    chatPanel.includes('chatMode === "operate" && isSuperuser && authenticatedUser'),
  'Superuser operator marker must be guarded by superuser-only rendering.',
);

console.log('dashboard superuser operator agent tests passed');
