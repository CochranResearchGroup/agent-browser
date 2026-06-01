#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const servicePanel = readFileSync('packages/dashboard/src/components/service-panel.tsx', 'utf8');
const dashboardPage = readFileSync('packages/dashboard/src/app/page.tsx', 'utf8');
const appShell = readFileSync('packages/dashboard/src/components/app-shell.tsx', 'utf8');
const serviceRoutePage = readFileSync('packages/dashboard/src/app/service/page.tsx', 'utf8');
const activityRoutePage = readFileSync('packages/dashboard/src/app/activity/page.tsx', 'utf8');
const browsersRoutePage = readFileSync('packages/dashboard/src/app/browsers/page.tsx', 'utf8');
const validationSelector = readFileSync('scripts/dev/select-validation.js', 'utf8');

assert.match(
  servicePanel,
  /export type ServiceInspectorSelection =[\s\S]*\| \{ kind: "incident"; incident: IncidentRecord \}[\s\S]*\| \{ kind: "job"; job: ServiceJob \};/,
  'ServiceInspectorSelection must keep incident and job selected-record state as data only',
);

assert.match(
  servicePanel,
  /export type ServiceInspectorActions = \{[\s\S]*onControlBrowser\?:[\s\S]*onControlTab\?:[\s\S]*onSelectBrowserId\?:[\s\S]*onSelectProfileId\?:[\s\S]*onSelectSessionId\?:[\s\S]*onSelectTabId\?:[\s\S]*onSelectJobId\?:[\s\S]*onAcknowledgeIncident\?:[\s\S]*onResolveIncident\?:[\s\S]*onShowIncidentTrace\?:[\s\S]*onCancelJob\?:/,
  'ServiceInspectorActions must expose control, related-record navigation, incident, and job callbacks separately',
);

assert.match(
  servicePanel,
  /export function ServiceDetailInspector\(\{[\s\S]*selection,[\s\S]*actions = \{\},/,
  'ServiceDetailInspector must accept actions separately from selected-record state',
);

assert.match(
  servicePanel,
  /selection\.kind === "browser"[\s\S]*<BrowserDetailContent[\s\S]*browser=\{selection\.browser\}[\s\S]*onControlBrowser=\{actions\.onControlBrowser\}[\s\S]*onSelectProfileId=\{actions\.onSelectProfileId\}[\s\S]*onSelectSessionId=\{actions\.onSelectSessionId\}/,
  'Browser inspector must receive remote-control and related-record callbacks from inspector actions',
);

assert.match(
  servicePanel,
  /selection\.kind === "incident"[\s\S]*<IncidentDetailContent[\s\S]*onAcknowledge=\{actions\.onAcknowledgeIncident\}[\s\S]*onResolve=\{actions\.onResolveIncident\}[\s\S]*onShowTrace=\{actions\.onShowIncidentTrace\}/,
  'Incident inspector must receive acknowledge, resolve, and trace callbacks from inspector actions',
);

assert.match(
  servicePanel,
  /selection\.kind === "job" && \([\s\S]*<JobDetailContent[\s\S]*job=\{selection\.job\}[\s\S]*onCancel=\{actions\.onCancelJob\}[\s\S]*onSelectBrowserId=\{actions\.onSelectBrowserId\}[\s\S]*onSelectProfileId=\{actions\.onSelectProfileId\}[\s\S]*onSelectSessionId=\{actions\.onSelectSessionId\}[\s\S]*onSelectTabId=\{actions\.onSelectTabId\}/,
  'Job inspector must receive cancel and related target callbacks from inspector actions',
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
  /const showJobsForDisplayAllocation = useCallback\(\(displayIsolation: string \| null, jobIds: string\[\] = \[\]\) => \{[\s\S]*selectWorkspaceTab\("jobs"\)[\s\S]*setJobDisplayFilter\(displayFilter\)[\s\S]*setJobSortKey\("displayIsolation"\)[\s\S]*setJobLimit\(\(current\) => \(current < 100 \? 100 : current\)\)/,
  'Service dashboard must switch to Jobs, update the route, and apply display-allocation filters from trace summary cards',
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
  /const showTraceJob = useCallback\(\(jobId: string\) => \{[\s\S]*selectWorkspaceTab\("jobs"\)[\s\S]*setJobQuery\(jobId\)[\s\S]*setJobDisplayFilter\("all"\)[\s\S]*setJobLimit\(\(current\) => \(current < 100 \? 100 : current\)\)/,
  'Service dashboard must switch to Jobs, update the route, and filter by job id from trace cards',
);

assert.match(
  servicePanel,
  /const showTraceIncident = useCallback\(\(incidentId: string\) => \{[\s\S]*selectWorkspaceTab\("incidents"\)[\s\S]*setIncidentQuery\(incidentId\)[\s\S]*setIncidentHandlingFilter\("all"\)[\s\S]*setIncidentLimit\(\(current\) => \(current < 100 \? 100 : current\)\)/,
  'Service dashboard must switch to Incidents, update the route, and filter by incident id from trace rows',
);

assert.match(
  servicePanel,
  /function incidentTraceFilters\(incident: IncidentRecord\): TraceFilters \{[\s\S]*serviceName: contextRecord\?\.serviceName \?\? ""[\s\S]*browserId: incident\.browserId \?\? contextRecord\?\.browserId \?\? ""[\s\S]*limit: 50/,
  'Service dashboard must derive trace filters from retained incident context',
);

assert.match(
  servicePanel,
  /const showIncidentTrace = useCallback\(\(incident: IncidentRecord\) => \{[\s\S]*const filters = incidentTraceFilters\(incident\)[\s\S]*selectWorkspaceTab\("events"\)[\s\S]*setTraceFilters\(filters\)[\s\S]*loadTraceForFilters\(filters\)/,
  'Incident detail actions must switch to Events, update the route, and load the related trace immediately',
);

assert.match(
  servicePanel,
  /function traceCliCommand\(filters: TraceFilters\): string \{[\s\S]*return traceHandoff\(filters\)\.cliCommand/,
  'Trace explorer must build its copyable CLI command from the shared trace handoff helper',
);

assert.match(
  servicePanel,
  /function traceHttpPath\(filters: TraceFilters\): string \{[\s\S]*return traceHandoff\(filters\)\.httpPath/,
  'Trace explorer must build its copyable HTTP trace path from the shared trace handoff helper',
);

assert.match(
  servicePanel,
  /function incidentHandoff\(filters: TraceFilters, trace: ServiceTraceData \| null\)[\s\S]*createServiceIncidentHandoff\(\{[\s\S]*incidentId: singleIncidentId[\s\S]*createServiceIncidentHandoff\(\{[\s\S]*state: "active"[\s\S]*handlingState: "unacknowledged"[\s\S]*summary: true/,
  'Trace explorer must build incident handoff references from the shared incident handoff helper',
);

assert.match(
  servicePanel,
  /className="service-trace-handoff"[\s\S]*aria-label="Trace handoff commands"[\s\S]*Trace CLI[\s\S]*copyTraceHandoff\("CLI trace command", cliCommand\)[\s\S]*Trace HTTP[\s\S]*copyTraceHandoff\("HTTP trace path", httpPath\)[\s\S]*Incidents CLI[\s\S]*copyTraceHandoff\("CLI incident command", incidentCliCommand\)[\s\S]*Incidents HTTP[\s\S]*copyTraceHandoff\("HTTP incident path", incidentHttpPath\)/,
  'Trace explorer must render trace and incident handoff copy affordances',
);

assert.match(
  servicePanel,
  /incidentActivityCommand && \([\s\S]*Activity CLI[\s\S]*copyTraceHandoff\("CLI incident activity command", incidentActivityCommand\)/,
  'Trace explorer must render incident activity handoff copy affordance when an incident id is known',
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
  /\{ label: "Display allocation", value: job\.displayIsolation \? displayIsolationLabel\(job\.displayIsolation\) : null \}/,
  'Job inspector must show requested display allocation policy',
);

assert.match(
  servicePanel,
  /onInspectorActionsChange\(\{[\s\S]*actingIncidentId,[\s\S]*onControlBrowser: focusBrowserViewStream,[\s\S]*onControlTab: inspectTabViewStream,[\s\S]*onSelectBrowserId: selectBrowserById,[\s\S]*onSelectProfileId: selectProfileById,[\s\S]*onSelectSessionId: selectSessionById,[\s\S]*onSelectTabId: selectTabById,[\s\S]*onSelectJobId: selectJobById,[\s\S]*onAcknowledgeIncident: acknowledgeInspectorIncident,[\s\S]*onResolveIncident: resolveInspectorIncident,[\s\S]*onShowIncidentTrace: showIncidentTrace,[\s\S]*onCancelJob: cancelInspectorJob,[\s\S]*\}\);/,
  'ServicePanel must publish right-pane control, navigation, incident, and job handlers through onInspectorActionsChange',
);

assert.match(
  servicePanel,
  /function InspectorHero\([\s\S]*function InspectorActionBar\([\s\S]*function InspectorSection\([\s\S]*function InspectorFactRows\([\s\S]*function InspectorEvidenceDisclosure\(/,
  'Service inspector must define shared hero, action, section, fact-row, and evidence primitives',
);

assert.match(
  servicePanel,
  /<InspectorEvidenceDisclosure[\s\S]*Raw browser record/,
  'Selected-record inspectors must move raw record payloads into evidence disclosures',
);
for (const label of ['Raw allocation', 'Raw session record', 'Raw tab record', 'Raw job record', 'Raw incident record']) {
  assert.match(servicePanel, new RegExp(label), `Inspector evidence must include ${label}`);
}

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
  /export type ServiceSession = \{[\s\S]*cleanup\?: string \| null;[\s\S]*profileLeaseDisposition\?: string \| null;[\s\S]*profileLeaseConflictSessionIds\?: string\[\];[\s\S]*lastLeaseObservedAt\?: string \| null;/,
  'Service sessions must expose human takeover lease, cleanup, conflict, and observation fields to the dashboard',
);

assert.match(
  servicePanel,
  /function isHumanTakeoverSession\(session: ServiceSession\): boolean \{[\s\S]*return \(session\.lease \?\? ""\)\.toLowerCase\(\) === "human_takeover";/,
  'Service dashboard must recognize the service-owned human takeover lease state',
);

assert.match(
  servicePanel,
  /function sessionStateLabel\(session: ServiceSession\): string \{[\s\S]*if \(isHumanTakeoverSession\(session\)\) return "human takeover";/,
  'Service session rows and inspectors must surface human takeover as the session state',
);

assert.match(
  servicePanel,
  /<InspectorSection title="Operator Takeover">[\s\S]*label: "Owner"[\s\S]*label: "Queue impact"[\s\S]*label: "Resume", value: "No service-owned resume action is exposed yet\."[\s\S]*label: "Selected browser"[\s\S]*label: "Selected tab"/,
  'Service session inspector must show operator takeover owner, queue impact, disabled resume reason, and selected target',
);

assert.match(
  servicePanel,
  /const inspectTabViewStream = useCallback\(async \(tab: ServiceTab\) => \{[\s\S]*const browser = tab\.browserId \? browserById\.get\(tab\.browserId\) : null;[\s\S]*const stream = browserPrimaryViewStream\(browser\);[\s\S]*if \(!canOpenControlViewStream\(stream\)\)[\s\S]*const tabIndex = tabIndexById\.get\(tab\.id\);[\s\S]*const targetId = tab\.targetId\?\.trim\(\);[\s\S]*const sessionName = daemonSessionNameForBrowser\(browser\);[\s\S]*action: "view_focus"[\s\S]*taskName: "inspect-hidden-rdp-tab"[\s\S]*params: targetId[\s\S]*sessionName[\s\S]*openViewStream\(stream, browser, tab, focusMessage\);/,
  'Service tab remote-control action must queue target-specific view_focus on the selected browser daemon before opening the stream',
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
  /retainedPruneSummary\(retainedPruneResult\)[\s\S]*Dry-run prune[\s\S]*retainedPruneTotal\(retainedPruneResult\) === 0[\s\S]*Apply prune/,
  'Retained-state cleanup controls must require a dry-run result before the guarded apply action is available',
);

assert.match(
  dashboardPage,
  /const SECTION_PATHS: Record<DashboardSection, string> = \{[\s\S]*service: "\/service"[\s\S]*activity: "\/activity"[\s\S]*\};/,
  'Dashboard page must define direct route paths for top-level sections',
);

assert.match(
  dashboardPage,
  /dashboardSectionFromPath\(window\.location\.pathname\)[\s\S]*window\.history\.pushState\(\{ dashboardSection: section \}, "", nextUrl\)[\s\S]*window\.addEventListener\("popstate", onPopState\)/,
  'Dashboard page must initialize, push, and restore active section state from browser history',
);

assert.match(
  appShell,
  /const NAV_PATHS: Record<DashboardSection, string> = \{[\s\S]*service: "\/service"[\s\S]*activity: "\/activity"[\s\S]*\};[\s\S]*<a[\s\S]*href=\{NAV_PATHS\[item\.id\]\}[\s\S]*aria-current=\{activeSection === item\.id \? "page" : undefined\}/,
  'Dashboard nav must expose real hrefs for URL-addressable sections',
);

assert.match(
  serviceRoutePage,
  /<DashboardPage initialSection="service" \/>/,
  'Service route page must deep-link directly to the Service dashboard section',
);

assert.match(
  activityRoutePage,
  /<DashboardPage initialSection="activity" \/>/,
  'Activity route page must deep-link directly to the Activity dashboard section',
);

assert.match(
  browsersRoutePage,
  /<DashboardPage initialSection="browsers" \/>/,
  'Browsers route page must deep-link directly to the Browsers dashboard section',
);

assert.match(
  servicePanel,
  /const \[workspaceTab, setWorkspaceTab\] = useState<ServiceWorkspaceTab>\(\(\) => \{[\s\S]*serviceWorkspaceFromSearch\(window\.location\.search\)[\s\S]*const selectWorkspaceTab = useCallback\(\(tab: ServiceWorkspaceTab\) => \{[\s\S]*pushServiceWorkspaceUrl\(tab\)/,
  'Service panel must persist the selected workspace tab in the URL query string',
);

assert.match(
  servicePanel,
  /serviceWorkspaceFromSearch\(search: string\): ServiceWorkspaceTab[\s\S]*params\.get\("view"\)[\s\S]*view\?\.startsWith\("service:"\)[\s\S]*legacyWorkspace = params\.get\("workspace"\)[\s\S]*: "browsers"/,
  'Service panel must default to the browser records workspace while treating workspace as a legacy service-tab key only',
);

assert.match(
  servicePanel,
  /function isInspectableServiceTab\(tab: ServiceTab\)[\s\S]*isActiveServiceTab\(tab\) \|\| !isBlankServiceTab\(tab\)/,
  'Service tabs workspace must distinguish inspectable tabs from closed blank placeholders',
);

assert.match(
  servicePanel,
  /const filteredTabRecords = useMemo[\s\S]*sessionTabQueryText[\s\S]*tabRecords\.filter\(\(tab\) => includesQuery\(tabSearchText\(tab\), sessionTabQueryText\)\)[\s\S]*tabRecords\.filter\(isInspectableServiceTab\)/,
  'Service tabs workspace must suppress placeholder tabs by default while allowing search to inspect retained raw records',
);

assert.match(
  servicePanel,
  /hiddenPlaceholderTabCount[\s\S]*placeholder tabs suppressed[\s\S]*Only closed blank placeholder tabs are retained\. Use search to inspect them\./,
  'Service tabs workspace must explain when placeholder tab records are hidden instead of presenting them as useful rows',
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
