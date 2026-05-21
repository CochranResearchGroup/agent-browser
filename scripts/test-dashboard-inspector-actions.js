#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const servicePanel = readFileSync('packages/dashboard/src/components/service-panel.tsx', 'utf8');
const dashboardPage = readFileSync('packages/dashboard/src/app/page.tsx', 'utf8');
const validationSelector = readFileSync('scripts/dev/select-validation.js', 'utf8');

assert.match(
  servicePanel,
  /export type ServiceInspectorSelection =[\s\S]*\| \{ kind: "incident"; incident: IncidentRecord \}[\s\S]*\| \{ kind: "job"; job: ServiceJob \};/,
  'ServiceInspectorSelection must keep incident and job selected-record state as data only',
);

assert.match(
  servicePanel,
  /export type ServiceInspectorActions = \{[\s\S]*onControlBrowser\?:[\s\S]*onAcknowledgeIncident\?:[\s\S]*onResolveIncident\?:[\s\S]*onCancelJob\?:/,
  'ServiceInspectorActions must expose browser control, incident acknowledge, incident resolve, and job cancel callbacks separately',
);

assert.match(
  servicePanel,
  /export function ServiceDetailInspector\(\{[\s\S]*selection,[\s\S]*actions = \{\},/,
  'ServiceDetailInspector must accept actions separately from selected-record state',
);

assert.match(
  servicePanel,
  /selection\.kind === "browser"[\s\S]*<BrowserDetailContent browser=\{selection\.browser\} onControlBrowser=\{actions\.onControlBrowser\} \/>/,
  'Browser inspector must receive the remote-control callback from inspector actions',
);

assert.match(
  servicePanel,
  /selection\.kind === "incident"[\s\S]*<IncidentDetailContent[\s\S]*onAcknowledge=\{actions\.onAcknowledgeIncident\}[\s\S]*onResolve=\{actions\.onResolveIncident\}/,
  'Incident inspector must receive acknowledge and resolve callbacks from inspector actions',
);

assert.match(
  servicePanel,
  /selection\.kind === "job" && <JobDetailContent job=\{selection\.job\} onCancel=\{actions\.onCancelJob\} \/>/,
  'Job inspector must receive the cancel callback from inspector actions',
);

assert.match(
  servicePanel,
  /onInspectorActionsChange\(\{[\s\S]*actingIncidentId,[\s\S]*onControlBrowser: focusBrowserViewStream,[\s\S]*onAcknowledgeIncident: acknowledgeInspectorIncident,[\s\S]*onResolveIncident: resolveInspectorIncident,[\s\S]*onCancelJob: cancelInspectorJob,[\s\S]*\}\);/,
  'ServicePanel must publish right-pane action handlers through onInspectorActionsChange',
);

assert.match(
  servicePanel,
  /const controlAvailable = canOpenControlViewStream\(primaryViewStream\);[\s\S]*disabled=\{!controlAvailable\}[\s\S]*title=\{viewStreamControlTitle\(primaryViewStream\)\}[\s\S]*onClick=\{\(\) => onControlBrowser\(browser\)\}[\s\S]*Open remote control/,
  'Browser inspector remote-control action must use service stream control metadata for gating and disabled copy',
);

assert.match(
  servicePanel,
  /await handleIncident\(incident, "acknowledge", note, false\)/,
  'Right-pane acknowledge must keep the inspector open while applying the service action',
);

assert.match(
  servicePanel,
  /await handleIncident\(incident, "resolve", note, false\)/,
  'Right-pane resolve must keep the inspector open while applying the service action',
);

assert.match(
  servicePanel,
  /AlertDialogTrigger asChild[\s\S]*\{cleanupApplyLabel\}[\s\S]*AlertDialogContent[\s\S]*Apply cleanup/,
  'Retained-state cleanup apply must be guarded by an AlertDialog confirmation',
);

assert.match(
  dashboardPage,
  /const \[serviceInspectorActions, setServiceInspectorActions\] = useState<ServiceInspectorActions>\(\{\}\);/,
  'Dashboard page must hold inspector actions outside the selected-record state',
);

assert.match(
  dashboardPage,
  /<ServiceDetailInspector selection=\{serviceInspectorSelection\} actions=\{serviceInspectorActions\} \/>/,
  'Dashboard page must pass inspector actions into the right-pane inspector',
);

assert.match(
  dashboardPage,
  /onInspectorActionsChange=\{setServiceInspectorActions\}/,
  'Dashboard page must receive action handlers from the active ServicePanel',
);

assert.match(
  validationSelector,
  /pnpm test:dashboard-inspector-actions/,
  'Validation selector must recommend the dashboard inspector action smoke for Service dashboard wiring changes',
);

console.log('Dashboard inspector action contract smoke passed');
