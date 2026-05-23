#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const servicePanel = readFileSync('packages/dashboard/src/components/service-panel.tsx', 'utf8');
const dashboardCss = readFileSync('packages/dashboard/src/app/globals.css', 'utf8');
const validationSelector = readFileSync('scripts/dev/select-validation.js', 'utf8');

assert.match(
  servicePanel,
  /type BrowserLifecycleFilter = "actionable" \| "all" \| "live" \| "retained";/,
  'Browser table must keep explicit live, retained, all, and actionable lifecycle filters',
);

assert.match(
  servicePanel,
  /type BrowserStreamFilter = "all" \| "with_stream" \| "without_stream";/,
  'Browser table must keep an explicit view-stream availability filter type',
);

assert.match(
  servicePanel,
  /type BrowserTableColumnKey = "health" \| "profile" \| "host" \| "ownership" \| "sessions" \| "streams" \| "lastError";/,
  'Browser table must keep ownership as a first-class visible column',
);

assert.match(
  servicePanel,
  /type BrowserOwnershipSummary = \{[\s\S]*serviceNames: string\[\];[\s\S]*agentNames: string\[\];[\s\S]*taskNames: string\[\];[\s\S]*sessionIds: string\[\];/,
  'Browser table ownership summaries must expose service, agent, task, and session evidence',
);

assert.match(
  servicePanel,
  /browserBuild\?: string \| null;/,
  'Browser table must accept service-provided browserBuild when the browser record exposes it',
);

assert.match(
  servicePanel,
  /BROWSER_TABLE_LIFECYCLE_FILTER_STORAGE_KEY = "agent-browser-dashboard-browser-table-lifecycle-filter"/,
  'Browser table must persist the selected lifecycle filter under a stable localStorage key',
);

assert.match(
  servicePanel,
  /BROWSER_TABLE_VISIBLE_COLUMNS_STORAGE_KEY = "agent-browser-dashboard-browser-table-visible-columns"/,
  'Browser table must persist visible column preferences under a stable localStorage key',
);

assert.match(
  servicePanel,
  /BROWSER_TABLE_COLUMN_WIDTHS_STORAGE_KEY = "agent-browser-dashboard-browser-table-column-widths"/,
  'Browser table must persist adjusted column widths under a stable localStorage key',
);

assert.match(
  servicePanel,
  /BROWSER_TABLE_DENSITY_STORAGE_KEY = "agent-browser-dashboard-browser-table-density"/,
  'Browser table must persist row density under a stable localStorage key',
);

assert.match(
  servicePanel,
  /const BROWSER_TABLE_INITIAL_ROW_LIMIT = 50;/,
  'Browser table must keep an explicit initial row window for large retained-state sets',
);

assert.match(
  servicePanel,
  /const BROWSER_TABLE_ROW_LIMIT_STEP = 50;/,
  'Browser table must keep an explicit row-window expansion step',
);

assert.match(
  servicePanel,
  /DEFAULT_BROWSER_TABLE_COLUMN_WIDTHS: Record<BrowserTableColumnId, number>/,
  'Browser table must define default widths for every adjustable column',
);

assert.match(
  servicePanel,
  /BROWSER_TABLE_VIEW_STORAGE_KEYS = \[[\s\S]*BROWSER_TABLE_LIFECYCLE_FILTER_STORAGE_KEY[\s\S]*BROWSER_TABLE_VISIBLE_COLUMNS_STORAGE_KEY[\s\S]*BROWSER_TABLE_COLUMN_WIDTHS_STORAGE_KEY[\s\S]*BROWSER_TABLE_DENSITY_STORAGE_KEY[\s\S]*\]/,
  'Browser table must keep a single reset list for persisted view preference keys',
);

assert.match(
  servicePanel,
  /function initialBrowserLifecycleFilter\(\): BrowserLifecycleFilter[\s\S]*return isBrowserLifecycleFilter\(stored\) \? stored : "actionable";/,
  'Browser table must validate persisted lifecycle filters and default to actionable records',
);

assert.match(
  servicePanel,
  /function initialBrowserTableColumns\(\): BrowserTableColumnKey\[\][\s\S]*parsed\.filter\(isBrowserTableColumnKey\)[\s\S]*DEFAULT_BROWSER_TABLE_COLUMNS/,
  'Browser table must validate persisted visible columns before applying them',
);

assert.match(
  servicePanel,
  /function initialBrowserTableColumnWidths\(\): Record<BrowserTableColumnId, number>[\s\S]*clampBrowserTableColumnWidth\(value\)/,
  'Browser table must validate persisted column widths before applying them',
);

assert.match(
  servicePanel,
  /function initialBrowserTableDensity\(\): BrowserTableDensity[\s\S]*return isBrowserTableDensity\(stored\) \? stored : "standard";/,
  'Browser table must validate persisted row density before applying it',
);

assert.match(
  servicePanel,
  /const \[lifecycleFilter, setLifecycleFilter\] = useState<BrowserLifecycleFilter>\(initialBrowserLifecycleFilter\);/,
  'Browser table lifecycle state must use the persisted preference initializer',
);

assert.match(
  servicePanel,
  /const \[rowLimit, setRowLimit\] = useState\(BROWSER_TABLE_INITIAL_ROW_LIMIT\);/,
  'Browser table must track the visible row window in component state',
);

assert.match(
  servicePanel,
  /const \[healthFilter, setHealthFilter\] = useState\("all"\);[\s\S]*const \[hostFilter, setHostFilter\] = useState\("all"\);[\s\S]*const \[browserBuildFilter, setBrowserBuildFilter\] = useState\("all"\);[\s\S]*const \[streamFilter, setStreamFilter\] = useState<BrowserStreamFilter>\("all"\);[\s\S]*const \[ownershipServiceFilter, setOwnershipServiceFilter\] = useState\("all"\);[\s\S]*const \[ownershipAgentFilter, setOwnershipAgentFilter\] = useState\("all"\);[\s\S]*const \[ownershipTaskFilter, setOwnershipTaskFilter\] = useState\("all"\);/,
  'Browser table must track health, host, browser-build, stream, and ownership filters',
);

assert.match(
  servicePanel,
  /const rowButtonRefs = useRef\(new Map<string, HTMLButtonElement>\(\)\);/,
  'Browser table must keep stable row button refs for keyboard row navigation',
);

assert.match(
  servicePanel,
  /const \[visibleColumns, setVisibleColumns\] = useState<BrowserTableColumnKey\[\]>\(initialBrowserTableColumns\);/,
  'Browser table visible columns state must use the persisted preference initializer',
);

assert.match(
  servicePanel,
  /function browserOwnershipSummary\(browser: ServiceBrowser, sessions: ServiceSession\[\]\): BrowserOwnershipSummary[\s\S]*session\.browserIds\?\.includes\(browser\.id\)[\s\S]*browser\.activeSessionIds\?\.includes\(session\.id\)[\s\S]*serviceNames: uniqueStringValues\(linkedSessions\.map\(\(session\) => session\.serviceName\)\)[\s\S]*agentNames: uniqueStringValues\(linkedSessions\.map\(\(session\) => session\.agentName\)\)[\s\S]*taskNames: uniqueStringValues\(linkedSessions\.map\(\(session\) => session\.taskName\)\)/,
  'Browser table ownership must derive from service session links instead of frontend guesses',
);

assert.match(
  servicePanel,
  /function browserOwnershipSearchText\(ownership: BrowserOwnershipSummary\): string[\s\S]*ownership\.serviceNames[\s\S]*ownership\.agentNames[\s\S]*ownership\.taskNames[\s\S]*ownership\.sessionIds/,
  'Browser table text search must include service-owned browser ownership evidence',
);

assert.match(
  servicePanel,
  /const healthOptions = useMemo\(\(\) => browserFilterOptionValues\(browsers, "health"\)[\s\S]*const hostOptions = useMemo\(\(\) => browserFilterOptionValues\(browsers, "host"\)[\s\S]*const browserBuildOptions = useMemo\(\(\) => browserFilterOptionValues\(browsers, "browserBuild"\)/,
  'Browser table must derive health, host, and browser-build filter options from records',
);

assert.match(
  servicePanel,
  /function BrowserTable\(\{[\s\S]*browsers,[\s\S]*sessions,[\s\S]*onSelect,[\s\S]*onViewStream,[\s\S]*onFocusViewStream,[\s\S]*onCloseBrowser,[\s\S]*onRepairBrowser,[\s\S]*closeSupported,[\s\S]*repairSupported,[\s\S]*activeSessionName,[\s\S]*actingBrowserActionId,[\s\S]*selectedBrowserId,[\s\S]*\}: \{[\s\S]*browsers: ServiceBrowser\[\];[\s\S]*sessions: ServiceSession\[\];[\s\S]*onSelect: \(browser: ServiceBrowser\) => void;[\s\S]*onViewStream\?: \(browser: ServiceBrowser\) => void;[\s\S]*onFocusViewStream\?: \(browser: ServiceBrowser\) => void;[\s\S]*onCloseBrowser\?: \(browser: ServiceBrowser\) => void;[\s\S]*onRepairBrowser\?: \(browser: ServiceBrowser\) => void;/,
  'Browser table must accept service sessions and row action callbacks',
);

assert.match(
  servicePanel,
  /const browserOwnershipById = useMemo\([\s\S]*new Map\(browsers\.map\(\(browser\) => \[browser\.id, browserOwnershipSummary\(browser, sessions\)\]\)\)[\s\S]*\[browsers, sessions\]/,
  'Browser table must memoize ownership summaries from browsers and sessions',
);

assert.match(
  servicePanel,
  /const browserOwnershipValues = useMemo\(\(\) => Array\.from\(browserOwnershipById\.values\(\)\), \[browserOwnershipById\]\);[\s\S]*const ownershipServiceOptions = useMemo\([\s\S]*ownership\.serviceNames[\s\S]*const ownershipAgentOptions = useMemo\([\s\S]*ownership\.agentNames[\s\S]*const ownershipTaskOptions = useMemo\([\s\S]*ownership\.taskNames/,
  'Browser table must derive service, agent, and task filter options from ownership summaries',
);

assert.match(
  servicePanel,
  /ownershipServiceFilter !== "all"[\s\S]*ownership\.serviceNames\.includes\(ownershipServiceFilter\)[\s\S]*ownershipAgentFilter !== "all"[\s\S]*ownership\.agentNames\.includes\(ownershipAgentFilter\)[\s\S]*ownershipTaskFilter !== "all"[\s\S]*ownership\.taskNames\.includes\(ownershipTaskFilter\)/,
  'Browser table must apply service, agent, and task ownership filters before text search',
);

assert.match(
  servicePanel,
  /\["health", "id", "profile", "host", "ownership", "sessions", "streams", "lastError", "actions"\] as BrowserTableColumnId\[\]/,
  'Browser table must include ownership in the active column ordering',
);

assert.match(
  servicePanel,
  /localStorage\.setItem\(BROWSER_TABLE_LIFECYCLE_FILTER_STORAGE_KEY, lifecycleFilter\)/,
  'Browser table must save lifecycle filter changes locally',
);

assert.match(
  servicePanel,
  /localStorage\.setItem\(BROWSER_TABLE_VISIBLE_COLUMNS_STORAGE_KEY, JSON\.stringify\(visibleColumns\)\)/,
  'Browser table must save visible column changes locally',
);

assert.match(
  servicePanel,
  /localStorage\.setItem\(BROWSER_TABLE_COLUMN_WIDTHS_STORAGE_KEY, JSON\.stringify\(columnWidths\)\)/,
  'Browser table must save column width changes locally',
);

assert.match(
  servicePanel,
  /localStorage\.setItem\(BROWSER_TABLE_DENSITY_STORAGE_KEY, density\)/,
  'Browser table must save row density changes locally',
);

assert.match(
  servicePanel,
  /function BrowserTableHeaderCell\([\s\S]*service-browser-table-resize[\s\S]*onMouseDown=\{\(event\) => onResizeStart\(column, event\)\}/,
  'Browser table headers must expose resize handles',
);

assert.match(
  servicePanel,
  /window\.addEventListener\("mousemove", handleMouseMove\)/,
  'Browser table column resizing must attach mousemove listeners',
);

assert.match(
  servicePanel,
  /window\.removeEventListener\("mousemove", handleMouseMove\)/,
  'Browser table column resizing must remove mousemove listeners',
);

assert.match(
  servicePanel,
  /const TERMINAL_BROWSER_HEALTH = new Set\(\["closed", "faulted", "not_started", "process_exited"\]\);[\s\S]*function isLiveBrowserRecord\(browser: ServiceBrowser\): boolean \{[\s\S]*TERMINAL_BROWSER_HEALTH\.has\(\(browser\.health \?\? ""\)\.toLowerCase\(\)\)[\s\S]*return false[\s\S]*browser\.pid/,
  'Browser table must not classify terminal health records as live just because stale PID evidence exists',
);

assert.match(
  servicePanel,
  /function isInertRetainedBrowserRecord\(browser: ServiceBrowser\): boolean[\s\S]*browser\.health[\s\S]*"not_started"/,
  'Browser table must classify inert retained not_started browser records explicitly',
);

assert.match(
  servicePanel,
  /const resetTableView = \(\) => \{[\s\S]*setLifecycleFilter\("actionable"\)[\s\S]*setHealthFilter\("all"\)[\s\S]*setHostFilter\("all"\)[\s\S]*setBrowserBuildFilter\("all"\)[\s\S]*setStreamFilter\("all"\)[\s\S]*setOwnershipServiceFilter\("all"\)[\s\S]*setOwnershipAgentFilter\("all"\)[\s\S]*setOwnershipTaskFilter\("all"\)[\s\S]*setVisibleColumns\(DEFAULT_BROWSER_TABLE_COLUMNS\)[\s\S]*setColumnWidths\(DEFAULT_BROWSER_TABLE_COLUMN_WIDTHS\)[\s\S]*setDensity\("standard"\)[\s\S]*localStorage\.removeItem\(key\)/,
  'Browser table must reset all persisted view state at once',
);

assert.match(
  servicePanel,
  /useEffect\(\(\) => \{[\s\S]*setRowLimit\(BROWSER_TABLE_INITIAL_ROW_LIMIT\);[\s\S]*\}, \[browserBuildFilter, filter, healthFilter, hostFilter, lifecycleFilter, ownershipAgentFilter, ownershipServiceFilter, ownershipTaskFilter, sortDirection, sortKey, streamFilter\]\);/,
  'Browser table must reset the row window when filtering or sorting changes',
);

assert.match(
  servicePanel,
  /const navigateBrowserRows = \(browser: ServiceBrowser, event: ReactKeyboardEvent<HTMLButtonElement>\) => \{[\s\S]*event\.key === "ArrowDown"[\s\S]*event\.key === "ArrowUp"[\s\S]*event\.key === "Home"[\s\S]*event\.key === "End"/,
  'Browser table must support ArrowUp, ArrowDown, Home, and End row navigation from browser row buttons',
);

assert.match(
  servicePanel,
  /id="service-browser-table-keyboard-hint" className="sr-only"[\s\S]*Arrow Up, Arrow Down, Home, and End/,
  'Browser table must expose a screen-reader hint for row keyboard navigation',
);

assert.match(
  servicePanel,
  /rowButtonRef=\{browser\.id \? \(node\) => setRowButtonRef\(browser\.id, node\) : undefined\}[\s\S]*onKeyDown=\{\(event\) => onNavigate\(browser, event\)\}[\s\S]*aria-describedby="service-browser-table-keyboard-hint"/,
  'Browser row buttons must wire refs, keyboard handling, and the keyboard hint together',
);

assert.match(
  servicePanel,
  /<BrowserTableHeaderCell column="ownership"[\s\S]*Ownership[\s\S]*<\/BrowserTableHeaderCell>/,
  'Browser table must render an ownership column header',
);

assert.match(
  servicePanel,
  /ownership=\{browserOwnershipById\.get\(browser\.id\) \?\? EMPTY_BROWSER_OWNERSHIP\}/,
  'Browser table rows must receive service-derived ownership summaries',
);

assert.match(
  servicePanel,
  /function BrowserOwnershipCell\(\{ ownership \}: \{ ownership: BrowserOwnershipSummary \}\)[\s\S]*svc \{formatStringList\(ownership\.serviceNames, "unknown"\)\}[\s\S]*agent \{formatStringList\(ownership\.agentNames, "unknown"\)\}[\s\S]*task \{formatStringList\(ownership\.taskNames, "unknown"\)\}/,
  'Browser table ownership cell must show service, agent, and task chips',
);

assert.match(
  servicePanel,
  /<BrowserTable[\s\S]*browsers=\{browserRecords\}[\s\S]*sessions=\{sessionRecords\}[\s\S]*onSelect=\{inspectBrowser\}[\s\S]*onViewStream=\{openBrowserViewStream\}[\s\S]*onFocusViewStream=\{focusBrowserViewStream\}[\s\S]*onCloseBrowser=\{closeServiceBrowser\}[\s\S]*onRepairBrowser=\{repairServiceBrowser\}[\s\S]*closeSupported=\{browserCloseSupported\}[\s\S]*repairSupported=\{browserRepairSupported\}[\s\S]*activeSessionName=\{activeSession\}[\s\S]*actingBrowserActionId=\{actingBrowserActionId\}[\s\S]*selectedBrowserId=\{selectedBrowserId\}/,
  'Service dashboard must pass service sessions and row action callbacks into the browser table',
);

assert.match(
  dashboardCss,
  /\.service-browser-ownership-cell[\s\S]*\.service-browser-ownership-chip[\s\S]*\.service-browser-ownership-service/,
  'Browser ownership chips must keep compact dedicated styling',
);

assert.match(
  servicePanel,
  /const browserTabsById = useMemo\(\(\) => \{[\s\S]*new Map<string, ServiceTab\[\]>\(\)[\s\S]*tab\.browserId[\s\S]*grouped\.set\(tab\.browserId/,
  'Service dashboard must group service-owned tabs by browser for browser-row focus actions',
);

assert.match(
  servicePanel,
  /const openBrowserViewStream = useCallback\(\(browser: ServiceBrowser\) => \{[\s\S]*browserPrimaryViewStream\(browser\)[\s\S]*openViewStream\(stream, browser\)/,
  'Browser row View must open the service-owned primary view stream',
);

assert.match(
  servicePanel,
  /const focusBrowserViewStream = useCallback\(async \(browser: ServiceBrowser\) => \{[\s\S]*browserPrimaryViewStream\(browser\)[\s\S]*action: "view_focus"[\s\S]*taskName: "focus-browser-row-view"[\s\S]*params: \{ index: tabIndex, maximize: true \}[\s\S]*openViewStream\(stream, browser, primaryTab, focusMessage\)/,
  'Browser row Focus must queue the existing service-owned view_focus action before opening the stream',
);

assert.match(
  servicePanel,
  /const closeServiceBrowser = useCallback\(async \(browser: ServiceBrowser\) => \{[\s\S]*action: "service_browser_close"[\s\S]*taskName: "close-browser-row"[\s\S]*params: \{ browserId: browser\.id \}[\s\S]*await fetchService\(false\)/,
  'Browser row Close must queue the service-owned browser close request and refresh service state',
);

assert.match(
  servicePanel,
  /const repairServiceBrowser = useCallback\(async \(browser: ServiceBrowser\) => \{[\s\S]*action: "service_browser_repair"[\s\S]*taskName: "repair-browser-row"[\s\S]*browserId: browser\.id[\s\S]*Dashboard row repair requested[\s\S]*await fetchService\(false\)/,
  'Browser row Repair must queue the service-owned browser repair request and refresh service state',
);

assert.match(
  servicePanel,
  /const serviceRequestActions = useMemo\([\s\S]*contracts\?\.contracts\?\.serviceRequest\?\.actions[\s\S]*const browserCloseSupported = serviceRequestActions\.has\("service_browser_close"\);[\s\S]*const browserRepairSupported = serviceRequestActions\.has\("service_browser_repair"\);/,
  'Browser row remedies must be gated by advertised service request actions',
);

assert.match(
  servicePanel,
  /const contractsPromise = fetch\(`\$\{serviceBase\(activePort\)\}\/contracts`\)\.catch\(\(\) => null\);[\s\S]*contractsPromise,[\s\S]*const contractsJson = contractsResp\?\.ok[\s\S]*: null;[\s\S]*setContracts\(contractsJson\?\.success \? contractsJson\.data \?\? null : null\);/,
  'Service contracts discovery must be optional so older services render with row remedies disabled',
);

assert.match(
  servicePanel,
  /function BrowserTableRow\([\s\S]*onViewStream,[\s\S]*onFocusViewStream,[\s\S]*onCloseBrowser,[\s\S]*onRepairBrowser,[\s\S]*closeSupported,[\s\S]*repairSupported,[\s\S]*activeSessionName,[\s\S]*acting,[\s\S]*onViewStream\?: \(browser: ServiceBrowser\) => void;[\s\S]*onFocusViewStream\?: \(browser: ServiceBrowser\) => void;[\s\S]*onCloseBrowser\?: \(browser: ServiceBrowser\) => void;[\s\S]*onRepairBrowser\?: \(browser: ServiceBrowser\) => void;/,
  'Browser table rows must receive view, focus, close, and repair action callbacks explicitly',
);

assert.match(
  servicePanel,
  /const closeAvailable = Boolean\(closeSupported && onCloseBrowser && activeSessionName && browser\.id === `session:\$\{activeSessionName\}`\);/,
  'Browser row Close must only enable for the active service browser row',
);

assert.match(
  servicePanel,
  /const repairAvailable = Boolean\(repairSupported && onRepairBrowser && \["degraded", "faulted"\]\.includes\(\(browser\.health \?\? ""\)\.toLowerCase\(\)\)\);/,
  'Browser row Repair must only enable for degraded or faulted browser rows',
);

assert.match(
  servicePanel,
  /browserRowCloseTitle,[\s\S]*browserRowRepairTitle,[\s\S]*from "@\/lib\/service-browser-row-actions";/,
  'Browser row remedy disabled titles must come from the shared rendered-title helper',
);

assert.match(
  servicePanel,
  /const closeTitle = browserRowCloseTitle\(\{[\s\S]*available: closeAvailable,[\s\S]*supported: Boolean\(closeSupported\),[\s\S]*\}\);/,
  'Browser row Close must derive rendered disabled reasons from support and eligibility',
);

assert.match(
  servicePanel,
  /const repairTitle = browserRowRepairTitle\(\{[\s\S]*available: repairAvailable,[\s\S]*supported: Boolean\(repairSupported\),[\s\S]*\}\);/,
  'Browser row Repair must distinguish unsupported backend capability from row ineligibility',
);

assert.match(
  servicePanel,
  /function BrowserRowActions\(\{[\s\S]*const viewStreamAvailable = canOpenViewStream\(primaryViewStream\);[\s\S]*const controlAvailable = canOpenControlViewStream\(primaryViewStream\);[\s\S]*const unavailableActionCount = \[[\s\S]*!viewStreamAvailable \|\| !onViewStream[\s\S]*!controlAvailable \|\| !onFocusViewStream[\s\S]*!closeAvailable[\s\S]*!repairAvailable[\s\S]*service-browser-row-actions[\s\S]*Inspect[\s\S]*\{viewStreamAvailable && onViewStream && \([\s\S]*View[\s\S]*\{controlAvailable && onFocusViewStream && \([\s\S]*Control[\s\S]*\{closeAvailable && \([\s\S]*AlertDialog[\s\S]*Close[\s\S]*\{repairAvailable && \([\s\S]*Repair[\s\S]*\{unavailableActionCount > 0 && \([\s\S]*Unavailable actions/,
  'Browser row actions must use a shared action component with enabled actions inline and unavailable reasons in a row menu',
);

assert.match(
  servicePanel,
  /className="service-browser-card-list" aria-label="Managed browser cards"[\s\S]*visibleBrowsers\.map\(\(browser, index\) => \([\s\S]*<BrowserTableCard[\s\S]*onViewStream=\{onViewStream\}[\s\S]*onFocusViewStream=\{onFocusViewStream\}[\s\S]*onCloseBrowser=\{onCloseBrowser\}[\s\S]*onRepairBrowser=\{onRepairBrowser\}/,
  'Browser table must render mobile managed-browser cards from the same visible browser window and action callbacks',
);

assert.match(
  servicePanel,
  /function BrowserTableCard\([\s\S]*service-browser-card-primary[\s\S]*service-browser-card-grid[\s\S]*<BrowserOwnershipCell ownership=\{ownership\} \/>[\s\S]*<BrowserRowActions/,
  'Mobile browser cards must expose primary browser details, ownership evidence, and shared row actions',
);

assert.match(
  servicePanel,
  /function RemoteViewReadinessStrip\(\{ browser, stream \}: \{ browser: ServiceBrowser; stream\?: ServiceViewStream \| null \}\)[\s\S]*canOpenViewStream\(stream\)[\s\S]*canOpenControlViewStream\(stream\)[\s\S]*aria-label="Remote view readiness"[\s\S]*Remote view[\s\S]*Remote control[\s\S]*Display[\s\S]*Gateway URL/,
  'Browser detail inspector must show remote view, control, and display readiness from service metadata',
);

assert.match(
  servicePanel,
  /<RemoteViewReadinessStrip browser=\{browser\} stream=\{primaryViewStream\} \/>/,
  'Browser detail content must render the remote view readiness strip for selected browsers',
);

assert.match(
  dashboardCss,
  /\.service-remote-view-readiness[\s\S]*grid-template-columns: repeat\(5, minmax\(0, 1fr\)\)[\s\S]*\.service-remote-view-readiness p[\s\S]*grid-column: 1 \/ -1/,
  'Remote view readiness strip must use compact multi-column styling with a full-width context line',
);

assert.match(
  servicePanel,
  /title=\{closeTitle\}/,
  'Browser row Close must explain the active-service-browser safety rail when disabled',
);

assert.match(
  servicePanel,
  /title=\{repairTitle\}/,
  'Browser row Repair must explain backend support or health-state eligibility when disabled',
);

assert.match(
  dashboardCss,
  /\.service-browser-table-streams[\s\S]*grid[\s\S]*\.service-browser-row-actions[\s\S]*min-width: 12rem[\s\S]*flex-wrap: wrap[\s\S]*justify-content: flex-end[\s\S]*\.service-browser-card[\s\S]*\.service-browser-card-grid[\s\S]*@media \(max-width: 767px\)[\s\S]*\.service-browser-table-scroll[\s\S]*display: none[\s\S]*\.service-browser-card-list[\s\S]*display: grid/,
  'Browser row actions must keep compact wrapping action-group styling and stream posture labels',
);

assert.match(
  servicePanel,
  /DropdownMenuCheckboxItem[\s\S]*Visible columns[\s\S]*Reset columns[\s\S]*Reset widths[\s\S]*Reset table view/,
  'Browser table must expose column visibility plus width and full view reset controls',
);

assert.match(
  servicePanel,
  /service-browser-table-density[\s\S]*Compact[\s\S]*Standard[\s\S]*Expanded/,
  'Browser table must expose compact, standard, and expanded density controls',
);

assert.match(
  servicePanel,
  /service-browser-table-control-group[\s\S]*Records[\s\S]*service-browser-table-density[\s\S]*Density[\s\S]*service-browser-table-column-menu[\s\S]*Layout/,
  'Browser table toolbar must group record, density, and layout controls explicitly',
);

assert.match(
  servicePanel,
  /service-browser-table-controls" aria-label="Browser table controls"[\s\S]*aria-label="Browser record lifecycle filters"[\s\S]*aria-label="Browser table density"[\s\S]*service-browser-table-column-menu/,
  'Browser table toolbar groups must expose stable accessibility labels for browser-driven smoke checks',
);

assert.match(
  servicePanel,
  /DropdownMenuTrigger asChild[\s\S]*<Button size="sm" variant="outline"[\s\S]*<MoreHorizontal className="size-3" \/>[\s\S]*Columns[\s\S]*DropdownMenuContent align="end" className="w-44"/,
  'Browser table column menu must keep a compact accessible trigger and right-aligned menu content',
);

assert.match(
  servicePanel,
  /DropdownMenuLabel>Visible columns<\/DropdownMenuLabel>[\s\S]*BROWSER_TABLE_COLUMNS\.map[\s\S]*DropdownMenuCheckboxItem[\s\S]*checked=\{visibleColumnSet\.has\(column\.key\)\}/,
  'Browser table column menu must render the configured visible columns as checked menu items',
);

assert.match(
  servicePanel,
  /service-browser-table-density-\$\{density\}/,
  'Browser table must apply the selected density as a table class',
);

assert.match(
  servicePanel,
  /const managedRecordDetail = useMemo\(\(\) => \[[\s\S]*retained browser records[\s\S]*managed profile records[\s\S]*service sessions[\s\S]*tracked tabs[\s\S]*site policies[\s\S]*providers/,
  'Service dashboard must explain retained managed-state counts through one compact record detail summary',
);

assert.match(
  servicePanel,
  /function isActiveServiceJob\(job: ServiceJob\): boolean[\s\S]*state === "queued" \|\| state === "running"/,
  'Service dashboard must classify queued and running jobs as active work',
);

assert.match(
  servicePanel,
  /function isActiveServiceSession\(session: ServiceSession\): boolean[\s\S]*session\.browserIds\?\.length[\s\S]*session\.tabIds\?\.length/,
  'Service dashboard must classify sessions with browsers or tabs as active work',
);

assert.match(
  servicePanel,
  /const sessionActivitySummary = useMemo\(\(\) => \{[\s\S]*activeSessions[\s\S]*retainedSessions[\s\S]*activeTabs[\s\S]*retainedTabs/,
  'Service dashboard must derive active and retained summaries for sessions and tabs',
);

assert.match(
  servicePanel,
  /const jobActivitySummary = useMemo\(\(\) => \{[\s\S]*active = retainedServiceJobs\.filter\(isActiveServiceJob\)\.length[\s\S]*terminal = retainedServiceJobs\.filter\(isRetainedTerminalServiceJob\)\.length[\s\S]*retained/,
  'Service dashboard must derive active, terminal, and retained summaries for service jobs',
);

assert.match(
  servicePanel,
  /label: "Sessions"[\s\S]*count: sessionActivitySummary\.activeSessions[\s\S]*detail: `\$\{sessionActivitySummary\.retainedSessions\} retained`/,
  'Service workspace Sessions tab must badge only session work and label retained session history',
);

assert.match(
  servicePanel,
  /label: "Tabs"[\s\S]*count: sessionActivitySummary\.activeTabs[\s\S]*detail: `\$\{sessionActivitySummary\.retainedTabs\} retained`/,
  'Service workspace Tabs tab must badge tab work separately from sessions',
);

assert.match(
  servicePanel,
  /label: "Jobs"[\s\S]*count: jobActivitySummary\.active[\s\S]*detail: `\$\{jobActivitySummary\.retained\} retained`/,
  'Service workspace Jobs tab must badge queued or running jobs and label retained history separately',
);

assert.match(
  servicePanel,
  /<ServiceStatusLight[\s\S]*label="Queue"[\s\S]*Control-plane worker is/,
  'Service status strip must explain the worker as the queue indicator instead of a separate unexplained Worker light',
);

assert.match(
  servicePanel,
  /<ServiceStatusLight[\s\S]*label="Control health"[\s\S]*onClick=\{\(\) => setWorkspaceTab\("browsers"\)\}/,
  'Service status strip must avoid a conflicting Browser label and drill control health into browser records',
);

assert.match(
  servicePanel,
  /<ServiceStatusLight[\s\S]*label="Jobs"[\s\S]*onClick=\{\(\) => setWorkspaceTab\("jobs"\)\}/,
  'Service Jobs status light must drill into the Jobs workspace',
);

assert.match(
  servicePanel,
  /<ServiceStatusLight[\s\S]*label="Records"[\s\S]*value=\{`\$\{entityCounts\.browsers\} browsers`\}[\s\S]*detail=\{`Retained service-state counts: \$\{managedRecordDetail\}`\}[\s\S]*icon=\{GitBranch\}[\s\S]*onClick=\{\(\) => setWorkspaceTab\("browsers"\)\}/,
  'Service dashboard must expose retained record counts as a compact Records status light that drills into browser records',
);

assert.match(
  servicePanel,
  /const managedAttentionCount = \[[\s\S]*reconciliation\?\.lastError,[\s\S]*retainedStateCleanupNeeded,[\s\S]*\.filter\(Boolean\)\.length;/,
  'Service dashboard must derive a compact managed-state attention summary before rendering actionable alert details',
);

assert.match(
  servicePanel,
  /const \[workspaceTab, setWorkspaceTab\] = useState<ServiceWorkspaceTab>\("profiles"\);[\s\S]*label: "Profiles"[\s\S]*label: "Browsers"[\s\S]*detail: `\$\{browserRecords\.filter\(isLiveBrowserRecord\)\.length\} live`/,
  'Service workspace must default to Profiles and expose Browsers as a sibling tab instead of a preamble',
);

assert.match(
  servicePanel,
  /managedAttentionCount > 0[\s\S]*service-state-alerts service-state-alerts-inline[\s\S]*Review events[\s\S]*Review browsers[\s\S]*Review jobs/,
  'Service dashboard must render managed-state attention as actionable inline alerts',
);

assert.doesNotMatch(
  servicePanel,
  /service-entity-strip/,
  'Service dashboard must not reintroduce the bulky managed entity strip in the primary scan path',
);

assert.match(
  servicePanel,
  /browserDefaultRank\(left\) - browserDefaultRank\(right\)/,
  'Browser table sorting must keep non-ready or live records ahead of inert retained records',
);

assert.match(
  servicePanel,
  /if \(healthFilter !== "all" && browser\.health !== healthFilter\) return false;[\s\S]*if \(hostFilter !== "all" && browser\.host !== hostFilter\) return false;[\s\S]*if \(browserBuildFilter !== "all" && browser\.browserBuild !== browserBuildFilter\) return false;/,
  'Browser table must apply service-backed health, host, and browser-build filters before rendering',
);

assert.match(
  servicePanel,
  /const hasViewStream = \(browser\.viewStreams\?\.length \?\? 0\) > 0;[\s\S]*streamFilter === "with_stream"[\s\S]*streamFilter === "without_stream"/,
  'Browser table must filter by service-backed view-stream availability',
);

assert.match(
  servicePanel,
  /const visibleBrowsers = useMemo\([\s\S]*filteredBrowsers\.slice\(0, rowLimit\)[\s\S]*const hiddenBrowserCount = Math\.max\(0, filteredBrowsers\.length - visibleBrowsers\.length\)/,
  'Browser table must render a row window and compute hidden matching rows',
);

assert.match(
  servicePanel,
  /visibleBrowsers\.map\(\(browser, index\) => \(/,
  'Browser table body must render the bounded visible browser row window',
);

assert.match(
  dashboardCss,
  /\.service-browser-table-controls/,
  'Browser table lifecycle and column controls must have an explicit layout hook',
);

assert.match(
  dashboardCss,
  /\.service-status-strip[\s\S]*grid-template-columns: repeat\(auto-fit, minmax\(6\.6rem, 1fr\)\)/,
  'Service status strip must stay dense enough to include retained record status without another summary row',
);

assert.match(
  dashboardCss,
  /\.service-state-alerts-inline[\s\S]*\.service-state-alert[\s\S]*flex-wrap: wrap[\s\S]*\.service-retained-state-hint[\s\S]*\.service-state-alert-actions/,
  'Service dashboard must style exceptional managed-state alerts as compact actionable rows',
);

assert.match(
  dashboardCss,
  /\.service-workspace-tab-detail[\s\S]*\.service-workspace-summary-chips[\s\S]*\.service-workspace-summary-chips span/,
  'Service workspace must style retained-history tab details and job summary chips',
);

assert.doesNotMatch(
  dashboardCss,
  /\.service-entity-strip|\.service-entity-count-chip/,
  'Dashboard CSS must not keep the removed bulky managed entity strip styles',
);

assert.match(
  dashboardCss,
  /\.service-browser-table-toolbar[\s\S]*flex-wrap: wrap[\s\S]*justify-content: space-between/,
  'Browser table toolbar must wrap instead of crowding the filter and count at narrower widths',
);

assert.match(
  dashboardCss,
  /\.service-browser-table-controls[\s\S]*flex: 1 1 100%[\s\S]*flex-wrap: wrap[\s\S]*justify-content: flex-end/,
  'Browser table control groups must wrap as a full-width trailing control row',
);

assert.match(
  servicePanel,
  /service-browser-table-advanced-filters" aria-label="Browser table field filters"[\s\S]*Health[\s\S]*All health states[\s\S]*Host[\s\S]*All hosts[\s\S]*Build[\s\S]*All builds[\s\S]*Streams[\s\S]*View stream available[\s\S]*No view stream/,
  'Browser table must expose compact service-backed health, host, browser-build, and stream filters',
);

assert.match(
  servicePanel,
  /ownershipServiceOptions\.length > 0[\s\S]*<span>Service<\/span>[\s\S]*All services[\s\S]*ownershipAgentOptions\.length > 0[\s\S]*<span>Agent<\/span>[\s\S]*All agents[\s\S]*ownershipTaskOptions\.length > 0[\s\S]*<span>Task<\/span>[\s\S]*All tasks/,
  'Browser table must expose conditional service, agent, and task ownership filters',
);

assert.match(
  servicePanel,
  /\{browserBuildOptions\.length > 0 && \([\s\S]*<span>Build<\/span>/,
  'Browser table must only show the build filter when service browser records expose build values',
);

assert.match(
  dashboardCss,
  /\.service-browser-table-advanced-filters[\s\S]*grid-template-columns: repeat\(auto-fit, minmax\(9\.5rem, 1fr\)\)[\s\S]*\.service-browser-table-advanced-filters select[\s\S]*min-height: 1\.8rem/,
  'Browser table field filters must use compact responsive native select controls',
);

assert.match(
  dashboardCss,
  /\.service-browser-table-control-group,[\s\S]*\.service-browser-table-density[\s\S]*border-radius: 999px/,
  'Browser table toolbar groups must have compact pill styling',
);

assert.match(
  dashboardCss,
  /\.service-browser-table-resize[\s\S]*cursor: col-resize/,
  'Browser table resize handles must have an explicit resize affordance',
);

assert.match(
  servicePanel,
  /TabsContent value="browsers"[\s\S]*Browser rows should identify the browser build, runtime profile, owning service, agent, task, sessions, streams, and available controls[\s\S]*<BrowserTable[\s\S]*browsers=\{browserRecords\}[\s\S]*sessions=\{sessionRecords\}[\s\S]*onSelect=\{inspectBrowser\}[\s\S]*selectedBrowserId=\{selectedBrowserId\}/,
  'Service browser table must live in the Browsers sub-tab and receive service sessions plus selected browser state',
);

assert.match(
  servicePanel,
  /function BrowserExecutableBadge[\s\S]*browserExecutableKind[\s\S]*browserExecutablePlatform[\s\S]*service-executable-badge/,
  'Service browser and profile records must have a reusable executable and platform badge',
);

assert.match(
  servicePanel,
  /<BrowserExecutableBadge[\s\S]*browserBuild=\{browser\.browserBuild\}[\s\S]*executablePath=\{browser\.executablePath\}/,
  'Managed browser rows and cards must surface executable identity from service browser records',
);

assert.match(
  servicePanel,
  /<BrowserExecutableBadge[\s\S]*browserBuild=\{browserBuild\}[\s\S]*host=\{profile\.defaultBrowserHost\}/,
  'Runtime profile cards must show the preferred browser executable family and host badge',
);

assert.match(
  servicePanel,
  /<EventDetailItem label="Executable" value=\{browserExecutableDetail\(browser\)\}/,
  'Browser inspector must show the executable detail rather than only host and health',
);

assert.match(
  servicePanel,
  /TabsContent value="sessions"[\s\S]*Sessions: \{sessionActivitySummary\.activeSessions\} active[\s\S]*TabsContent value="tabs"[\s\S]*Tabs: \{sessionActivitySummary\.activeTabs\} active/,
  'Sessions and Tabs must be separate workspaces instead of a confusing two-column layout',
);

assert.doesNotMatch(
  servicePanel,
  /service-split-records/,
  'Service workspace must not render the old two-column Sessions and Tabs layout',
);

assert.match(
  servicePanel,
  /<details className="service-advanced-trace">[\s\S]*Advanced trace explorer[\s\S]*<TraceExplorer/,
  'Trace Explorer free-form filters must be hidden behind an advanced disclosure by default',
);

assert.match(
  dashboardCss,
  /\.service-browser-table-scroll[\s\S]*max-height: clamp\(16rem, 42vh, 34rem\)[\s\S]*overflow: auto/,
  'Browser table must be vertically bounded so operational records remain reachable',
);

assert.match(
  dashboardCss,
  /\.service-executable-badge[\s\S]*\.service-executable-icon[\s\S]*\.service-executable-platform/,
  'Executable badges must have compact icon and platform styling',
);

assert.match(
  dashboardCss,
  /\.service-advanced-trace[\s\S]*summary[\s\S]*cursor: pointer/,
  'Advanced trace explorer disclosure must have an explicit interactive affordance',
);

assert.match(
  servicePanel,
  /hiddenBrowserCount > 0[\s\S]*service-browser-table-window[\s\S]*Show \{Math\.min\(BROWSER_TABLE_ROW_LIMIT_STEP, hiddenBrowserCount\)\} more[\s\S]*Show all/,
  'Browser table must expose explicit Show more and Show all controls when records are hidden by the row window',
);

assert.match(
  dashboardCss,
  /\.service-browser-table-window[\s\S]*justify-content: flex-end[\s\S]*\.service-browser-table-window span[\s\S]*margin-right: auto/,
  'Browser table row-window controls must have a compact trailing layout',
);

assert.match(
  servicePanel,
  /<p className="service-workspace-title">Service records<\/p>[\s\S]*Profiles are first because browser routing and identity policy determine which sessions should exist/,
  'Service workspace copy must explain why Profiles lead the service plane',
);

assert.doesNotMatch(
  servicePanel + dashboardCss,
  /Service actions|Reconcile retained state|Dry-run repair|Apply reviewed cleanup|service-panel-action-button|service-operator-input/,
  'Service plane must not expose one-item refresh menus, unrelated repair cleanup actions, or ordinary-user operator identity controls',
);

assert.match(
  servicePanel,
  /action: "service_prune_retained"[\s\S]*serviceState[\s\S]*Dry-run prune[\s\S]*Apply prune/,
  'Retained-state warning must expose a real dry-run and guarded apply prune path backed by service_prune_retained',
);

assert.match(
  dashboardCss,
  /\.service-status-strip \{[\s\S]*display: grid[\s\S]*padding-bottom: 0\.1rem[\s\S]*\.service-workspace-card \{[\s\S]*grid-row: 3[\s\S]*\.service-workspace-header \{[\s\S]*padding: 0\.58rem 0\.65rem 0\.55rem/,
  'Service status indicators and record tabs must have separated grid rows and enough header padding to avoid visual overlap',
);

assert.doesNotMatch(
  servicePanel + dashboardCss,
  /service-operator-card|Operator identity|service-audit-actor|Action signer|Name recorded on dashboard actions/,
  'Audit signer controls must stay hidden from ordinary dashboard users',
);

assert.match(
  servicePanel,
  /const operatorIdentity = "default";/,
  'Dashboard actions must use the hidden default signer until multi-user identity exists',
);

assert.match(
  dashboardCss,
  /\.service-browser-table-density-compact[\s\S]*\.service-browser-table-density-expanded/,
  'Browser table density modes must have explicit compact and expanded CSS hooks',
);

assert.match(
  validationSelector,
  /pnpm test:dashboard-browser-table/,
  'Validation selector must recommend the browser table smoke for Service dashboard wiring changes',
);

console.log('Dashboard browser table contract smoke passed');
