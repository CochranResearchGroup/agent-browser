#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '..');
const check = process.argv.includes('--check');

const schemas = {
  job: readSchema('service-job-record.v1.schema.json'),
  jobsResponse: readSchema('service-jobs-response.v1.schema.json'),
  incident: readSchema('service-incident-record.v1.schema.json'),
  incidentsResponse: readSchema('service-incidents-response.v1.schema.json'),
  incidentActivityResponse: readSchema('service-incident-activity-response.v1.schema.json'),
  event: readSchema('service-event-record.v1.schema.json'),
  eventsResponse: readSchema('service-events-response.v1.schema.json'),
  traceResponse: readSchema('service-trace-response.v1.schema.json'),
};

const generatedJsPath = path.join(repoRoot, 'packages/client/src/service-observability.generated.js');
const generatedTypesPath = path.join(repoRoot, 'packages/client/src/service-observability.generated.d.ts');

const constants = {
  SERVICE_JOB_STATES: enumValues(schemas.job, ['properties', 'state', 'enum']),
  SERVICE_JOB_PRIORITIES: enumValues(schemas.job, ['properties', 'priority', 'enum']),
  SERVICE_NAMING_WARNINGS: enumValues(schemas.job, [
    'properties',
    'namingWarnings',
    'items',
    'enum',
  ]),
  SERVICE_INCIDENT_STATES: enumValues(schemas.incident, ['properties', 'state', 'enum']),
  SERVICE_INCIDENT_SEVERITIES: enumValues(schemas.incident, ['properties', 'severity', 'enum']),
  SERVICE_INCIDENT_ESCALATIONS: enumValues(schemas.incident, ['properties', 'escalation', 'enum']),
  SERVICE_INCIDENT_HANDLING_STATES: enumValues(schemas.incidentsResponse, [
    'properties',
    'filters',
    'properties',
    'handlingState',
    'enum',
  ]).filter((value) => value !== null),
  SERVICE_EVENT_KINDS: enumValues(schemas.event, ['properties', 'kind', 'enum']),
  SERVICE_BROWSER_HEALTH_STATES: enumValues(schemas.event, [
    'properties',
    'currentHealth',
    'oneOf',
    '0',
    'enum',
  ]),
};

writeGenerated(generatedJsPath, renderGeneratedJs());
writeGenerated(generatedTypesPath, renderGeneratedTypes());

function readSchema(fileName) {
  const filePath = path.join(repoRoot, 'docs/dev/contracts', fileName);
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function enumValues(schema, pathParts) {
  let value = schema;
  for (const part of pathParts) {
    value = value?.[part];
  }
  if (!Array.isArray(value)) {
    throw new Error(`Missing enum at ${pathParts.join('.')}`);
  }
  return value;
}

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
  return `${generatedHeader()}${Object.entries(constants)
    .map(([name, value]) => `export const ${name} = ${JSON.stringify(value, null, 2)};`)
    .join('\n\n')}
`;
}

function renderGeneratedTypes() {
  return `${generatedHeader()}${stringUnionType('ServiceJobState', constants.SERVICE_JOB_STATES)}
${stringUnionType('ServiceJobPriority', constants.SERVICE_JOB_PRIORITIES)}
${stringUnionType('ServiceNamingWarning', constants.SERVICE_NAMING_WARNINGS)}
${stringUnionType('ServiceIncidentState', constants.SERVICE_INCIDENT_STATES)}
${stringUnionType('ServiceIncidentSeverity', constants.SERVICE_INCIDENT_SEVERITIES)}
${stringUnionType('ServiceIncidentEscalation', constants.SERVICE_INCIDENT_ESCALATIONS)}
${stringUnionType('ServiceIncidentHandlingState', constants.SERVICE_INCIDENT_HANDLING_STATES)}
${stringUnionType('ServiceEventKind', constants.SERVICE_EVENT_KINDS)}
${stringUnionType('ServiceBrowserHealthState', constants.SERVICE_BROWSER_HEALTH_STATES)}

export interface ServiceJobRecord {
  id: string;
  action: string;
  serviceName: string | null;
  agentName: string | null;
  taskName: string | null;
  namingWarnings: ServiceNamingWarning[];
  hasNamingWarning: boolean;
  target: unknown;
  owner: unknown;
  state: ServiceJobState;
  priority: ServiceJobPriority;
  submittedAt: string | null;
  startedAt: string | null;
  completedAt: string | null;
  timeoutMs: number | null;
  result: unknown;
  error: string | null;
  [key: string]: unknown;
}

export interface ServiceEventRecord {
  id: string;
  timestamp: string;
  kind: ServiceEventKind;
  message: string;
  browserId: string | null;
  profileId: string | null;
  sessionId: string | null;
  serviceName: string | null;
  agentName: string | null;
  taskName: string | null;
  previousHealth: ServiceBrowserHealthState | null;
  currentHealth: ServiceBrowserHealthState | null;
  details: unknown;
  [key: string]: unknown;
}

export interface ServiceIncidentRecord {
  id: string;
  browserId: string | null;
  label: string;
  state: ServiceIncidentState;
  severity: ServiceIncidentSeverity;
  escalation: ServiceIncidentEscalation;
  recommendedAction: string;
  acknowledgedAt: string | null;
  acknowledgedBy: string | null;
  acknowledgementNote: string | null;
  resolvedAt: string | null;
  resolvedBy: string | null;
  resolutionNote: string | null;
  latestTimestamp: string;
  latestMessage: string;
  latestKind: string;
  currentHealth: ServiceBrowserHealthState | null;
  eventIds: string[];
  jobIds: string[];
  [key: string]: unknown;
}

export interface ServiceListResponse<TRecord> {
  count: number;
  matched: number;
  total: number;
  [key: string]: unknown;
}

export interface ServiceJobsResponse extends ServiceListResponse<ServiceJobRecord> {
  jobs: ServiceJobRecord[];
  job?: ServiceJobRecord;
}

export interface ServiceEventsResponse extends ServiceListResponse<ServiceEventRecord> {
  events: ServiceEventRecord[];
}

export interface ServiceIncidentSummaryGroup {
  escalation: string;
  severity: string;
  state: string;
  count: number;
  latestTimestamp: string;
  recommendedAction: string;
  incidentIds: string[];
  [key: string]: unknown;
}

export interface ServiceIncidentSummary {
  groupCount: number;
  groups: ServiceIncidentSummaryGroup[];
  [key: string]: unknown;
}

export interface ServiceIncidentsResponse extends ServiceListResponse<ServiceIncidentRecord> {
  incidents: ServiceIncidentRecord[];
  incident?: ServiceIncidentRecord;
  events?: ServiceEventRecord[];
  jobs?: ServiceJobRecord[];
  summary?: ServiceIncidentSummary;
  filters?: Record<string, unknown>;
}

export interface ServiceIncidentActivityResponse {
  incident: ServiceIncidentRecord;
  activity: Record<string, unknown>[];
  count: number;
  [key: string]: unknown;
}

export interface ServiceTraceResponse {
  filters: Record<string, unknown>;
  events: ServiceEventRecord[];
  jobs: ServiceJobRecord[];
  incidents: ServiceIncidentRecord[];
  activity: Record<string, unknown>[];
  summary: Record<string, unknown>;
  counts: Record<'events' | 'jobs' | 'incidents' | 'activity', number>;
  matched: Record<'events' | 'jobs' | 'incidents' | 'activity', number>;
  total: Record<'events' | 'jobs' | 'incidents', number>;
  [key: string]: unknown;
}

export interface ServiceObservabilityHttpOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  signal?: AbortSignal;
}

export interface ServiceQueryOptions extends ServiceObservabilityHttpOptions {
  query?: Record<string, string | number | boolean | null | undefined>;
}

export interface ServiceIdOptions extends ServiceObservabilityHttpOptions {
  id: string;
}

export interface ServiceIncidentActivityOptions extends ServiceObservabilityHttpOptions {
  incidentId: string;
}

${Object.entries(constants)
  .map(([name]) => `export declare const ${name}: readonly string[];`)
  .join('\n')}

export declare function getServiceJobs(options: ServiceQueryOptions): Promise<ServiceJobsResponse>;
export declare function getServiceJob(options: ServiceIdOptions): Promise<ServiceJobsResponse>;
export declare function getServiceEvents(options: ServiceQueryOptions): Promise<ServiceEventsResponse>;
export declare function getServiceIncidents(options: ServiceQueryOptions): Promise<ServiceIncidentsResponse>;
export declare function getServiceIncident(options: ServiceIdOptions): Promise<ServiceIncidentsResponse>;
export declare function getServiceIncidentActivity(options: ServiceIncidentActivityOptions): Promise<ServiceIncidentActivityResponse>;
export declare function getServiceTrace(options: ServiceQueryOptions): Promise<ServiceTraceResponse>;
`;
}

function stringUnionType(name, values) {
  return `export type ${name} =\n${values.map((value) => `  | ${JSON.stringify(value)}`).join('\n')};\n`;
}

function generatedHeader() {
  return `// Generated by scripts/generate-service-observability-client.js from docs/dev/contracts/service-*.json.
// Do not edit by hand.

`;
}
