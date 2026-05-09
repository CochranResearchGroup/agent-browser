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

  return `${generatedHeader('ts')}export type ServiceRequestAction =
${actionUnion};

export interface ServiceRequest {
  action: ServiceRequestAction;
  params?: Record<string, unknown>;
${stringFieldLines}
${stringArrayFieldLines}
${integerFieldLines}
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

export interface ServiceTabNewData {
  index: number;
  url: string;
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

export interface ServiceTabRecord {
  index: number;
  title: string;
  url: string;
  type: string;
  active: boolean;
  targetId?: string;
  sessionId?: string;
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

export interface ServiceRequestActionDataMap {
  navigate: ServiceNavigateData;
  back: ServiceUrlData;
  forward: ServiceUrlData;
  reload: ServiceUrlData;
  tab_new: ServiceTabNewData;
  tab_switch: ServiceTabSwitchData;
  tab_close: ServiceTabCloseData;
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
  decision: {
    serviceRequest: {
      request: ServiceRequestForAction<"tab_new">;
      [key: string]: unknown;
    };
    [key: string]: unknown;
  };
  [key: string]: unknown;
}

export interface ServiceTabRequestOptions extends Omit<ServiceRequest, "action" | "params"> {
  accessPlan?: ServiceTabAccessPlan;
  url?: string;
  params?: Record<string, unknown>;
}

export interface ServiceTabRequestHttpOptions extends ServiceTabRequestOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export declare const SERVICE_REQUEST_ACTIONS: readonly ServiceRequestAction[];
export declare const SERVICE_REQUEST_REQUIRED_FIELDS: readonly string[];
export declare const SERVICE_REQUEST_STRING_FIELDS: readonly string[];
export declare const SERVICE_REQUEST_STRING_ARRAY_FIELDS: readonly string[];
export declare const SERVICE_REQUEST_INTEGER_FIELDS: readonly string[];
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
export declare function requestServiceTab(options: ServiceTabRequestHttpOptions): Promise<ServiceRequestResponse<ServiceTabNewData>>;
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
