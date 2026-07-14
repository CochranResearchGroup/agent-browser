#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '..');
const check = process.argv.includes('--check');

const requestSchemaPath = path.join(repoRoot, 'docs/dev/contracts/service-request.v1.schema.json');
const mcpToolCallSchemaPath = path.join(
  repoRoot,
  'docs/dev/contracts/service-request-mcp-tool-call.v1.schema.json',
);
const generatedJsPath = path.join(repoRoot, 'packages/client/src/service-request.generated.js');
const generatedTypesPath = path.join(repoRoot, 'packages/client/src/service-request.generated.d.ts');

const requestSchema = JSON.parse(fs.readFileSync(requestSchemaPath, 'utf8'));
const mcpToolCallSchema = JSON.parse(fs.readFileSync(mcpToolCallSchemaPath, 'utf8'));

const actions = requestSchema.properties.action.enum;
const stringFields = Object.entries(requestSchema.properties)
  .filter(([, property]) => property.type === 'string')
  .map(([name]) => name);
const stringArrayFields = Object.entries(requestSchema.properties)
  .filter(([, property]) => property.type === 'array' && property.items?.type === 'string')
  .map(([name]) => name);
const integerFields = Object.entries(requestSchema.properties)
  .filter(([, property]) => property.type === 'integer')
  .map(([name]) => name);
const booleanFields = Object.entries(requestSchema.properties)
  .filter(([, property]) => property.type === 'boolean')
  .map(([name]) => name);
const objectFields = Object.entries(requestSchema.properties)
  .filter(
    ([name, property]) =>
      name !== 'params' &&
      (property.type === 'object' || typeof property.$ref === 'string'),
  )
  .map(([name]) => name);
const requiredFields = requestSchema.required;
const mcpToolName = mcpToolCallSchema.properties.name.const;

writeGenerated(generatedJsPath, renderGeneratedJs());
writeGenerated(generatedTypesPath, renderGeneratedTypes());

function writeGenerated(filePath, content) {
  if (check) {
    const existing = fs.existsSync(filePath) ? fs.readFileSync(filePath, 'utf8') : '';
    if (existing !== content) {
      console.error(`${path.relative(repoRoot, filePath)} is stale. Run pnpm generate:service-client.`);
      process.exitCode = 1;
    }
    return;
  }

  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, content);
}

function renderGeneratedJs() {
  return `${generatedHeader('js')}export const SERVICE_REQUEST_ACTIONS = ${json(actions)};

export const SERVICE_REQUEST_REQUIRED_FIELDS = ${json(requiredFields)};

export const SERVICE_REQUEST_STRING_FIELDS = ${json(stringFields)};

export const SERVICE_REQUEST_STRING_ARRAY_FIELDS = ${json(stringArrayFields)};

export const SERVICE_REQUEST_INTEGER_FIELDS = ${json(integerFields)};

export const SERVICE_REQUEST_BOOLEAN_FIELDS = ${json(booleanFields)};

export const SERVICE_REQUEST_OBJECT_FIELDS = ${json(objectFields)};

export const SERVICE_REQUEST_MCP_TOOL_NAME = ${json(mcpToolName)};
`;
}

function renderGeneratedTypes() {
  const actionUnion = actions.map((action) => `  | ${JSON.stringify(action)}`).join('\n');
  const stringFieldLines = stringFields
    .filter((field) => !requiredFields.includes(field))
    .map((field) => `  ${field}?: string;`)
    .join('\n');
  const stringArrayFieldLines = stringArrayFields.map((field) => `  ${field}?: string[];`).join('\n');
  const integerFieldLines = integerFields.map((field) => `  ${field}?: number;`).join('\n');
  const booleanFieldLines = booleanFields.map((field) => `  ${field}?: boolean;`).join('\n');
  const objectFieldTypeByName = new Map([['serviceTabHandle', 'ServiceTabHandle']]);
  const objectFieldLines = objectFields
    .map((field) => `  ${field}?: ${objectFieldTypeByName.get(field) ?? 'Record<string, unknown>'};`)
    .join('\n');

  return `${generatedHeader('ts')}export type ServiceRequestAction =
${actionUnion};

export interface ServiceRequest {
  action: ServiceRequestAction;
  params?: Record<string, unknown>;
${stringFieldLines}
${stringArrayFieldLines}
${integerFieldLines}
${booleanFieldLines}
${objectFieldLines}
}

export type ServiceRequestForAction<TAction extends ServiceRequestAction> =
  Omit<ServiceRequest, "action"> & { action: TAction };

export interface ServiceRequestMcpToolCall {
  name: ${JSON.stringify(mcpToolName)};
  arguments: ServiceRequest;
}

export interface ServiceRequestHttpOptions<TRequest extends ServiceRequest = ServiceRequest> {
  baseUrl: string;
  request: TRequest;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceNavigateData {
  url: string;
  title?: string;
}

export interface ServiceCdpFreeLaunchData {
  launched: true;
  cdpFree: true;
  cdpAttachmentAllowed: false;
  browserId: string;
  browserPid: number;
  profileId?: string | null;
  runtimeProfile?: string | null;
  userDataDir: string;
  url?: string | null;
  supportedOperations: string[];
  unsupportedOperations: string[];
  unsupportedCommands: ServiceRequestAction[];
}

export interface ServiceCdpFreeLaunchAvailability {
  controlPlaneMode: "cdp_free";
  lifecycleOnly: true;
  cdpAttachmentAllowed: boolean;
  supportedOperations: string[];
  unsupportedOperations: string[];
  unsupportedCommands: ServiceRequestAction[];
  availableCommands: ServiceRequestAction[];
  hasUnsupportedCommandList: boolean;
}

export interface ServiceSharedTabAcquisition {
  policy: "shared_browser_tabs" | string;
  mode: "tab_new" | string;
  action: "opened_new_tab" | "waited" | "rejected_duplicate_process" | string;
  browserReused: boolean;
  tabOpened: boolean;
  waitedForProfileLease: boolean;
  rejectedDuplicateProcess: boolean;
  duplicateProcessAllowed: boolean;
  browserId: string;
  sessionName: string;
  profileId?: string | null;
  requestedBrowserId?: string | null;
  requestedSessionName?: string | null;
  routeHintSource?: string;
  [key: string]: unknown;
}

export interface ServiceExternalByopAdoptData {
  ok: boolean;
  action: "external_byop_adopt";
  adopted: boolean;
  browserId: string;
  sessionName: string;
  profileId: string;
  profileOrigin: "external_byop" | string;
  browserHost: "attached_existing" | string;
  targetId?: string | null;
  url?: string | null;
  title?: string | null;
  tabNew?: ServiceTabNewData;
  serviceTabHandle: ServiceTabHandle;
  [key: string]: unknown;
}

export interface ServiceCdpAttachDescriptor {
  attached: true;
  controlPlaneMode: "cdp";
  attachKind: "service_tab_handle" | string;
  browserId: string;
  sessionName: string;
  tabId: string;
  targetId: string;
  pageSessionId: string;
  profileId?: string | null;
  profileOrigin?: 'agent_browser_owned' | 'external_byop' | 'external_observed' | string | null;
  leaseId?: string | null;
  leaseState?: 'shared' | 'exclusive' | 'human_takeover' | 'released' | 'expired' | string | null;
  cleanupPolicy?: 'detach' | 'close_tabs' | 'close_browser' | 'release_only' | string | null;
  browserWebSocketUrl: string;
  cdpAttachmentAllowed: true;
  detachAction: "cdp_detach";
  detachRequired: boolean;
  closeBrowserOnDetach: false;
  browserProcessPreserved: true;
  traceFilter: ServiceTabHandleTraceFilter;
  serviceTabHandle: ServiceTabHandle;
  attachedAt?: string;
  [key: string]: unknown;
}

export interface ServiceCdpDetachData {
  detached: true;
  controlPlaneMode: "cdp";
  detachKind: "service_tab_handle" | string;
  browserId: string;
  sessionName: string;
  tabId?: string | null;
  targetId?: string | null;
  profileId?: string | null;
  browserProcessPreserved: true;
  closeBrowserOnDetach: false;
  serviceTabHandle: ServiceTabHandle;
  detachedAt?: string;
  [key: string]: unknown;
}

export interface ServiceEvaluateData {
  ok: boolean;
  action: "evaluate";
  result?: unknown;
  resultTruncated?: boolean;
  resultBytes?: number;
  maxReturnBytes?: number;
  timeoutMs?: number;
  returnByValue?: boolean;
  url?: string;
  title?: string;
  targetId?: string;
  tabId?: string | null;
  profileId?: string | null;
  serviceTabHandle?: ServiceTabHandle;
  evaluatedAt?: string;
  [key: string]: unknown;
}

export interface ServiceProbeDetectorResult {
  id?: string;
  type?: "evaluate" | "url_title" | "selector_text" | "client_evidence" | string;
  ok: boolean;
  result?: unknown;
  evidence?: unknown;
  error?: string;
  resultTruncated?: boolean;
  resultBytes?: number;
  maxReturnBytes?: number;
  [key: string]: unknown;
}

export interface ServiceProbeIdentity {
  detectedIdentity?: string | null;
  detectedAccountId?: string | null;
  expectedIdentity?: string | null;
  confidence: "none" | "low" | "medium" | "high" | string;
  source?: string | null;
  [key: string]: unknown;
}

export interface ServiceProbeData {
  ok: boolean;
  action: "probe";
  observedAt: string;
  url?: string | null;
  title?: string | null;
  targetId?: string | null;
  tabId?: string | null;
  profileId?: string | null;
  serviceTabHandle: ServiceTabHandle;
  probe: Record<string, unknown>;
  identity: ServiceProbeIdentity;
  detectors: ServiceProbeDetectorResult[];
  freshness?: Record<string, unknown> | null;
  [key: string]: unknown;
}

export interface ServiceTabHandleRefreshData {
  ok: boolean;
  action: "tab_handle_refresh";
  refreshed: boolean;
  decision: string;
  repairPolicy: "reject_only" | "reuse_compatible" | "open_if_missing" | "replace_duplicates" | string;
  observedAt: string;
  browserId: string;
  targetId?: string | null;
  url?: string | null;
  title?: string | null;
  serviceTabHandle?: ServiceTabHandle | null;
  candidates: Record<string, unknown>[];
  [key: string]: unknown;
}

export interface ServiceTabHandleReleaseData {
  ok: boolean;
  action: "tab_handle_release";
  released: boolean;
  tabReleased: boolean;
  tabMissing: boolean;
  browserProcessPreserved: true;
  sessionRoutePreserved: true;
  closeBrowserOnRelease: false;
  physicalTabClose?: Record<string, unknown> | null;
  physicalTabCloseAttempted?: boolean;
  physicalTabClosed?: boolean;
  physicalTabCloseSkippedReason?: string | null;
  browserId: string;
  sessionName: string;
  tabId: string;
  targetId?: string | null;
  cleanupPolicy?: string | null;
  beforeLifecycle?: string | null;
  afterLifecycle?: string | null;
  serviceTabHandle?: ServiceTabHandle | null;
  releasedAt?: string;
  [key: string]: unknown;
}

export interface ServiceUiActionStepResult {
  index: number;
  type: string;
  id?: string | null;
  ok: boolean;
  selector?: string | null;
  result?: unknown;
  page?: Record<string, unknown>;
  error?: string;
  [key: string]: unknown;
}

export interface ServiceUiActionData {
  ok: boolean;
  action: "ui_action";
  observedAt: string;
  targetId?: string | null;
  tabId?: string | null;
  profileId?: string | null;
  serviceTabHandle: ServiceTabHandle;
  traceFilter?: ServiceTabHandleTraceFilter | null;
  uiAction: Record<string, unknown>;
  before?: Record<string, unknown>;
  after?: Record<string, unknown>;
  steps: ServiceUiActionStepResult[];
  failedStepIndex?: number;
  caller?: Record<string, unknown>;
  [key: string]: unknown;
}

export interface ServiceNetworkCaptureBody {
  captured: boolean;
  base64Encoded?: boolean;
  body?: string;
  bodyBase64?: string;
  bodyBytes?: number;
  bodyTruncated?: boolean;
  maxBodyBytes?: number | null;
  error?: string;
  [key: string]: unknown;
}

export interface ServiceNetworkCaptureEvent {
  requestId: string;
  url?: string | null;
  method?: string | null;
  resourceType?: string | null;
  status?: number | null;
  statusText?: string | null;
  mimeType?: string | null;
  encodedDataLength?: number | null;
  headersRedacted?: boolean;
  requestHeaders?: Record<string, unknown>;
  responseHeaders?: Record<string, unknown>;
  body?: ServiceNetworkCaptureBody;
  [key: string]: unknown;
}

export interface ServiceNetworkCaptureData {
  ok: boolean;
  action: "network_capture";
  observedAt: string;
  timedOut: boolean;
  targetId?: string | null;
  tabId?: string | null;
  profileId?: string | null;
  serviceTabHandle: ServiceTabHandle;
  traceFilter?: ServiceTabHandleTraceFilter | null;
  networkCapture: Record<string, unknown>;
  before?: Record<string, unknown>;
  after?: Record<string, unknown>;
  events: ServiceNetworkCaptureEvent[];
  caller?: Record<string, unknown>;
  [key: string]: unknown;
}

export interface ServiceFileTransferData {
  ok: boolean;
  action: "file_transfer";
  observedAt: string;
  targetId?: string | null;
  tabId?: string | null;
  profileId?: string | null;
  serviceTabHandle: ServiceTabHandle;
  traceFilter?: ServiceTabHandleTraceFilter | null;
  fileTransfer: Record<string, unknown>;
  before?: Record<string, unknown>;
  after?: Record<string, unknown>;
  upload?: Record<string, unknown> | null;
  download?: Record<string, unknown> | null;
  failedPhase?: string;
  error?: string;
  diagnostics?: Record<string, unknown> | null;
  caller?: Record<string, unknown>;
  [key: string]: unknown;
}

export interface ServiceDiagnosticsData {
  ok: boolean;
  action: "diagnostics";
  observedAt: string;
  compact: boolean;
  browserId: string;
  sessionName: string;
  tabId: string;
  targetId?: string | null;
  activeSessionId?: string | null;
  profileId?: string | null;
  profileOrigin?: string | null;
  url?: string | null;
  title?: string | null;
  serviceTabHandle: ServiceTabHandle;
  traceFilter?: ServiceTabHandleTraceFilter | null;
  browser?: Record<string, unknown> | null;
  session?: Record<string, unknown> | null;
  tab?: Record<string, unknown> | null;
  profile?: Record<string, unknown> | null;
  remoteViewRoutes: Record<string, unknown>[];
  snapshotSummary: Record<string, unknown>;
  screenshot: Record<string, unknown>;
  console: Record<string, unknown>;
  errors: Record<string, unknown>;
  requests: Record<string, unknown>;
  caller: Record<string, unknown>;
  [key: string]: unknown;
}

export interface ServiceTabHandleTraceFilter {
  browserId?: string | null;
  profileId?: string | null;
  sessionId?: string | null;
  serviceName?: string | null;
  agentName?: string | null;
  taskName?: string | null;
}

export interface ServiceTabHandle {
  browserId: string;
  sessionName?: string | null;
  tabId: string;
  targetId?: string | null;
  url?: string | null;
  title?: string | null;
  profileId?: string | null;
  profileOrigin: 'agent_browser_owned' | 'external_byop' | 'external_observed' | string;
  leaseId?: string | null;
  leaseState?: 'shared' | 'exclusive' | 'human_takeover' | 'released' | 'expired' | string | null;
  cleanupPolicy?: 'detach' | 'close_tabs' | 'close_browser' | 'release_only' | string | null;
  leaseHeartbeatExpected: boolean;
  ownerSessionId?: string | null;
  jobId?: string | null;
  traceFilter: ServiceTabHandleTraceFilter;
  valid: boolean;
  staleReason?: string | null;
}

export interface ServiceTabNewData {
  index: number;
  url: string;
  targetId?: string;
  pageSessionId?: string;
  browserId?: string;
  sessionId?: string;
  runtimeProfile?: string;
  profileId?: string;
  sharedAcquisition?: ServiceSharedTabAcquisition;
  serviceTabHandle?: ServiceTabHandle;
}

export interface ServiceTabSwitchData {
  index: number;
  url: string;
  title: string;
}

export interface ServiceTabCloseData {
  closed: number;
  activeIndex: number;
}

export interface ServiceViewFocusData {
  broughtToFront: boolean;
  maximizeRequested: boolean;
  maximized: boolean;
  windowId?: number;
  maximizeError?: string;
  tabSwitch?: ServiceTabSwitchData;
}

export interface ServiceViewTakeoverData {
  takeoverRequested: boolean;
  reconnectRequested: boolean;
  browserProcessPreserved: boolean;
  browserId: string;
  sessionName: string;
  streamId?: string | null;
  provider?: string | null;
  openMode?: string;
  reason?: string;
  targetId?: string | null;
  index?: number | null;
  requestedAt?: string;
}

export interface ServiceRemoteViewRouteMutationData {
  status: string;
  routeId?: string;
  remoteViewRouteId?: string;
  displayAllocationId?: string;
  routePoolEntryId?: string | null;
  previousRouteId?: string | null;
  previousRoutePoolEntryId?: string | null;
  newRouteId?: string | null;
  newRoutePoolEntryId?: string | null;
  browserId?: string;
  sessionName?: string;
  frameUrl?: string | null;
  externalUrl?: string | null;
  providerMode?: string;
  remoteViewRoute?: Record<string, unknown> | null;
  routePoolEntry?: Record<string, unknown> | null;
  reattachRepair?: Record<string, unknown> | null;
  routeSwitchRelease?: Record<string, unknown> | null;
  routeSwitchParking?: Record<string, unknown> | null;
  checkout?: Record<string, unknown> | null;
  releasedViewerLeaseIds?: string[];
  updatedAt?: string;
  [key: string]: unknown;
}

export interface ServiceRemoteViewOpenProofSummary {
  ready: boolean;
  state: string | null;
  routeId: string | null;
  displayAllocationId: string | null;
  displayName: string | null;
  browserId: string | null;
  sessionName: string | null;
  tabId: string | null;
  profileId: string | null;
  visualProof: string | null;
  browserBuildState: string | null;
  requestedBrowserBuild: string | null;
  selectedBrowserBuild: string | null;
  actualExecutablePath: string | null;
  browserBuildMismatchReason: string | null;
  failureReason: string | null;
  summary: string;
}

export interface ServiceSharedProfileAcquisitionSummary {
  available: boolean;
  recommendedAction: string | null;
  acquisitionMode: string | null;
  requestedProfile: string | null;
  plannedProfile: string | null;
  runtimeProfile: string | null;
  profileId: string | null;
  browserId: string | null;
  sessionName: string | null;
  tabId: string | null;
  targetId: string | null;
  browserReused: boolean | null;
  tabOpened: boolean | null;
  profileProcessPolicy: string | null;
  clientSharingPolicy: string | null;
  duplicateProcessPolicy: string | null;
  requiresRouteHints: boolean;
  routeHintFields: string[];
  serviceTabHandle: ServiceTabHandle | null;
  summary: string;
}

export interface ServiceRoutePoolRepairData {
  repaired: boolean;
  dryRun: boolean;
  observedAt?: string;
  policy?: Record<string, unknown>;
  before?: Record<string, number>;
  after?: Record<string, number>;
  candidates?: Record<string, string[]>;
  candidateReasons?: Record<string, Record<string, unknown>>;
  candidateCounts: Record<string, number>;
  skipped?: Record<string, string[]>;
  skippedCounts?: Record<string, number>;
  repairedCounts: Record<string, number>;
  recommendedNextStep?: string;
}

export interface ServiceViewerLeaseMutationData {
  status: string;
  routeId?: string | null;
  remoteViewRouteId?: string;
  viewerLeaseId?: string;
  controllerLeaseId?: string | null;
  previousControllerLeaseId?: string | null;
  viewerLease?: Record<string, unknown> | null;
  remoteViewRoute?: Record<string, unknown> | null;
  updatedAt?: string;
  [key: string]: unknown;
}

export interface ServiceTabRecord {
  index: number;
  title: string;
  url: string;
  type: string;
  active: boolean;
  targetId?: string;
  sessionId?: string;
  serviceTabHandle?: ServiceTabHandle;
  [key: string]: unknown;
}

export interface ServiceTabListData {
  tabs: ServiceTabRecord[];
}

export interface ServiceUrlData {
  url: string;
}

export interface ServiceTitleData {
  title: string;
}

export interface ServiceSnapshotRef {
  role: string;
  name: string;
  [key: string]: unknown;
}

export interface ServiceSnapshotData {
  snapshot: string;
  origin: string;
  refs: Record<string, ServiceSnapshotRef>;
}

export interface ServiceScreenshotAnnotationBox {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface ServiceScreenshotAnnotation {
  ref: string;
  number: number;
  role: string;
  name?: string;
  box: ServiceScreenshotAnnotationBox;
}

export interface ServiceScreenshotData {
  path: string;
  annotations?: ServiceScreenshotAnnotation[];
}

export interface ServiceTextData {
  text: string;
  origin?: string;
}

export interface ServiceValueData {
  value: unknown;
  origin?: string;
}

export interface ServiceHtmlData {
  html: string;
}

export interface ServiceVisibilityData {
  visible: boolean;
  origin: string;
}

export interface ServiceEnabledData {
  enabled: boolean;
  origin: string;
}

export interface ServiceCheckedData {
  checked: boolean;
  origin: string;
}

export interface ServiceCountData {
  count: number;
  selector: string;
}

export interface ServiceBoundingBoxData {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface ServiceStylesData {
  styles: Record<string, string>;
}

export interface ServiceClickData {
  clicked: string;
  newTab?: boolean;
  url?: string;
  fallbackNavigation?: boolean;
  deferredActivation?: boolean;
}

export interface ServiceFillData {
  filled: string;
}

export interface ServiceTypeData {
  typed: string;
}

export interface ServicePressData {
  pressed: string;
}

export interface ServiceHoverData {
  hovered: string;
}

export interface ServiceScrollData {
  scrolled: true;
}

export interface ServiceSelectData {
  selected: string[];
}

export interface ServiceCheckData {
  checked: string;
}

export interface ServiceUncheckData {
  unchecked: string;
}

export type ServiceWaitData =
  | { waited: "text"; text: string }
  | { waited: "selector"; selector: string }
  | { waited: "url"; url: string }
  | { waited: "function" }
  | { waited: "load"; state: string }
  | { waited: "timeout"; ms: number };

export interface ServiceFocusData {
  focused: string;
}

export interface ServiceClearData {
  cleared: string;
}

export interface ServiceScrollIntoViewData {
  scrolled: string;
}

export interface ServiceSetData {
  set: true;
}

export interface ServiceOfflineData {
  offline: boolean;
}

export interface ServiceViewportData {
  width: number;
  height: number;
  deviceScaleFactor: number;
  mobile: boolean;
}

export interface ServiceUserAgentData {
  userAgent: string;
}

export interface ServiceTimezoneData {
  timezoneId: string;
}

export interface ServiceLocaleData {
  locale: string;
}

export interface ServiceGeolocationData {
  latitude: number;
  longitude: number;
}

export interface ServicePermissionsData {
  granted: string[];
}

export type ServiceDialogData =
  | { hasDialog: true; type: string; message: string; defaultPrompt?: string }
  | { hasDialog: false }
  | { handled: true; accepted: boolean };

export type ServiceClipboardData =
  | { text: unknown }
  | { written: string }
  | { copied: true }
  | { pasted: true };

export interface ServiceUploadData {
  uploaded: number;
  selector: string;
}

export interface ServiceCookieRecord {
  name: string;
  value: string;
  domain: string;
  path: string;
  expires?: number;
  size?: number;
  httpOnly?: boolean;
  secure?: boolean;
  session?: boolean;
  sameSite?: string;
  [key: string]: unknown;
}

export interface ServiceCookiesData {
  cookies: ServiceCookieRecord[];
}

export interface ServiceClearedData {
  cleared: true;
}

export type ServiceStorageGetData =
  | { key: string; value: unknown }
  | { data: Record<string, unknown> };

export interface ServiceConsoleMessage {
  type: string;
  text: string;
  args?: unknown[];
  [key: string]: unknown;
}

export interface ServiceConsoleData {
  messages: ServiceConsoleMessage[];
}

export interface ServiceErrorEntry {
  text: string;
  url?: string | null;
  line?: number | null;
  column?: number | null;
  [key: string]: unknown;
}

export interface ServiceErrorsData {
  errors: ServiceErrorEntry[];
}

export interface ServicePathData {
  path: string;
}

export interface ServiceResponseBodyData {
  body: string;
  status: number;
  headers: Record<string, unknown>;
}

export interface ServiceHarStartData {
  started: true;
}

export interface ServiceHarStopData {
  path: string;
  requestCount: number;
}

export interface ServiceRouteData {
  routed: string;
}

export interface ServiceUnrouteData {
  unrouted: string;
}

export interface ServiceTrackedRequest {
  url: string;
  method: string;
  headers: Record<string, unknown>;
  timestamp: number;
  resourceType: string;
  requestId: string;
  postData?: string;
  status?: number;
  responseHeaders?: Record<string, unknown>;
  mimeType?: string;
  responseBody?: string;
  [key: string]: unknown;
}

export interface ServiceRequestsData {
  requests: ServiceTrackedRequest[];
}

export interface ServiceRetainedCleanupData {
  pruned?: boolean;
  repaired?: boolean;
  dryRun: boolean;
  observedAt?: string;
  policy?: Record<string, unknown>;
  before?: Record<string, number>;
  after?: Record<string, number>;
  candidates?: Record<string, string[]>;
  candidateCounts: Record<string, number>;
  candidateReasons?: Record<string, unknown>;
  candidateClassCounts?: Record<string, Record<string, number>>;
  skipped?: Record<string, string[]>;
  skippedCounts?: Record<string, number>;
  skippedSummary?: Record<string, unknown>;
  removed?: Record<string, number>;
  repairedCounts?: Record<string, number>;
  recommendedNextStep?: string;
}

export interface ServiceBrowserCloseData {
  closed: boolean;
  browserId: string;
  requestedBrowserId: string;
  serviceOwned: boolean;
  [key: string]: unknown;
}

export interface ServiceBrowserRepairData {
  repaired: boolean;
  browser: Record<string, unknown>;
  incident?: Record<string, unknown> | null;
}

export interface ServiceRequestActionDataMap {
  navigate: ServiceNavigateData;
  cdp_free_launch: ServiceCdpFreeLaunchData;
  external_byop_adopt: ServiceExternalByopAdoptData;
  cdp_attach: ServiceCdpAttachDescriptor;
  cdp_detach: ServiceCdpDetachData;
  evaluate: ServiceEvaluateData;
  probe: ServiceProbeData;
  tab_handle_refresh: ServiceTabHandleRefreshData;
  tab_handle_release: ServiceTabHandleReleaseData;
  ui_action: ServiceUiActionData;
  network_capture: ServiceNetworkCaptureData;
  file_transfer: ServiceFileTransferData;
  diagnostics: ServiceDiagnosticsData;
  back: ServiceUrlData;
  forward: ServiceUrlData;
  reload: ServiceUrlData;
  tab_new: ServiceTabNewData;
  tab_switch: ServiceTabSwitchData;
  tab_close: ServiceTabCloseData;
  view_focus: ServiceViewFocusData;
  view_takeover: ServiceViewTakeoverData;
  remote_view_open: ServiceRemoteViewRouteMutationData;
  service_remote_view_route_preflight: ServiceRemoteViewRouteMutationData;
  service_remote_view_browser_reattach: ServiceRemoteViewRouteMutationData;
  service_remote_view_route_switch: ServiceRemoteViewRouteMutationData;
  service_remote_view_route_checkout: ServiceRemoteViewRouteMutationData;
  service_remote_view_route_release: ServiceRemoteViewRouteMutationData;
  service_route_pool_repair: ServiceRoutePoolRepairData;
  service_viewer_lease_request: ServiceViewerLeaseMutationData;
  service_viewer_lease_heartbeat: ServiceViewerLeaseMutationData;
  service_viewer_lease_release: ServiceViewerLeaseMutationData;
  service_controller_lease_takeover: ServiceViewerLeaseMutationData;
  tab_list: ServiceTabListData;
  url: ServiceUrlData;
  title: ServiceTitleData;
  snapshot: ServiceSnapshotData;
  screenshot: ServiceScreenshotData;
  gettext: ServiceTextData;
  inputvalue: ServiceValueData;
  isvisible: ServiceVisibilityData;
  getattribute: ServiceValueData;
  innerhtml: ServiceHtmlData;
  styles: ServiceStylesData;
  count: ServiceCountData;
  boundingbox: ServiceBoundingBoxData;
  isenabled: ServiceEnabledData;
  ischecked: ServiceCheckedData;
  click: ServiceClickData;
  fill: ServiceFillData;
  type: ServiceTypeData;
  press: ServicePressData;
  hover: ServiceHoverData;
  scroll: ServiceScrollData;
  select: ServiceSelectData;
  check: ServiceCheckData;
  uncheck: ServiceUncheckData;
  wait: ServiceWaitData;
  focus: ServiceFocusData;
  clear: ServiceClearData;
  scrollintoview: ServiceScrollIntoViewData;
  setcontent: ServiceSetData;
  headers: ServiceSetData;
  offline: ServiceOfflineData;
  viewport: ServiceViewportData;
  user_agent: ServiceUserAgentData;
  timezone: ServiceTimezoneData;
  locale: ServiceLocaleData;
  geolocation: ServiceGeolocationData;
  permissions: ServicePermissionsData;
  emulatemedia: ServiceSetData;
  dialog: ServiceDialogData;
  clipboard: ServiceClipboardData;
  upload: ServiceUploadData;
  cookies_get: ServiceCookiesData;
  cookies_set: ServiceSetData;
  cookies_clear: ServiceClearedData;
  storage_get: ServiceStorageGetData;
  storage_set: ServiceSetData;
  storage_clear: ServiceClearedData;
  console: ServiceConsoleData | ServiceClearedData;
  errors: ServiceErrorsData;
  download: ServicePathData;
  waitfordownload: ServicePathData;
  pdf: ServicePathData;
  responsebody: ServiceResponseBodyData;
  har_start: ServiceHarStartData;
  har_stop: ServiceHarStopData;
  route: ServiceRouteData;
  unroute: ServiceUnrouteData;
  requests: ServiceRequestsData | ServiceClearedData;
  request_detail: ServiceTrackedRequest;
  service_browser_close: ServiceBrowserCloseData;
  service_browser_repair: ServiceBrowserRepairData;
  service_prune_retained: ServiceRetainedCleanupData;
  service_repair_retained: ServiceRetainedCleanupData;
}

export type ServiceRequestDataForAction<TAction extends ServiceRequestAction> =
  TAction extends keyof ServiceRequestActionDataMap ? ServiceRequestActionDataMap[TAction] : unknown;

export interface ServiceRequestResponse<TData = unknown> {
  id?: string;
  success: boolean;
  data?: TData;
  error?: unknown;
  warning?: unknown;
  [key: string]: unknown;
}

export interface ServiceTabAccessPlan {
  readinessSummary?: {
    manualSeedingRequired?: boolean;
  };
  seedingHandoff?: {
    command?: string;
  } | null;
  decision: {
    manualSeedingRequired?: boolean;
    serviceRequest: {
      request: ServiceRequestForAction<"tab_new">;
      [key: string]: unknown;
    };
    profileReuse?: {
      sharedAcquisition?: {
        mode?: "tab_new" | string | null;
        browserId?: string | null;
        sessionName?: string | null;
        [key: string]: unknown;
      };
      [key: string]: unknown;
    };
    [key: string]: unknown;
  };
  [key: string]: unknown;
}

export interface ServiceMonitorRunDueSummary {
  targetServiceIds?: string[];
  matched?: number;
  monitorIds?: string[];
  resultStates?: string[];
  staleProfileIds?: string[];
  freshTargetServiceIds?: string[];
  expiredTargetServiceIds?: string[];
  unverifiedTargetServiceIds?: string[];
  succeeded?: boolean;
  failed?: boolean;
  recommendedAction?: string;
  matchingResults?: unknown[];
  [key: string]: unknown;
}

export interface ServiceTabRequestOptions extends Omit<ServiceRequest, "action" | "params"> {
  accessPlan?: ServiceTabAccessPlan;
  allowManualAction?: boolean;
  allowMonitorFreshnessRisk?: boolean;
  monitorRunDueSummary?: ServiceMonitorRunDueSummary;
  url?: string;
  params?: Record<string, unknown>;
}

export interface ServiceTabRequestHttpOptions extends ServiceTabRequestOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceCdpFreeLaunchRequestOptions extends Omit<ServiceRequest, "action" | "params"> {
  accessPlan?: ServiceTabAccessPlan;
  allowManualAction?: boolean;
  allowMonitorFreshnessRisk?: boolean;
  monitorRunDueSummary?: ServiceMonitorRunDueSummary;
  url?: string;
  params?: Record<string, unknown>;
}

export interface ServiceCdpFreeLaunchRequestHttpOptions extends ServiceCdpFreeLaunchRequestOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceExternalByopAdoptRequestOptions extends Omit<ServiceRequest, "action" | "params"> {
  profileId?: string;
  runtimeProfile?: string;
  cdpUrl?: string;
  cdpPort?: number;
  url?: string;
  params?: Record<string, unknown>;
}

export interface ServiceExternalByopAdoptRequestHttpOptions extends ServiceExternalByopAdoptRequestOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceCdpAttachRequestOptions extends Omit<ServiceRequest, "action" | "params"> {
  serviceTabHandle: ServiceTabHandle;
  params?: Record<string, unknown>;
}

export interface ServiceCdpAttachRequestHttpOptions extends ServiceCdpAttachRequestOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceCdpDetachRequestOptions extends Omit<ServiceRequest, "action" | "params"> {
  serviceTabHandle: ServiceTabHandle;
  params?: Record<string, unknown>;
}

export interface ServiceCdpDetachRequestHttpOptions extends ServiceCdpDetachRequestOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceEvaluateRequestOptions extends Omit<ServiceRequest, "action" | "params"> {
  serviceTabHandle: ServiceTabHandle;
  script?: string;
  expression?: string;
  returnByValue?: boolean;
  timeoutMs: number;
  maxReturnBytes: number;
  captureEvidenceOnFailure?: boolean;
  params?: Record<string, unknown>;
}

export interface ServiceEvaluateRequestHttpOptions extends ServiceEvaluateRequestOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceDiagnosticsRequestOptions extends Omit<ServiceRequest, "action" | "params"> {
  serviceTabHandle: ServiceTabHandle;
  includeScreenshot?: boolean;
  screenshotDir?: string;
  maxConsoleEntries?: number;
  maxErrorEntries?: number;
  maxRequestEntries?: number;
  params?: Record<string, unknown>;
}

export interface ServiceDiagnosticsRequestHttpOptions extends ServiceDiagnosticsRequestOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceProbeRequestOptions extends Omit<ServiceRequest, "action" | "params"> {
  serviceTabHandle: ServiceTabHandle;
  probe: Record<string, unknown>;
  timeoutMs: number;
  maxReturnBytes: number;
  params?: Record<string, unknown>;
}

export interface ServiceProbeRequestHttpOptions extends ServiceProbeRequestOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceUiActionRequestOptions extends Omit<ServiceRequest, "action" | "params"> {
  serviceTabHandle: ServiceTabHandle;
  uiAction: Record<string, unknown>;
  timeoutMs: number;
  maxTextBytes?: number;
  params?: Record<string, unknown>;
}

export interface ServiceUiActionRequestHttpOptions extends ServiceUiActionRequestOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceNetworkCaptureRequestOptions extends Omit<ServiceRequest, "action" | "params"> {
  serviceTabHandle: ServiceTabHandle;
  networkCapture: Record<string, unknown>;
  timeoutMs: number;
  maxBodyBytes?: number;
  params?: Record<string, unknown>;
}

export interface ServiceNetworkCaptureRequestHttpOptions extends ServiceNetworkCaptureRequestOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceFileTransferRequestOptions extends Omit<ServiceRequest, "action" | "params"> {
  serviceTabHandle: ServiceTabHandle;
  fileTransfer: Record<string, unknown>;
  timeoutMs: number;
  params?: Record<string, unknown>;
}

export interface ServiceFileTransferRequestHttpOptions extends ServiceFileTransferRequestOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceTabHandleRefreshOptions extends Omit<ServiceRequest, "action" | "params"> {
  serviceTabHandle: ServiceTabHandle;
  repairPolicy?: "reject_only" | "reuse_compatible" | "open_if_missing" | "replace_duplicates";
  url?: string;
  desiredUrl?: string;
  params?: Record<string, unknown>;
}

export interface ServiceTabHandleRefreshHttpOptions extends ServiceTabHandleRefreshOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceTabHandleReleaseOptions extends Omit<ServiceRequest, "action" | "params"> {
  serviceTabHandle: ServiceTabHandle;
  params?: Record<string, unknown>;
}

export interface ServiceTabHandleReleaseHttpOptions extends ServiceTabHandleReleaseOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceRemoteViewRouteCheckoutOptions extends Omit<ServiceRequest, "action" | "params"> {
  displayAllocationId: string;
  routeId?: string;
  remoteViewRouteId?: string;
  routePoolEntryId?: string;
  routePoolEntry?: Record<string, unknown>;
  routePool?: Record<string, unknown>[];
  browserId?: string;
  sessionName?: string;
  streamId?: string;
  viewStreamProvider?: string;
  provider?: string;
  providerMode?: string;
  frameUrl?: string;
  externalUrl?: string;
  connectionId?: string;
  connectionName?: string;
  routeDescriptor?: Record<string, unknown>;
  remoteHeadedDisplay?: string;
  display?: string;
  displayName?: string;
  url?: string;
  dryRun?: boolean;
  allowInfrastructureOnlyReadiness?: boolean;
  params?: Record<string, unknown>;
}

export interface ServiceRemoteViewBrowserReattachOptions extends Omit<ServiceRequest, "action" | "params"> {
  browserId?: string;
  profileId?: string;
  sessionName?: string;
  displayAllocationId?: string;
  routeId?: string;
  remoteViewRouteId?: string;
  routePoolEntryId?: string;
  routePoolEntry?: Record<string, unknown>;
  routePool?: Record<string, unknown>[];
  streamId?: string;
  viewStreamProvider?: string;
  provider?: string;
  providerMode?: string;
  frameUrl?: string;
  externalUrl?: string;
  connectionId?: string;
  connectionName?: string;
  routeDescriptor?: Record<string, unknown>;
  openMode?: "embedded" | "external" | "fullscreen" | "tile" | string;
  viewerId?: string;
  viewerName?: string;
  viewerRole?: "observer" | "controller" | "pending_controller" | "none" | string;
  controllerTakeover?: boolean;
  params?: Record<string, unknown>;
}

export interface ServiceRemoteViewRouteReleaseOptions extends Omit<ServiceRequest, "action" | "params"> {
  routeId: string;
  params?: Record<string, unknown>;
}

export interface ServiceRoutePoolRepairOptions extends Omit<ServiceRequest, "action" | "params"> {
  apply?: boolean;
  staleCheckouts?: boolean;
  serviceState?: Record<string, unknown>;
  params?: Record<string, unknown>;
}

export interface ServiceViewerLeaseRequestOptions extends Omit<ServiceRequest, "action" | "params"> {
  routeId: string;
  viewerId?: string;
  viewerName?: string;
  viewerRole?: "observer" | "controller" | "pending_controller" | "none" | string;
  openMode?: "embedded" | "external" | "fullscreen" | "tile" | string;
  browserId?: string;
  expiresAt?: string;
  params?: Record<string, unknown>;
}

export interface ServiceViewerLeaseReleaseOptions extends Omit<ServiceRequest, "action" | "params"> {
  viewerLeaseId: string;
  params?: Record<string, unknown>;
}

export interface ServiceViewerLeaseHeartbeatOptions extends Omit<ServiceRequest, "action" | "params"> {
  viewerLeaseId: string;
  expiresAt?: string;
  params?: Record<string, unknown>;
}

export interface ServiceControllerLeaseTakeoverOptions extends ServiceViewerLeaseRequestOptions {
  viewerLeaseId?: string;
}

export interface ServiceRemoteViewRouteCheckoutHttpOptions extends ServiceRemoteViewRouteCheckoutOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceRemoteViewBrowserReattachHttpOptions extends ServiceRemoteViewBrowserReattachOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceRemoteViewRouteReleaseHttpOptions extends ServiceRemoteViewRouteReleaseOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceRoutePoolRepairHttpOptions extends ServiceRoutePoolRepairOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceViewerLeaseRequestHttpOptions extends ServiceViewerLeaseRequestOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceViewerLeaseReleaseHttpOptions extends ServiceViewerLeaseReleaseOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceViewerLeaseHeartbeatHttpOptions extends ServiceViewerLeaseHeartbeatOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceControllerLeaseTakeoverHttpOptions extends ServiceControllerLeaseTakeoverOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export declare const SERVICE_REQUEST_ACTIONS: readonly ServiceRequestAction[];
export declare const SERVICE_REQUEST_REQUIRED_FIELDS: readonly string[];
export declare const SERVICE_REQUEST_STRING_FIELDS: readonly string[];
export declare const SERVICE_REQUEST_STRING_ARRAY_FIELDS: readonly string[];
export declare const SERVICE_REQUEST_INTEGER_FIELDS: readonly string[];
export declare const SERVICE_REQUEST_BOOLEAN_FIELDS: readonly string[];
export declare const SERVICE_REQUEST_OBJECT_FIELDS: readonly string[];
export declare const SERVICE_REQUEST_MCP_TOOL_NAME: ${JSON.stringify(mcpToolName)};

export declare function createServiceRequest<TRequest extends ServiceRequest>(input: TRequest): TRequest;
export declare function createServiceRequestMcpToolCall(input: ServiceRequest): ServiceRequestMcpToolCall;
export declare function postServiceRequest<TRequest extends ServiceRequest>(
  options: ServiceRequestHttpOptions<TRequest>,
): Promise<ServiceRequestResponse<ServiceRequestDataForAction<TRequest["action"]>>>;
export declare function createServiceTabRequest(input: ServiceTabRequestOptions): ServiceRequestForAction<"tab_new">;
export declare function createServiceTabRequestFromAccessPlan(
  accessPlan: ServiceTabAccessPlan,
  input?: Omit<ServiceTabRequestOptions, "accessPlan">,
): ServiceRequestForAction<"tab_new">;
export declare function getServiceTabHandle(response: unknown): ServiceTabHandle | null;
export declare function requireServiceTabHandle(response: unknown): ServiceTabHandle;
export declare function createServiceCdpFreeLaunchRequest(
  input: ServiceCdpFreeLaunchRequestOptions,
): ServiceRequestForAction<"cdp_free_launch">;
export declare function createServiceExternalByopAdoptRequest(
  input: ServiceExternalByopAdoptRequestOptions,
): ServiceRequestForAction<"external_byop_adopt">;
export declare function createServiceCdpAttachRequest(
  input: ServiceCdpAttachRequestOptions,
): ServiceRequestForAction<"cdp_attach">;
export declare function createServiceCdpDetachRequest(
  input: ServiceCdpDetachRequestOptions,
): ServiceRequestForAction<"cdp_detach">;
export declare function createServiceEvaluateRequest(
  input: ServiceEvaluateRequestOptions,
): ServiceRequestForAction<"evaluate">;
export declare function createServiceDiagnosticsRequest(
  input: ServiceDiagnosticsRequestOptions,
): ServiceRequestForAction<"diagnostics">;
export declare function createServiceProbeRequest(
  input: ServiceProbeRequestOptions,
): ServiceRequestForAction<"probe">;
export declare function createServiceUiActionRequest(
  input: ServiceUiActionRequestOptions,
): ServiceRequestForAction<"ui_action">;
export declare function createServiceNetworkCaptureRequest(
  input: ServiceNetworkCaptureRequestOptions,
): ServiceRequestForAction<"network_capture">;
export declare function createServiceFileTransferRequest(
  input: ServiceFileTransferRequestOptions,
): ServiceRequestForAction<"file_transfer">;
export declare function createServiceTabHandleRefreshRequest(
  input: ServiceTabHandleRefreshOptions,
): ServiceRequestForAction<"tab_handle_refresh">;
export declare function createServiceTabHandleReleaseRequest(
  input: ServiceTabHandleReleaseOptions,
): ServiceRequestForAction<"tab_handle_release">;
export declare function createServiceRemoteViewRoutePreflightRequest(
  input: ServiceRemoteViewRouteCheckoutOptions,
): ServiceRequestForAction<"service_remote_view_route_preflight">;
export declare function createServiceRemoteViewOpenRequest(
  input: ServiceRemoteViewRouteCheckoutOptions,
): ServiceRequestForAction<"remote_view_open">;
export declare function createServiceRemoteViewBrowserReattachRequest(
  input: ServiceRemoteViewBrowserReattachOptions,
): ServiceRequestForAction<"service_remote_view_browser_reattach">;
export declare function createServiceRemoteViewRouteSwitchRequest(
  input: ServiceRemoteViewBrowserReattachOptions,
): ServiceRequestForAction<"service_remote_view_route_switch">;
export declare function createServiceRemoteViewRouteCheckoutRequest(
  input: ServiceRemoteViewRouteCheckoutOptions,
): ServiceRequestForAction<"service_remote_view_route_checkout">;
export declare function createServiceRemoteViewRouteReleaseRequest(
  input: ServiceRemoteViewRouteReleaseOptions,
): ServiceRequestForAction<"service_remote_view_route_release">;
export declare function createServiceRoutePoolRepairRequest(
  input?: ServiceRoutePoolRepairOptions,
): ServiceRequestForAction<"service_route_pool_repair">;
export declare function createServiceViewerLeaseRequest(
  input: ServiceViewerLeaseRequestOptions,
): ServiceRequestForAction<"service_viewer_lease_request">;
export declare function createServiceViewerLeaseHeartbeatRequest(
  input: ServiceViewerLeaseHeartbeatOptions,
): ServiceRequestForAction<"service_viewer_lease_heartbeat">;
export declare function createServiceViewerLeaseReleaseRequest(
  input: ServiceViewerLeaseReleaseOptions,
): ServiceRequestForAction<"service_viewer_lease_release">;
export declare function createServiceControllerLeaseTakeoverRequest(
  input: ServiceControllerLeaseTakeoverOptions,
): ServiceRequestForAction<"service_controller_lease_takeover">;
export declare function requestServiceTab(options: ServiceTabRequestHttpOptions): Promise<ServiceRequestResponse<ServiceTabNewData>>;
export declare function requestServiceTabFromAccessPlan(
  options: ServiceTabRequestHttpOptions,
): Promise<ServiceRequestResponse<ServiceTabNewData>>;
export declare function requestServiceCdpFreeLaunch(
  options: ServiceCdpFreeLaunchRequestHttpOptions,
): Promise<ServiceRequestResponse<ServiceCdpFreeLaunchData>>;
export declare function requestServiceExternalByopAdopt(
  options: ServiceExternalByopAdoptRequestHttpOptions,
): Promise<ServiceRequestResponse<ServiceExternalByopAdoptData>>;
export declare const adoptExternalByopBrowser: typeof requestServiceExternalByopAdopt;
export declare function requestServiceCdpAttach(
  options: ServiceCdpAttachRequestHttpOptions,
): Promise<ServiceRequestResponse<ServiceCdpAttachDescriptor>>;
export declare function attachServiceTabCdp(
  options: ServiceCdpAttachRequestHttpOptions,
): Promise<ServiceRequestResponse<ServiceCdpAttachDescriptor>>;
export declare function requestServiceCdpDetach(
  options: ServiceCdpDetachRequestHttpOptions,
): Promise<ServiceRequestResponse<ServiceCdpDetachData>>;
export declare function requestServiceEvaluate(
  options: ServiceEvaluateRequestHttpOptions,
): Promise<ServiceRequestResponse<ServiceEvaluateData>>;
export declare function evaluateServiceTab(
  options: ServiceEvaluateRequestHttpOptions,
): Promise<ServiceRequestResponse<ServiceEvaluateData>>;
export declare function requestServiceDiagnostics(
  options: ServiceDiagnosticsRequestHttpOptions,
): Promise<ServiceRequestResponse<ServiceDiagnosticsData>>;
export declare function getServiceTabDiagnostics(
  options: ServiceDiagnosticsRequestHttpOptions,
): Promise<ServiceRequestResponse<ServiceDiagnosticsData>>;
export declare function requestServiceProbe(
  options: ServiceProbeRequestHttpOptions,
): Promise<ServiceRequestResponse<ServiceProbeData>>;
export declare function probeServiceTab(
  options: ServiceProbeRequestHttpOptions,
): Promise<ServiceRequestResponse<ServiceProbeData>>;
export declare function requestServiceUiAction(
  options: ServiceUiActionRequestHttpOptions,
): Promise<ServiceRequestResponse<ServiceUiActionData>>;
export declare function runServiceUiAction(
  options: ServiceUiActionRequestHttpOptions,
): Promise<ServiceRequestResponse<ServiceUiActionData>>;
export declare function requestServiceNetworkCapture(
  options: ServiceNetworkCaptureRequestHttpOptions,
): Promise<ServiceRequestResponse<ServiceNetworkCaptureData>>;
export declare function captureServiceNetwork(
  options: ServiceNetworkCaptureRequestHttpOptions,
): Promise<ServiceRequestResponse<ServiceNetworkCaptureData>>;
export declare function requestServiceFileTransfer(
  options: ServiceFileTransferRequestHttpOptions,
): Promise<ServiceRequestResponse<ServiceFileTransferData>>;
export declare function transferServiceFiles(
  options: ServiceFileTransferRequestHttpOptions,
): Promise<ServiceRequestResponse<ServiceFileTransferData>>;
export declare function refreshServiceTabHandle(
  options: ServiceTabHandleRefreshHttpOptions,
): Promise<ServiceRequestResponse<ServiceTabHandleRefreshData>>;
export declare function requestServiceTabHandleRefresh(
  options: ServiceTabHandleRefreshHttpOptions,
): Promise<ServiceRequestResponse<ServiceTabHandleRefreshData>>;
export declare function releaseServiceTabHandle(
  options: ServiceTabHandleReleaseHttpOptions,
): Promise<ServiceRequestResponse<ServiceTabHandleReleaseData>>;
export declare function requestServiceTabHandleRelease(
  options: ServiceTabHandleReleaseHttpOptions,
): Promise<ServiceRequestResponse<ServiceTabHandleReleaseData>>;
export declare function requestServiceRemoteViewRoutePreflight(
  options: ServiceRemoteViewRouteCheckoutHttpOptions,
): Promise<ServiceRequestResponse<ServiceRemoteViewRouteMutationData>>;
export declare function requestServiceRemoteViewOpen(
  options: ServiceRemoteViewRouteCheckoutHttpOptions,
): Promise<ServiceRequestResponse<ServiceRemoteViewRouteMutationData>>;
export declare function requestServiceRemoteViewBrowserReattach(
  options: ServiceRemoteViewBrowserReattachHttpOptions,
): Promise<ServiceRequestResponse<ServiceRemoteViewRouteMutationData>>;
export declare function requestServiceRemoteViewRouteSwitch(
  options: ServiceRemoteViewBrowserReattachHttpOptions,
): Promise<ServiceRequestResponse<ServiceRemoteViewRouteMutationData>>;
export declare function getServiceRemoteViewOpenOperatorVisible(response: unknown): Record<string, unknown> | null;
export declare function isServiceRemoteViewOpenOperatorVisibleReady(response: unknown): boolean;
export declare function summarizeServiceRemoteViewOpenProof(response: unknown): ServiceRemoteViewOpenProofSummary;
export declare function summarizeServiceSharedProfileAcquisition(
  input: unknown,
): ServiceSharedProfileAcquisitionSummary;
export declare function requireServiceRemoteViewOpenOperatorVisible(
  response: unknown,
  options?: { allowInfrastructureOnlyReadiness?: boolean },
): Record<string, unknown> | null;
export declare function requestServiceRemoteViewRouteCheckout(
  options: ServiceRemoteViewRouteCheckoutHttpOptions,
): Promise<ServiceRequestResponse<ServiceRemoteViewRouteMutationData>>;
export declare function requestServiceRemoteViewRouteRelease(
  options: ServiceRemoteViewRouteReleaseHttpOptions,
): Promise<ServiceRequestResponse<ServiceRemoteViewRouteMutationData>>;
export declare function requestServiceRoutePoolRepair(
  options: ServiceRoutePoolRepairHttpOptions,
): Promise<ServiceRequestResponse<ServiceRoutePoolRepairData>>;
export declare function requestServiceViewerLease(
  options: ServiceViewerLeaseRequestHttpOptions,
): Promise<ServiceRequestResponse<ServiceViewerLeaseMutationData>>;
export declare function heartbeatServiceViewerLease(
  options: ServiceViewerLeaseHeartbeatHttpOptions,
): Promise<ServiceRequestResponse<ServiceViewerLeaseMutationData>>;
export declare function releaseServiceViewerLease(
  options: ServiceViewerLeaseReleaseHttpOptions,
): Promise<ServiceRequestResponse<ServiceViewerLeaseMutationData>>;
export declare function takeoverServiceControllerLease(
  options: ServiceControllerLeaseTakeoverHttpOptions,
): Promise<ServiceRequestResponse<ServiceViewerLeaseMutationData>>;
export declare function summarizeServiceCdpFreeLaunchAvailability(
  data: ServiceCdpFreeLaunchData,
): ServiceCdpFreeLaunchAvailability;
export declare function isServiceCdpFreeActionAvailable(
  data: ServiceCdpFreeLaunchData,
  action: ServiceRequestAction,
): boolean;
`;
}

function generatedHeader(extension) {
  const comment = extension === 'js' ? '//' : '//';
  return `${comment} Generated by scripts/generate-service-request-client.js from docs/dev/contracts/service-request.v1.schema.json.
${comment} Do not edit by hand.

`;
}

function json(value) {
  return `${JSON.stringify(value, null, 2)} as const`.replace(' as const', '');
}
