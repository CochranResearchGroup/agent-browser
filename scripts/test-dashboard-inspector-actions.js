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
  /export type ServiceInspectorActions = \{[\s\S]*onControlBrowser\?:[\s\S]*onAcknowledgeIncident\?:[\s\S]*onResolveIncident\?:[\s\S]*onShowIncidentTrace\?:[\s\S]*onCancelJob\?:/,
  'ServiceInspectorActions must expose browser control, incident acknowledge, incident resolve, incident trace, and job cancel callbacks separately',
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
  /selection\.kind === "incident"[\s\S]*<IncidentDetailContent[\s\S]*onAcknowledge=\{actions\.onAcknowledgeIncident\}[\s\S]*onResolve=\{actions\.onResolveIncident\}[\s\S]*onShowTrace=\{actions\.onShowIncidentTrace\}/,
  'Incident inspector must receive acknowledge, resolve, and trace callbacks from inspector actions',
);

assert.match(
  servicePanel,
  /selection\.kind === "job" && <JobDetailContent job=\{selection\.job\} onCancel=\{actions\.onCancelJob\} \/>/,
  'Job inspector must receive the cancel callback from inspector actions',
);

assert.match(
  servicePanel,
  /export type ServiceJob = \{[\s\S]*displayIsolation\?: string \| null;/,
  'Service jobs must expose displayIsolation for operator request-intent visibility',
);

assert.match(
  servicePanel,
  /job\.displayIsolation && \([\s\S]*title=\{displayIsolationValueTitle\(job\.displayIsolation\)\}[\s\S]*\{displayIsolationLabel\(job\.displayIsolation\)\}/,
  'Job rows must show the requested display allocation policy when present',
);

assert.match(
  servicePanel,
  /type ServiceJobDisplayFilter = "all" \| "private_virtual_display" \| "shared_display" \| "ambient_display" \| "unrecorded";/,
  'Service dashboard must expose a typed job display allocation filter',
);

assert.match(
  servicePanel,
  /const filteredJobs = useMemo\(\(\) => \{[\s\S]*serviceJobDisplayMatchesFilter\(job, jobDisplayFilter\)[\s\S]*serviceJobSortValue\(left, jobSortKey\)[\s\S]*jobSortDirection === "asc"/,
  'Service dashboard Jobs workspace must filter and sort jobs by recorded display allocation intent',
);

assert.match(
  servicePanel,
  /<select[\s\S]*value=\{jobDisplayFilter\}[\s\S]*setJobDisplayFilter\(event\.target\.value as ServiceJobDisplayFilter\)[\s\S]*SERVICE_JOB_DISPLAY_FILTER_OPTIONS\.map/,
  'Service dashboard Jobs workspace must render a display allocation filter control',
);

assert.match(
  servicePanel,
  /onShowJobsForDisplayAllocation: \(displayIsolation: string \| null, jobIds\?: string\[\]\) => void;/,
  'Trace explorer must expose a typed action for jumping from display allocation summaries to Jobs',
);

assert.match(
  servicePanel,
  /onClick=\{\(\) =>[\s\S]*onShowJobsForDisplayAllocation\(allocation\.displayIsolation \?\? null, allocation\.jobIds\)[\s\S]*show in Jobs/,
  'Trace display allocation cards must jump to the filtered Jobs workspace',
);

assert.match(
  servicePanel,
  /const showJobsForDisplayAllocation = useCallback\(\(displayIsolation: string \| null, jobIds: string\[\] = \[\]\) => \{[\s\S]*setWorkspaceTab\("jobs"\)[\s\S]*setJobDisplayFilter\(displayFilter\)[\s\S]*setJobSortKey\("displayIsolation"\)[\s\S]*setJobLimit\(\(current\) => \(current < 100 \? 100 : current\)\)/,
  'Service dashboard must switch to Jobs and apply display-allocation filters from trace summary cards',
);

assert.match(
  servicePanel,
  /onShowTraceJob: \(jobId: string\) => void;/,
  'Trace explorer must expose a typed action for jumping trace job references to Jobs',
);

assert.match(
  servicePanel,
  /onShowTraceIncident: \(incidentId: string\) => void;/,
  'Trace explorer must expose a typed action for jumping trace incident references to Incidents',
);

assert.match(
  servicePanel,
  /onClick=\{\(\) => onShowTraceJob\(wait\.jobId\)\}[\s\S]*Show job in Jobs/,
  'Trace profile lease wait cards must jump to the retained job in the Jobs workspace',
);

assert.match(
  servicePanel,
  /const showTraceJob = useCallback\(\(jobId: string\) => \{[\s\S]*setWorkspaceTab\("jobs"\)[\s\S]*setJobQuery\(jobId\)[\s\S]*setJobDisplayFilter\("all"\)[\s\S]*setJobLimit\(\(current\) => \(current < 100 \? 100 : current\)\)/,
  'Service dashboard must switch to Jobs and filter by job id from trace cards',
);

assert.match(
  servicePanel,
  /const showTraceIncident = useCallback\(\(incidentId: string\) => \{[\s\S]*setWorkspaceTab\("incidents"\)[\s\S]*setIncidentQuery\(incidentId\)[\s\S]*setIncidentHandlingFilter\("all"\)[\s\S]*setIncidentLimit\(\(current\) => \(current < 100 \? 100 : current\)\)/,
  'Service dashboard must switch to Incidents and filter by incident id from trace rows',
);

assert.match(
  servicePanel,
  /function incidentTraceFilters\(incident: IncidentRecord\): TraceFilters \{[\s\S]*serviceName: contextRecord\?\.serviceName \?\? ""[\s\S]*browserId: incident\.browserId \?\? contextRecord\?\.browserId \?\? ""[\s\S]*limit: 50/,
  'Service dashboard must derive trace filters from retained incident context',
);

assert.match(
  servicePanel,
  /const showIncidentTrace = useCallback\(\(incident: IncidentRecord\) => \{[\s\S]*const filters = incidentTraceFilters\(incident\)[\s\S]*setWorkspaceTab\("events"\)[\s\S]*setTraceFilters\(filters\)[\s\S]*loadTraceForFilters\(filters\)/,
  'Incident detail actions must switch to Events and load the related trace immediately',
);

assert.match(
  servicePanel,
  /function traceCliCommand\(filters: TraceFilters\): string \{[\s\S]*"agent-browser", "service", "trace"[\s\S]*"--service-name"[\s\S]*"--limit", String\(filters\.limit\)/,
  'Trace explorer must build a copyable CLI command from active filters',
);

assert.match(
  servicePanel,
  /function traceHttpPath\(filters: TraceFilters\): string \{[\s\S]*`\/api\/service\/trace\?\$\{traceQueryParams\(filters\)\.toString\(\)\}`/,
  'Trace explorer must build a copyable HTTP trace path from active filters',
);

assert.match(
  servicePanel,
  /className="service-trace-handoff"[\s\S]*aria-label="Trace handoff commands"[\s\S]*\{cliCommand\}[\s\S]*copyTraceHandoff\("CLI trace command", cliCommand\)[\s\S]*\{httpPath\}[\s\S]*copyTraceHandoff\("HTTP trace path", httpPath\)/,
  'Trace explorer must render CLI and HTTP handoff copy affordances',
);

assert.match(
  servicePanel,
  /className="service-trace-timeline-job-link"[\s\S]*onClick=\{\(\) => onShowTraceJob\(jobId\)\}[\s\S]*Show job \{jobId\} in Jobs/,
  'Trace timeline job rows must jump to the retained job in the Jobs workspace',
);

assert.match(
  servicePanel,
  /className="service-trace-timeline-incident-link"[\s\S]*onClick=\{\(\) => onShowTraceIncident\(incidentId\)\}[\s\S]*Show incident \{incidentId\} in Incidents/,
  'Trace timeline incident rows must jump to the retained incident in the Incidents workspace',
);

assert.match(
  servicePanel,
  /<JobSortButton[\s\S]*sortKey=\{sortKey\}[\s\S]*activeSortKey=\{jobSortKey\}[\s\S]*onSort=\{toggleJobSort\}/,
  'Service dashboard Jobs workspace must render job sort controls',
);

assert.match(
  servicePanel,
  /<EventDetailItem label="Display allocation" value=\{job\.displayIsolation \? displayIsolationLabel\(job\.displayIsolation\) : null\} \/>/,
  'Job inspector must show requested display allocation policy',
);

assert.match(
  servicePanel,
  /onInspectorActionsChange\(\{[\s\S]*actingIncidentId,[\s\S]*onControlBrowser: focusBrowserViewStream,[\s\S]*onAcknowledgeIncident: acknowledgeInspectorIncident,[\s\S]*onResolveIncident: resolveInspectorIncident,[\s\S]*onShowIncidentTrace: showIncidentTrace,[\s\S]*onCancelJob: cancelInspectorJob,[\s\S]*\}\);/,
  'ServicePanel must publish right-pane action handlers through onInspectorActionsChange',
);

assert.match(
  servicePanel,
  /const controlAvailable = canOpenControlViewStream\(primaryViewStream\);[\s\S]*disabled=\{!controlAvailable\}[\s\S]*title=\{viewStreamControlTitle\(primaryViewStream\)\}[\s\S]*onClick=\{\(\) => onControlBrowser\(browser\)\}[\s\S]*Open remote control/,
  'Browser inspector remote-control action must use service stream control metadata for gating and disabled copy',
);

assert.match(
  servicePanel,
  /function ServiceTabRow\(\{[\s\S]*viewStreamAvailable,[\s\S]*onInspect,[\s\S]*onSelect,[\s\S]*\}: \{[\s\S]*viewStreamAvailable\?: boolean;[\s\S]*onInspect\?: \(tab: ServiceTab\) => void;[\s\S]*onSelect: \(tab: ServiceTab\) => void;[\s\S]*\}\) \{[\s\S]*aria-label=\{`Inspect tab \$\{tab\.id\}`\}[\s\S]*\{viewStreamAvailable && onInspect && \([\s\S]*onClick=\{\(\) => onInspect\(tab\)\}[\s\S]*Control/,
  'Service tab rows must keep a gated Control action wired to the tab inspect callback',
);

assert.match(
  servicePanel,
  /const inspectTabViewStream = useCallback\(async \(tab: ServiceTab\) => \{[\s\S]*const browser = tab\.browserId \? browserById\.get\(tab\.browserId\) : null;[\s\S]*const stream = browserPrimaryViewStream\(browser\);[\s\S]*if \(!canOpenControlViewStream\(stream\)\)[\s\S]*const tabIndex = tabIndexById\.get\(tab\.id\);[\s\S]*action: "view_focus"[\s\S]*taskName: "inspect-hidden-rdp-tab"[\s\S]*params: \{ index: tabIndex, maximize: true \}[\s\S]*openViewStream\(stream, browser, tab, focusMessage\);/,
  'Service tab remote-control action must queue tab-specific view_focus before opening the stream',
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
