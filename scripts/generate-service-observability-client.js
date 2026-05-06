#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '..');
const check = process.argv.includes('--check');

const schemas = {
  statusResponse: readSchema('service-status-response.v1.schema.json'),
  profile: readSchema('service-profile-record.v1.schema.json'),
  profilesResponse: readSchema('service-profiles-response.v1.schema.json'),
  profileAllocationResponse: readSchema('service-profile-allocation-response.v1.schema.json'),
  profileReadinessResponse: readSchema('service-profile-readiness-response.v1.schema.json'),
  profileLookupResponse: readSchema('service-profile-lookup-response.v1.schema.json'),
  accessPlanResponse: readSchema('service-access-plan-response.v1.schema.json'),
  browser: readSchema('service-browser-record.v1.schema.json'),
  browsersResponse: readSchema('service-browsers-response.v1.schema.json'),
  session: readSchema('service-session-record.v1.schema.json'),
  sessionsResponse: readSchema('service-sessions-response.v1.schema.json'),
  tab: readSchema('service-tab-record.v1.schema.json'),
  tabsResponse: readSchema('service-tabs-response.v1.schema.json'),
  sitePolicy: readSchema('service-site-policy-record.v1.schema.json'),
  sitePoliciesResponse: readSchema('service-site-policies-response.v1.schema.json'),
  provider: readSchema('service-provider-record.v1.schema.json'),
  providersResponse: readSchema('service-providers-response.v1.schema.json'),
  challenge: readSchema('service-challenge-record.v1.schema.json'),
  challengesResponse: readSchema('service-challenges-response.v1.schema.json'),
  reconcileResponse: readSchema('service-reconcile-response.v1.schema.json'),
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
  targetServiceId: string | null;
  siteId: string | null;
  loginId: string | null;
  targetServiceIds: string[];
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

export interface ServiceProfileRecord {
  id: string;
  name: string;
  userDataDir: string | null;
  targetReadiness: ServiceProfileTargetReadiness[];
  [key: string]: unknown;
}

export interface ServiceProfileSourceRecord {
  id: string;
  source: 'config' | 'runtime_observed' | 'persisted_state';
  overrideable: boolean;
  precedence: Array<'config' | 'runtime_observed' | 'persisted_state'>;
  [key: string]: unknown;
}

export interface ServiceProfileTargetReadiness {
  targetServiceId: string;
  loginId: string | null;
  state:
    | 'unknown'
    | 'needs_manual_seeding'
    | 'seeded_unknown_freshness'
    | 'fresh'
    | 'stale'
    | 'blocked_by_attached_devtools'
    | string;
  manualSeedingRequired: boolean;
  evidence: string;
  recommendedAction: string;
  lastVerifiedAt: string | null;
  freshnessExpiresAt: string | null;
  [key: string]: unknown;
}

export interface ServiceProfileAllocation {
  profileId: string;
  profileName: string;
  allocation: string;
  keyring: string;
  targetServiceIds: string[];
  authenticatedServiceIds: string[];
  targetReadiness: ServiceProfileTargetReadiness[];
  sharedServiceIds: string[];
  holderSessionIds: string[];
  holderCount: number;
  exclusiveHolderSessionIds: string[];
  waitingJobIds: string[];
  waitingJobCount: number;
  conflictSessionIds: string[];
  leaseState: 'available' | 'shared' | 'exclusive' | 'waiting' | 'conflicted' | string;
  recommendedAction: string;
  serviceNames: string[];
  agentNames: string[];
  taskNames: string[];
  browserIds: string[];
  tabIds: string[];
  [key: string]: unknown;
}

export interface ServiceBrowserRecord {
  id: string;
  profileId: string | null;
  health: ServiceBrowserHealthState;
  [key: string]: unknown;
}

export interface ServiceSessionRecord {
  id: string;
  serviceName: string | null;
  agentName: string | null;
  taskName: string | null;
  profileId: string | null;
  profileSelectionReason: 'explicit_profile' | 'authenticated_target' | 'target_match' | 'service_allow_list' | null;
  profileLeaseDisposition: 'new_browser' | 'reused_browser' | 'active_lease_conflict' | null;
  profileLeaseConflictSessionIds: string[];
  [key: string]: unknown;
}

export interface ServiceTabRecord {
  id: string;
  browserId: string | null;
  sessionId: string | null;
  lifecycle: string;
  [key: string]: unknown;
}

export interface ServiceSitePolicyRecord {
  id: string;
  originPattern: string | null;
  [key: string]: unknown;
}

export interface ServiceSitePolicySourceRecord {
  id: string;
  source: 'config' | 'persisted_state' | 'builtin';
  overrideable: boolean;
  precedence: Array<'config' | 'persisted_state' | 'builtin'>;
  [key: string]: unknown;
}

export interface ServiceProviderRecord {
  id: string;
  kind: string;
  displayName: string | null;
  enabled: boolean;
  [key: string]: unknown;
}

export interface ServiceChallengeRecord {
  id: string;
  tabId: string | null;
  kind: string;
  state: string;
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
  matched?: number;
  total?: number;
  [key: string]: unknown;
}

export interface ServiceJobsResponse extends ServiceListResponse<ServiceJobRecord> {
  jobs: ServiceJobRecord[];
  job?: ServiceJobRecord;
}

export interface ServiceControlPlaneStatus {
  worker_state?: string;
  browser_health?: string;
  queue_depth?: number;
  queue_capacity?: number;
  waiting_profile_lease_job_count?: number;
  service_job_timeout_ms?: number | null;
  [key: string]: unknown;
}

export interface ServiceStatusResponse {
  control_plane?: ServiceControlPlaneStatus;
  service_state: Record<string, unknown>;
  profileAllocations: ServiceProfileAllocation[];
  [key: string]: unknown;
}

export interface ServiceContractEndpoint {
  method?: string;
  route?: string;
  tool?: string;
  argumentsSchemaId?: string;
  toolCallSchemaId?: string;
  [key: string]: unknown;
}

export interface ServiceContractRecord {
  version: string;
  schemaId: string;
  schemaPath?: string;
  http?: ServiceContractEndpoint;
  mcp?: ServiceContractEndpoint;
  actions?: string[];
  actionCount?: number;
  tool?: string;
  [key: string]: unknown;
}

export interface ServiceContractsResponse {
  schemaVersion: string;
  contracts: Record<string, ServiceContractRecord>;
  http: Record<string, unknown>;
  mcp: Record<string, unknown>;
  [key: string]: unknown;
}

export interface ServiceProfilesResponse extends ServiceListResponse<ServiceProfileRecord> {
  profiles: ServiceProfileRecord[];
  profileSources: ServiceProfileSourceRecord[];
  profileAllocations: ServiceProfileAllocation[];
}

export interface ServiceProfileAllocationResponse {
  profileAllocation: ServiceProfileAllocation;
  [key: string]: unknown;
}

export interface ServiceProfileReadinessResponse {
  profileId: string;
  targetReadiness: ServiceProfileTargetReadiness[];
  count: number;
  [key: string]: unknown;
}

export interface ServiceBrowsersResponse extends ServiceListResponse<ServiceBrowserRecord> {
  browsers: ServiceBrowserRecord[];
}

export interface ServiceSessionsResponse extends ServiceListResponse<ServiceSessionRecord> {
  sessions: ServiceSessionRecord[];
}

export interface ServiceTabsResponse extends ServiceListResponse<ServiceTabRecord> {
  tabs: ServiceTabRecord[];
}

export interface ServiceSitePoliciesResponse extends ServiceListResponse<ServiceSitePolicyRecord> {
  sitePolicies: ServiceSitePolicyRecord[];
  sitePolicySources: ServiceSitePolicySourceRecord[];
}

export interface ServiceProvidersResponse extends ServiceListResponse<ServiceProviderRecord> {
  providers: ServiceProviderRecord[];
}

export interface ServiceChallengesResponse extends ServiceListResponse<ServiceChallengeRecord> {
  challenges: ServiceChallengeRecord[];
}

export interface ServiceReconcileResponse {
  reconciled: boolean;
  service_state: Record<string, unknown>;
  [key: string]: unknown;
}

export interface ServiceProfileUpsertResponse {
  id: string;
  upserted: boolean;
  profile: ServiceProfileRecord;
  [key: string]: unknown;
}

export interface ServiceProfileDeleteResponse {
  id: string;
  deleted: boolean;
  profile: ServiceProfileRecord | null;
  [key: string]: unknown;
}

export interface ServiceSessionUpsertResponse {
  id: string;
  upserted: boolean;
  session: ServiceSessionRecord;
  [key: string]: unknown;
}

export interface ServiceSessionDeleteResponse {
  id: string;
  deleted: boolean;
  session: ServiceSessionRecord | null;
  [key: string]: unknown;
}

export interface ServiceSitePolicyUpsertResponse {
  id: string;
  upserted: boolean;
  sitePolicy: ServiceSitePolicyRecord;
  [key: string]: unknown;
}

export interface ServiceSitePolicyDeleteResponse {
  id: string;
  deleted: boolean;
  sitePolicy: ServiceSitePolicyRecord | null;
  [key: string]: unknown;
}

export interface ServiceProviderUpsertResponse {
  id: string;
  upserted: boolean;
  provider: ServiceProviderRecord;
  [key: string]: unknown;
}

export interface ServiceProviderDeleteResponse {
  id: string;
  deleted: boolean;
  provider: ServiceProviderRecord | null;
  [key: string]: unknown;
}

export interface ServiceJobCancelResponse {
  cancelled: boolean;
  job: ServiceJobRecord;
  [key: string]: unknown;
}

export interface ServiceBrowserRetryResponse {
  retryEnabled: boolean;
  browser: ServiceBrowserRecord;
  incident: ServiceIncidentRecord | null;
  [key: string]: unknown;
}

export interface ServiceIncidentAcknowledgeResponse {
  acknowledged: boolean;
  incident: ServiceIncidentRecord;
  [key: string]: unknown;
}

export interface ServiceIncidentResolveResponse {
  resolved: boolean;
  incident: ServiceIncidentRecord;
  [key: string]: unknown;
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

export interface ServiceTraceProfileLeaseWait {
  jobId: string;
  profileId: string | null;
  outcome: string;
  startedAt: string | null;
  endedAt: string | null;
  waitedMs: number | null;
  retryAfterMs: number | null;
  conflictSessionIds: string[];
  serviceName: string | null;
  agentName: string | null;
  taskName: string | null;
  [key: string]: unknown;
}

export interface ServiceTraceProfileLeaseWaitSummary {
  count: number;
  activeCount: number;
  completedCount: number;
  waits: ServiceTraceProfileLeaseWait[];
  [key: string]: unknown;
}

export interface ServiceTraceSummaryContext {
  serviceName: string | null;
  agentName: string | null;
  taskName: string | null;
  browserId: string | null;
  profileId: string | null;
  sessionId: string | null;
  namingWarnings: ServiceNamingWarning[];
  hasNamingWarning: boolean;
  eventCount: number;
  jobCount: number;
  incidentCount: number;
  activityCount: number;
  targetIdentityCount: number;
  targetServiceIds: string[];
  latestTimestamp: string | null;
  [key: string]: unknown;
}

export interface ServiceTraceSummary {
  contextCount: number;
  hasTraceContext: boolean;
  namingWarningCount: number;
  profileLeaseWaits: ServiceTraceProfileLeaseWaitSummary;
  contexts: ServiceTraceSummaryContext[];
  [key: string]: unknown;
}

export interface ServiceTraceResponse {
  filters: Record<string, unknown>;
  events: ServiceEventRecord[];
  jobs: ServiceJobRecord[];
  incidents: ServiceIncidentRecord[];
  activity: Record<string, unknown>[];
  summary: ServiceTraceSummary;
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

export interface ServiceProfileMutationOptions extends ServiceIdOptions {
  profile: Record<string, unknown>;
}

export interface ServiceLoginProfileRegistrationOptions extends ServiceObservabilityHttpOptions {
  id: string;
  serviceName: string;
  loginId?: string;
  siteId?: string;
  targetServiceId?: string;
  targetServiceIds?: string[];
  authenticatedServiceIds?: string[];
  sharedServiceIds?: string[];
  targetReadiness?: ServiceProfileTargetReadiness[];
  readinessState?: ServiceProfileTargetReadiness['state'];
  readinessEvidence?: string;
  readinessRecommendedAction?: string;
  lastVerifiedAt?: string | null;
  freshnessExpiresAt?: string | null;
  name?: string;
  allocation?: string;
  keyring?: string;
  persistent?: boolean;
  authenticated?: boolean;
  userDataDir?: string;
  profile?: Record<string, unknown>;
}

export interface ServiceProfileReadinessSummary {
  /** True when at least one target explicitly needs detached manual profile seeding. */
  needsManualSeeding: boolean;
  /** True when any readiness row requires manual seeding before authenticated automation. */
  manualSeedingRequired: boolean;
  /** Target service or login IDs that need operator attention. */
  targetServiceIds: string[];
  /** Operator-facing recommended action codes returned by the service. */
  recommendedActions: string[];
}

export interface ServiceAccessPlanDecision {
  recommendedAction: string;
  browserHost: string | null;
  interactionMode: string | null;
  challengePolicy: string | null;
  profileId: string | null;
  manualActionRequired: boolean;
  manualSeedingRequired: boolean;
  providerIds: string[];
  challengeIds: string[];
  namingWarnings: ServiceNamingWarning[];
  hasNamingWarning: boolean;
  reasons: string[];
  [key: string]: unknown;
}

export interface ServiceProfileIdentityMatchOptions {
  /** Calling service name, for example "JournalDownloader" or "CanvaCLI". */
  serviceName?: string;
  /** Desired login identity or site login scope, for example "acs" or "canva". */
  loginId?: string;
  /** Desired site identity alias. Treated like a target service ID for profile selection. */
  siteId?: string;
  /** Desired target site or identity provider, for example "google", "microsoft", or "canva". */
  targetServiceId?: string;
  /** Additional desired login identities. */
  loginIds?: string[];
  /** Additional desired site identities. */
  siteIds?: string[];
  /** Additional desired target site or identity-provider IDs. */
  targetServiceIds?: string[];
}

export interface ServiceProfileIdentityMatchResult {
  profile: ServiceProfileRecord | null;
  reason: 'authenticated_target' | 'target_match' | 'service_allow_list' | null;
  matchedField: 'authenticatedServiceIds' | 'targetServiceIds' | 'sharedServiceIds' | null;
  matchedIdentity: string | null;
}

export interface ServiceProfileIdentityLookupOptions extends ServiceQueryOptions, ServiceProfileIdentityMatchOptions {
  /** Optional explicit profile ID whose readiness rows should be included with the lookup response. */
  readinessProfileId?: string;
}

export interface ServiceAccessPlanOptions extends ServiceProfileIdentityLookupOptions {
  /** Calling agent name for multi-agent traceability. */
  agentName?: string;
  /** Caller task name for queue and trace debugging. */
  taskName?: string;
  /** Optional site-policy ID when the caller wants a specific policy recommendation. */
  sitePolicyId?: string;
  /** Optional challenge ID when planning around one retained challenge. */
  challengeId?: string;
}

export interface ServiceProfileLookupQuery {
  serviceName: string | null;
  targetServiceIds: string[];
  readinessProfileId: string | null;
  [key: string]: unknown;
}

export interface ServiceProfileLookupMatch {
  profileId: string;
  profile: ServiceProfileRecord;
  reason: 'authenticated_target' | 'target_match' | 'service_allow_list';
  matchedField: 'authenticatedServiceIds' | 'targetServiceIds' | 'sharedServiceIds' | null;
  matchedIdentity: string | null;
  [key: string]: unknown;
}

export interface ServiceProfileLookupResponse {
  /** Normalized lookup query after login, site, and target aliases have been folded together. */
  query: ServiceProfileLookupQuery;
  /** Server-selected profile, or null when agent-browser has no suitable managed profile. */
  selectedProfile: ServiceProfileRecord | null;
  /** Provenance for the selected profile after config, runtime observation, and persisted state are layered. */
  selectedProfileSource: ServiceProfileSourceRecord | null;
  /** Match metadata explaining the selected profile and selector reason. */
  selectedProfileMatch: ServiceProfileLookupMatch | null;
  /** Optional no-launch readiness rows for the selected or requested readiness profile. */
  readiness: ServiceProfileReadinessResponse | null;
  /** Compact manual-seeding summary suitable for operator UI or logs. */
  readinessSummary: ServiceProfileReadinessSummary;
  [key: string]: unknown;
}

export interface ServiceAccessPlanQuery extends ServiceProfileLookupQuery {
  agentName: string | null;
  taskName: string | null;
  sitePolicyId: string | null;
  challengeId: string | null;
  namingWarnings: ServiceNamingWarning[];
  hasNamingWarning: boolean;
}

export interface ServiceSitePolicySource {
  id: string;
  source: 'config' | 'persisted_state' | 'builtin';
  matchedBy: 'explicit_site_policy_id' | 'target_service_id' | 'profile_site_policy_id';
  overrideable: boolean;
  precedence: Array<'config' | 'persisted_state' | 'builtin'>;
  [key: string]: unknown;
}

export interface ServiceAccessPlanResponse {
  /** Normalized access-plan query after login, site, and target aliases have been folded together. */
  query: ServiceAccessPlanQuery;
  /** Server-selected profile, or null when agent-browser has no suitable managed profile. */
  selectedProfile: ServiceProfileRecord | null;
  /** Provenance for the selected profile after config, runtime observation, and persisted state are layered. */
  selectedProfileSource: ServiceProfileSourceRecord | null;
  /** Match metadata explaining the selected profile and selector reason. */
  selectedProfileMatch: ServiceProfileLookupMatch | null;
  /** Optional no-launch readiness rows for the selected or requested readiness profile. */
  readiness: ServiceProfileReadinessResponse | null;
  /** Compact manual-seeding summary suitable for operator UI or logs. */
  readinessSummary: ServiceProfileReadinessSummary;
  /** Site policy selected for this request, or null when none matches. */
  sitePolicy: ServiceSitePolicyRecord | null;
  /** Provenance for the selected site policy after config, persisted state, and built-ins are layered. */
  sitePolicySource: ServiceSitePolicySource | null;
  /** Enabled providers relevant to the selected profile, site policy, or challenge. */
  providers: ServiceProviderRecord[];
  /** Retained non-resolved challenges or the explicit requested challenge. */
  challenges: ServiceChallengeRecord[];
  /** Service-owned recommendation before any browser launch or control request. */
  decision: ServiceAccessPlanDecision;
  [key: string]: unknown;
}

export interface ServiceSessionMutationOptions extends ServiceIdOptions {
  session: Record<string, unknown>;
}

export interface ServiceSitePolicyMutationOptions extends ServiceIdOptions {
  sitePolicy: Record<string, unknown>;
}

export interface ServiceProviderMutationOptions extends ServiceIdOptions {
  provider: Record<string, unknown>;
}

export interface ServiceJobCancelOptions extends ServiceObservabilityHttpOptions {
  jobId: string;
}

export interface ServiceBrowserRetryOptions extends ServiceObservabilityHttpOptions {
  browserId: string;
  by?: string;
  note?: string;
  serviceName?: string;
  agentName?: string;
  taskName?: string;
}

export interface ServiceIncidentMutationOptions extends ServiceObservabilityHttpOptions {
  incidentId: string;
  by?: string;
  note?: string;
}

export interface ServiceIncidentActivityOptions extends ServiceObservabilityHttpOptions {
  incidentId: string;
}

${Object.entries(constants)
  .map(([name]) => `export declare const ${name}: readonly string[];`)
  .join('\n')}

export declare function getServiceStatus(options: ServiceObservabilityHttpOptions): Promise<ServiceStatusResponse>;
export declare function getServiceContracts(options: ServiceObservabilityHttpOptions): Promise<ServiceContractsResponse>;
export declare function getServiceProfiles(options: ServiceQueryOptions): Promise<ServiceProfilesResponse>;
export declare function getServiceProfileAllocation(options: ServiceIdOptions): Promise<ServiceProfileAllocationResponse>;
/** Read one profile's no-launch target readiness rows. */
export declare function getServiceProfileReadiness(options: ServiceIdOptions): Promise<ServiceProfileReadinessResponse>;
export declare function summarizeServiceProfileReadiness(readiness?: ServiceProfileReadinessResponse | null): ServiceProfileReadinessSummary;
export declare function findServiceProfileForIdentity(profiles: ServiceProfileRecord[] | undefined | null, options: ServiceProfileIdentityMatchOptions): ServiceProfileIdentityMatchResult;
/** Older descriptive alias for lookupServiceProfile. */
export declare function getServiceProfileForIdentity(options: ServiceProfileIdentityLookupOptions): Promise<ServiceProfileLookupResponse>;
/** Ask agent-browser to select a managed profile by service plus login, site, or target identity. */
export declare function lookupServiceProfile(options: ServiceProfileIdentityLookupOptions): Promise<ServiceProfileLookupResponse>;
/** Ask agent-browser for the no-launch profile, policy, provider, challenge, and readiness recommendation. */
export declare function getServiceAccessPlan(options: ServiceAccessPlanOptions): Promise<ServiceAccessPlanResponse>;
export declare function getServiceBrowsers(options: ServiceQueryOptions): Promise<ServiceBrowsersResponse>;
export declare function getServiceSessions(options: ServiceQueryOptions): Promise<ServiceSessionsResponse>;
export declare function getServiceTabs(options: ServiceQueryOptions): Promise<ServiceTabsResponse>;
export declare function getServiceSitePolicies(options: ServiceQueryOptions): Promise<ServiceSitePoliciesResponse>;
export declare function getServiceProviders(options: ServiceQueryOptions): Promise<ServiceProvidersResponse>;
export declare function getServiceChallenges(options: ServiceQueryOptions): Promise<ServiceChallengesResponse>;
export declare function postServiceReconcile(options: ServiceObservabilityHttpOptions): Promise<ServiceReconcileResponse>;
export declare function upsertServiceProfile(options: ServiceProfileMutationOptions): Promise<ServiceProfileUpsertResponse>;
export declare function registerServiceLoginProfile(options: ServiceLoginProfileRegistrationOptions): Promise<ServiceProfileUpsertResponse>;
export declare function deleteServiceProfile(options: ServiceIdOptions): Promise<ServiceProfileDeleteResponse>;
export declare function upsertServiceSession(options: ServiceSessionMutationOptions): Promise<ServiceSessionUpsertResponse>;
export declare function deleteServiceSession(options: ServiceIdOptions): Promise<ServiceSessionDeleteResponse>;
export declare function upsertServiceSitePolicy(options: ServiceSitePolicyMutationOptions): Promise<ServiceSitePolicyUpsertResponse>;
export declare function deleteServiceSitePolicy(options: ServiceIdOptions): Promise<ServiceSitePolicyDeleteResponse>;
export declare function upsertServiceProvider(options: ServiceProviderMutationOptions): Promise<ServiceProviderUpsertResponse>;
export declare function deleteServiceProvider(options: ServiceIdOptions): Promise<ServiceProviderDeleteResponse>;
export declare function cancelServiceJob(options: ServiceJobCancelOptions): Promise<ServiceJobCancelResponse>;
export declare function retryServiceBrowser(options: ServiceBrowserRetryOptions): Promise<ServiceBrowserRetryResponse>;
export declare function acknowledgeServiceIncident(options: ServiceIncidentMutationOptions): Promise<ServiceIncidentAcknowledgeResponse>;
export declare function resolveServiceIncident(options: ServiceIncidentMutationOptions): Promise<ServiceIncidentResolveResponse>;
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
