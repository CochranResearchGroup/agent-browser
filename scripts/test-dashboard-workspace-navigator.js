#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const page = readFileSync('packages/dashboard/src/app/page.tsx', 'utf8');
const navigator = readFileSync('packages/dashboard/src/components/workspace-navigator.tsx', 'utf8');
const servicePanel = readFileSync('packages/dashboard/src/components/service-panel.tsx', 'utf8');
const sessionsStore = readFileSync('packages/dashboard/src/store/sessions.ts', 'utf8');
const chatStore = readFileSync('packages/dashboard/src/store/chat.ts', 'utf8');
const remoteViewport = readFileSync('packages/dashboard/src/components/workspace-remote-viewport.tsx', 'utf8');
const dashboardApi = readFileSync('packages/dashboard/src/lib/dashboard-api.ts', 'utf8');
const serviceWorkspaces = readFileSync('packages/dashboard/src/lib/service-workspaces.ts', 'utf8');
const workspaceUrl = readFileSync('packages/dashboard/src/lib/workspace-url-selection.ts', 'utf8');
const css = readFileSync('packages/dashboard/src/app/globals.css', 'utf8');
const selector = readFileSync('scripts/dev/select-validation.js', 'utf8');
const readme = readFileSync('README.md', 'utf8');
const dashboardDocs = readFileSync('docs/src/app/dashboard/page.mdx', 'utf8');
const serviceModeDocs = readFileSync('docs/src/app/service-mode/page.mdx', 'utf8');
const commandsDocs = readFileSync('docs/src/app/commands/page.mdx', 'utf8');
const skill = readFileSync('skills/agent-browser/SKILL.md', 'utf8');
const cliOutput = readFileSync('cli/src/output.rs', 'utf8');

assert.match(
  page,
  /import \{ WorkspaceNavigator \} from "@\/components\/workspace-navigator";/,
  'Dashboard page must render the workspace navigator instead of importing SessionTree',
);

assert.doesNotMatch(
  page,
  /<SessionTree \/>/,
  'Dashboard page must not render the raw session tree after the workspace navigator refactor',
);

assert.match(
  page,
  /type MobileDashboardPanel = "workspaces" \| "viewport" \| "activity" \| "service";[\s\S]*const \[mobilePanel, setMobilePanel\] = useState<MobileDashboardPanel>[\s\S]*value=\{mobilePanel\}[\s\S]*value="workspaces"[\s\S]*<WorkspaceNavigator \/>/,
  'Mobile dashboard tabs must be able to render the workspace navigator instead of falling through to the viewport',
);

assert.match(
  page,
  /<ResizablePanel id="sessions" defaultSize="20%" minSize="14%" maxSize="34%">/,
  'Desktop workspace pane must default wide enough for human labels, not only raw IDs',
);

assert.match(
  navigator,
  /deriveWorkspaceNodes\(\{?workspaceInput\}?\)/,
  'Workspace navigator must derive rows from the shared WorkspaceNode model',
);

assert.match(
  navigator,
  /startTransition\(\(\) => \{[\s\S]*setServiceStatus[\s\S]*setServiceContracts[\s\S]*setBrowserCapabilityRegistry/,
  'Workspace navigator must schedule service-state refreshes as non-urgent updates so scrolling and typing stay responsive',
);

assert.match(
  navigator,
  /const deferredQuery = useDeferredValue\(query\)[\s\S]*const text = deferredQuery\.trim\(\)\.toLowerCase\(\)/,
  'Workspace navigator must defer query filtering so large workspace lists do not block input',
);

assert.match(
  navigator,
  /fetch\(`\$\{serviceBase\(activePort\)\}\/status`\)/,
  'Workspace navigator must read service-owned state from the service status endpoint',
);

assert.match(
  dashboardApi,
  /export const SERVICE_API_BASE = "\/api\/service";[\s\S]*sessionTabsApiUrl\(port: number\)[\s\S]*\/api\/session-tabs\?port=/,
  'Dashboard browser clients must use same-origin API URLs so public dashboard fetches do not target the viewer localhost',
);

assert.match(
  sessionsStore,
  /fetch\(sessionTabsApiUrl\(s\.port\)\)/,
  'Workspace tab polling must use the same-origin dashboard session-tabs proxy',
);

for (const [label, text] of [
  ['workspace navigator', navigator],
  ['workspace remote viewport', remoteViewport],
  ['service panel', servicePanel],
  ['chat store', chatStore],
]) {
  assert.doesNotMatch(
    text,
    /NEXT_PUBLIC_DAEMON_URL|const DAEMON_URL|http:\/\/localhost:\$\{/,
    `${label} must not build browser-side API fetches from localhost or NEXT_PUBLIC_DAEMON_URL`,
  );
}

assert.match(
  workspaceUrl,
  /DASHBOARD_WORKSPACE_QUERY_KEYS = \[[\s\S]*"workspace"[\s\S]*"browser"[\s\S]*"session"[\s\S]*"tab"[\s\S]*"profile"[\s\S]*"job"[\s\S]*\] as const;/,
  'Workspace URL helper must define durable query keys plus the internal job key for related-record jumps',
);

assert.match(
  serviceWorkspaces,
  /function primaryServiceTab\(tabs: WorkspaceServiceTab\[\]\)[\s\S]*serviceWorkspaceTabScore[\s\S]*targetId: tab\.targetId/,
  'Workspace nodes must prefer live non-blank service tabs and retain target IDs for remote-control routing',
);

assert.match(
  workspaceUrl,
  /function readDashboardWorkspaceUrlSelection[\s\S]*params\.get\("workspace"\)[\s\S]*params\.get\("browser"\)[\s\S]*params\.get\("session"\)[\s\S]*params\.get\("tab"\)[\s\S]*params\.get\("profile"\)[\s\S]*params\.get\("job"\)/,
  'Workspace navigator must restore workspace selection identity from URL query params',
);

assert.match(
  workspaceUrl,
  /function writeDashboardWorkspaceUrlSelection[\s\S]*params\.set\(key, value\)[\s\S]*window\.history\.pushState\(state, "", nextUrl\)[\s\S]*dispatchDashboardWorkspaceSelectionChange\(selection\)/,
  'Workspace navigator must write selected workspace identity through browser history',
);

assert.match(
  navigator,
  /window\.addEventListener\("popstate", onPopState\)[\s\S]*window\.addEventListener\(DASHBOARD_WORKSPACE_SELECTION_EVENT, onUrlSelectionChange\)/,
  'Workspace navigator must listen for back-forward navigation and service-panel related-record selection',
);

assert.match(
  navigator,
  /workspaceUrlSelectionScore[\s\S]*selection\.workspaceId[\s\S]*selection\.browserId[\s\S]*selection\.sessionId[\s\S]*selection\.tabId[\s\S]*selection\.profileId[\s\S]*selection\.jobId/,
  'Workspace navigator must match URL selection against workspace, browser, session, tab, profile, and job identities',
);

assert.match(
  navigator,
  /selectNode\(bestNode, \{[\s\S]*focusDaemon: false,[\s\S]*persistUrl: false,[\s\S]*\}\);[\s\S]*updateDashboardWorkspaceUrlSelection\(\{ workspaceId: bestNode\.id \}, "replace"\)/,
  'Workspace navigator must fill only the derived workspace id when restoring URL selection so related-record jumps do not regain stale browser or session params',
);

assert.match(
  navigator,
  /lastScrolledSelectionRef[\s\S]*document\.querySelector\("\.workspace-nav-row-selected"\)[\s\S]*scrollIntoView\(\{ block: "center", inline: "nearest" \}\)/,
  'Workspace navigator must scroll restored selections into view so refresh persistence is visible',
);

assert.doesNotMatch(
  page,
  /params\.delete\("workspace"\)/,
  'Dashboard route changes must preserve workspace selection params instead of resetting to the home view',
);

assert.match(
  servicePanel,
  /url\.searchParams\.set\("view", `service:\$\{workspace\}`\);/,
  'Service panel tabs must use view=service:<tab> instead of overwriting the workspace selection param',
);

assert.match(
  servicePanel,
  /window\.history\.pushState\(\{ dashboardSection: "service", serviceView: workspace \}, "", nextUrl\);[\s\S]*const dispatchSelectionChange[\s\S]*new CustomEvent\(DASHBOARD_WORKSPACE_SELECTION_EVENT, \{ detail: readDashboardWorkspaceUrlSelection\(\) \}\)[\s\S]*dispatchSelectionChange\(\);[\s\S]*window\.setTimeout\(dispatchSelectionChange, 0\)/,
  'Service panel view changes must notify the workspace navigator after preserving the current record identity in the URL and after Next router state settles',
);

assert.match(
  servicePanel,
  /updateDashboardWorkspaceUrlSelection\(\{[\s\S]*workspaceId: null[\s\S]*browserId: null[\s\S]*sessionId: null[\s\S]*tabId: null[\s\S]*profileId: null[\s\S]*jobId: null[\s\S]*\.\.\.selection[\s\S]*\}, mode\)/,
  'Service panel related-record clicks must clear stale workspace identity before selecting a browser, profile, session, tab, or job',
);

assert.match(
  servicePanel,
  /window\.addEventListener\(DASHBOARD_WORKSPACE_SELECTION_EVENT, onWorkspaceSelectionChange\)/,
  'Service panel must observe workspace selection URL changes from the navigator',
);

assert.match(
  servicePanel,
  /workspaceUrlSelection\.browserId[\s\S]*inspectBrowser\(browser, \{ historyMode: "replace", syncWorkspace: false \}\)[\s\S]*workspaceUrlSelection\.profileId[\s\S]*inspectProfileAllocation\(allocation, \{ historyMode: "replace", syncWorkspace: false \}\)[\s\S]*workspaceUrlSelection\.sessionId[\s\S]*inspectSession\(session, \{ historyMode: "replace", syncWorkspace: false \}\)[\s\S]*workspaceUrlSelection\.tabId[\s\S]*inspectTab\(tab, \{ historyMode: "replace", syncWorkspace: false \}\)[\s\S]*workspaceUrlSelection\.jobId[\s\S]*inspectJob\(job, \{ historyMode: "replace", syncWorkspace: false \}\)/,
  'Service panel must synchronize browser, profile, session, tab, and job URL selection into the center view and inspector',
);

assert.match(
  servicePanel,
  /const selectProfileById[\s\S]*if \(allocation\) void inspectProfileAllocation\(allocation\);[\s\S]*selectWorkspaceTab\("profiles"\);[\s\S]*const selectSessionById[\s\S]*if \(session\) inspectSession\(session\);[\s\S]*selectWorkspaceTab\("sessions"\);[\s\S]*const selectTabById[\s\S]*if \(tab\) inspectTab\(tab\);[\s\S]*selectWorkspaceTab\("tabs"\);[\s\S]*const selectJobById[\s\S]*if \(job\) void inspectJob\(job\);[\s\S]*selectWorkspaceTab\("jobs"\);/,
  'Service inspector related-record actions must write the selected record identity before pushing the service view, avoiding stale Next router query replay',
);

assert.match(
  navigator,
  /serviceBrowsers: Object\.values\(serviceState\?\.browsers \?\? \{\}\)[\s\S]*serviceSessions: Object\.values\(serviceState\?\.sessions \?\? \{\}\)[\s\S]*serviceTabs: Object\.values\(serviceState\?\.tabs \?\? \{\}\)[\s\S]*profileAllocations: serviceStatus\?\.profileAllocations \?\? \[\][\s\S]*jobs: Object\.values\(serviceState\?\.jobs \?\? \{\}\)[\s\S]*incidents: serviceState\?\.incidents \?\? \[\]/,
  'Workspace navigator must feed browser, session, tab, profile, job, and incident state into WorkspaceNode derivation',
);

assert.match(
  navigator,
  /SCOPE_LABELS[\s\S]*active: "Active"[\s\S]*"needs-attention": "Attention"[\s\S]*retained: "Retained"/,
  'Workspace navigator must expose Active, Attention, and Retained scopes',
);

assert.match(
  navigator,
  /<WorkspaceGroup[\s\S]*title="Active"[\s\S]*nodes=\{grouped\.active\}[\s\S]*<WorkspaceGroup[\s\S]*title="Needs attention"[\s\S]*nodes=\{grouped\["needs-attention"\]\}[\s\S]*defaultOpen=\{attentionDefaultOpen\}[\s\S]*onDismiss=\{dismissAttentionNode\}[\s\S]*<WorkspaceGroup[\s\S]*title="Retained"[\s\S]*nodes=\{grouped\.retained\}/,
  'Workspace navigator must render active work first, keep attention collapsible, and expose dismissal for attention rows',
);

assert.match(
  navigator,
  /DISMISSED_ATTENTION_STORAGE_KEY[\s\S]*readDismissedAttentionIds[\s\S]*writeDismissedAttentionIds[\s\S]*dismissedAttentionIds[\s\S]*visibleNodes[\s\S]*dismissAttentionNode[\s\S]*restoreDismissedAttention/,
  'Workspace navigator must persist local dismissal state for noisy attention rows',
);

assert.match(
  navigator,
  /const WORKSPACE_RETAINED_ROW_WINDOW = 80;[\s\S]*const retainedDefaultOpen = scope === "retained" \|\| Boolean\(query\.trim\(\)\);[\s\S]*defaultOpen=\{retainedDefaultOpen\}[\s\S]*rowWindow=\{WORKSPACE_RETAINED_ROW_WINDOW\}/,
  'Workspace navigator must keep large retained history collapsed by default and windowed when opened',
);

assert.match(
  navigator,
  /const WORKSPACE_ACTIVE_ROW_WINDOW = 64;[\s\S]*const WORKSPACE_ATTENTION_ROW_WINDOW = 48;[\s\S]*rowWindow=\{WORKSPACE_ACTIVE_ROW_WINDOW\}[\s\S]*rowWindow=\{WORKSPACE_ATTENTION_ROW_WINDOW\}/,
  'Workspace navigator must window large active and attention groups so mobile workspaces stay responsive',
);

assert.match(
  navigator,
  /EMPTY_LAUNCHER_PREVIEW[\s\S]*const launcherPreview = useMemo\(\(\) => \{[\s\S]*if \(!newSessionOpen\) return EMPTY_LAUNCHER_PREVIEW;[\s\S]*deriveLauncherEligibilityPreview/,
  'Workspace navigator must not derive the launcher browser/profile matrix until the launcher dialog is open',
);

assert.match(
  navigator,
  /dispatchCreateSession[\s\S]*dispatchCloseSession[\s\S]*dispatchKillSession[\s\S]*dispatchCloseAllSessions[\s\S]*dispatchAddTab[\s\S]*dispatchSwitchTab/,
  'Workspace navigator must preserve existing daemon session create, close, kill, close-all, add-tab, and switch-tab actions',
);

assert.match(
  navigator,
  /const operatorControlIds: WorkspaceNodeActionId\[\] = \["control", "view"\][\s\S]*for \(const id of operatorControlIds\)[\s\S]*candidate\.id === id && candidate\.enabled[\s\S]*const preferredIds: WorkspaceNodeActionId\[\] = \["focus", "launch", "seed"\]/,
  'Workspace navigator primary actions must prefer service-owned Control and View before daemon Focus, Launch, or Seed',
);

assert.match(
  navigator,
  /function pushWorkspaceViewportSelectionUrl[\s\S]*url\.searchParams\.set\("view", `workspace:\$\{mode\}`\)[\s\S]*DASHBOARD_WORKSPACE_QUERY_KEYS/,
  'Workspace navigator must be able to route launch responses directly into the embedded workspace viewport',
);

assert.match(
  navigator,
  /function extractServiceRequestWorkspaceIdentity[\s\S]*browserId[\s\S]*sessionId[\s\S]*tabId[\s\S]*profileId/,
  'Workspace launch submission must extract browser, session, tab, and profile identity from service responses when available',
);

assert.match(
  navigator,
  /createLauncherSessionArgsFromAccessPlan\(accessPlan,[\s\S]*sessionName[\s\S]*executableId: row\.executableId[\s\S]*browserHostId: row\.browserHostId[\s\S]*execCommand\(args\)/,
  'Workspace launcher must start the selected browser/profile combo as its own daemon session instead of sending the request to an existing backend browser',
);

assert.match(
  navigator,
  /const launchedBrowserId = `session:\$\{sessionName\}`[\s\S]*freshStatus\?\.service_state\?\.browsers\?\.\[launchedBrowserId\][\s\S]*activeSessionIds\?\.includes\(sessionName\)[\s\S]*launchViewStream\(browser\)[\s\S]*setNewSessionOpen\(false\)[\s\S]*pushWorkspaceViewportSelectionUrl[\s\S]*pushServiceJobsView\(identity\.jobId\)/,
  'Workspace launcher must close after submit and prefer opening the launched session Guacamole viewport before falling back to Jobs',
);

assert.match(
  navigator,
  /useState<LauncherDisplayIsolation>\("shared_display"\)[\s\S]*useState<LauncherViewStreamPreference>\("rdp_gateway"\)[\s\S]*useState<LauncherControlInputPreference>\("manual_attached_desktop"\)/,
  'Workspace launcher defaults must request the shared remote desktop with RDP gateway control instead of launching an invisible local-headless tab',
);

assert.match(
  navigator,
  /params\.set\("displayIsolation", launcherDisplayIsolation\)[\s\S]*params\.set\("viewStreamProvider", launcherViewStreamProvider\)[\s\S]*params\.set\("browserHost", "remote_headed"\)[\s\S]*params\.set\("controlInputProvider", launcherControlInputProvider\)/,
  'Workspace launcher access-plan requests must preserve selected remote view and control posture before launch',
);

assert.doesNotMatch(
  navigator,
  /setLauncherDisplayIsolation\("service_default"\)|setLauncherViewStreamProvider\("service_default"\)|setLauncherControlInputProvider\("service_default"\)/,
  'Workspace launcher planning must not reset user-selected RDP and control posture back to service defaults',
);

assert.doesNotMatch(
  navigator,
  /preview\.rows\.slice\(/,
  'Workspace launcher must not hide browser/profile combinations behind a hard-coded visible row cap',
);

assert.match(
  navigator,
  /const rows = preview\.rows;[\s\S]*eligible: preview\.summary\.eligible[\s\S]*needsOperatorAction: preview\.summary\.needsOperatorAction[\s\S]*blocked: preview\.summary\.blocked[\s\S]*accessPlanFetched: preview\.summary\.accessPlanFetched/,
  'Workspace launcher summary must reflect the full browser/profile combination set',
);

assert.match(
  navigator,
  /const LAUNCHER_ROW_WINDOW = 48;[\s\S]*const filteredRows = useMemo[\s\S]*launcherRowSearchText\(row\)\.includes\(query\)[\s\S]*const visibleRows = filteredRows\.slice\(0, visibleRowCount\)[\s\S]*workspace-launcher-show-more/,
  'Workspace launcher must filter and incrementally render large browser/profile sets without changing full-count summaries',
);

assert.match(
  navigator,
  /node\.takeover\?\.ownerLabel[\s\S]*node\.takeover\?\.queueImpact/,
  'Workspace navigator search must include human takeover owner and queue-impact text',
);

assert.match(
  navigator,
  /function nodeStatusLabel\(node: WorkspaceNode\): string \{[\s\S]*if \(node\.takeover\?\.active\) return "takeover";[\s\S]*if \(node\.state === "blocked"\) return "needs review";/,
  'Workspace navigator rows must label service-owned human takeover state directly and avoid raw blocked wording',
);

assert.match(
  navigator,
  /function primaryAction\(node: WorkspaceNode\)[\s\S]*const operatorControlIds: WorkspaceNodeActionId\[\] = \["control", "view"\][\s\S]*candidate\.id === id && candidate\.enabled[\s\S]*if \(node\.takeover\?\.active\) \{[\s\S]*action\.id === "resume"/,
  'Workspace navigator primary action must prefer remote Control/View for takeover rows before falling back to disabled Resume',
);

assert.match(
  navigator,
  /RotateCcw[\s\S]*action\.id === "resume" \? <RotateCcw/,
  'Workspace navigator takeover Resume affordance must use a familiar resume icon',
);

assert.match(
  navigator,
  /<AlertDialog[\s\S]*pendingDangerAction[\s\S]*Confirm/,
  'Workspace navigator destructive actions must use shadcn AlertDialog instead of native dialogs',
);

assert.match(
  css,
  /\.workspace-nav-header[\s\S]*\.workspace-nav-scope[\s\S]*grid-template-columns: repeat\(4, minmax\(0, 1fr\)\)[\s\S]*\.workspace-nav-dismissed-restore[\s\S]*\.workspace-nav-row[\s\S]*grid-template-columns: minmax\(0, 1fr\) auto/,
  'Workspace navigator CSS must keep compact controls and stable row dimensions',
);

assert.match(
  css,
  /\.workspace-nav-scroll[\s\S]*overflow-y: auto[\s\S]*touch-action: pan-y[\s\S]*-webkit-overflow-scrolling: touch[\s\S]*@media \(max-width: 767px\)[\s\S]*\.dashboard-mobile-panel\[data-state="active"\][\s\S]*\.workspace-nav-row-side \.workspace-nav-icon-action[\s\S]*display: none[\s\S]*\.workspace-nav-row-selected \.workspace-nav-row-side \.workspace-nav-icon-action[\s\S]*display: inline-flex/,
  'Workspace navigator mobile CSS must keep the pane scrollable and hide tile controls until a row is selected or focused',
);

assert.doesNotMatch(
  css,
  /hsl\(var\(--/,
  'Workspace navigator CSS must use the app color variables directly instead of invalid hsl(var(--color-token)) syntax',
);

assert.match(
  selector,
  /pnpm test:dashboard-workspace-navigator/,
  'Validation selector must recommend the focused workspace navigator test',
);

for (const [label, text] of [
  ['README', readme],
  ['dashboard docs', dashboardDocs],
  ['service mode docs', serviceModeDocs],
  ['commands docs', commandsDocs],
  ['skill', skill],
  ['CLI help', cliOutput],
]) {
  assert.match(
    text,
    /view=service:|service:&lt;name&gt;|service:<name>/,
    `${label} must document service record tabs through view=service:<name>`,
  );
}

for (const [label, text] of [
  ['README', readme],
  ['dashboard docs', dashboardDocs],
  ['service mode docs', serviceModeDocs],
  ['skill', skill],
  ['CLI help', cliOutput],
]) {
  assert.match(
    text,
    /workspace[\s\S]*browser[\s\S]*session[\s\S]*tab[\s\S]*profile[\s\S]*job/,
    `${label} must document selected workspace identity query parameters`,
  );
}

for (const [label, text] of [
  ['README', readme],
  ['dashboard docs', dashboardDocs],
  ['service mode docs', serviceModeDocs],
  ['commands docs', commandsDocs],
  ['skill', skill],
  ['CLI help', cliOutput],
]) {
  assert.match(
    text,
    /launch[\s\S]*(browser|tab) identity[\s\S]*(workspace viewport|embedded workspace viewport|Service Jobs|Jobs)/i,
    `${label} must document post-launch viewport or Jobs routing`,
  );
}

for (const [label, text] of [
  ['README', readme],
  ['dashboard docs', dashboardDocs],
  ['service mode docs', serviceModeDocs],
  ['skill', skill],
]) {
  assert.doesNotMatch(
    text,
    /\/service\?workspace=<name>|\?workspace=<\/code>|\/service\?workspace=&lt;name&gt;/,
    `${label} must not document service record tabs as /service?workspace=<name>`,
  );
}

console.log('Dashboard workspace navigator structure smoke passed');
