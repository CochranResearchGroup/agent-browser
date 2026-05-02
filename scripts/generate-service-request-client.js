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

  return `${generatedHeader('ts')}export type ServiceRequestAction =
${actionUnion};

export interface ServiceRequest {
  action: ServiceRequestAction;
  params?: Record<string, unknown>;
  jobTimeoutMs?: number;
${stringFieldLines}
${stringArrayFieldLines}
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

export interface ServiceRequestActionDataMap {
  navigate: ServiceNavigateData;
  tab_new: ServiceTabNewData;
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

export interface ServiceTabRequestOptions extends Omit<ServiceRequest, "action" | "params"> {
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
export declare const SERVICE_REQUEST_MCP_TOOL_NAME: ${JSON.stringify(mcpToolName)};

export declare function createServiceRequest<TRequest extends ServiceRequest>(input: TRequest): TRequest;
export declare function createServiceRequestMcpToolCall(input: ServiceRequest): ServiceRequestMcpToolCall;
export declare function postServiceRequest<TRequest extends ServiceRequest>(
  options: ServiceRequestHttpOptions<TRequest>,
): Promise<ServiceRequestResponse<ServiceRequestDataForAction<TRequest["action"]>>>;
export declare function createServiceTabRequest(input: ServiceTabRequestOptions): ServiceRequestForAction<"tab_new">;
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
