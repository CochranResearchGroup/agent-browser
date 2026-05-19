"use client";

import type { MouseEvent as ReactMouseEvent, ReactNode } from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useAtomValue } from "jotai/react";
import {
  AlertTriangle,
  CheckCircle2,
  Clock3,
  Eye,
  ExternalLink,
  Filter,
  GitBranch,
  History,
  Loader2,
  Maximize2,
  Minimize2,
  MoreHorizontal,
  RadioTower,
  RefreshCw,
  ServerCog,
  ShieldCheck,
} from "lucide-react";
import { activePortAtom, activeSessionNameAtom } from "@/store/sessions";
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui/tabs";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  normalizeServiceTraceData,
  traceBrowserCapabilityLaunches,
  traceProfileLeaseWaits,
  traceFilterSummary,
  traceSummaryCards,
  traceTimelineItems,
  type ServiceTraceData,
  type ServiceTraceBrowserCapabilityLaunch,
  type ServiceTraceTimelineItem,
  type ServiceTraceToolPayload,
} from "@/lib/service-trace";
import {
  formatIncidentField,
  incidentPriorityView,
  type ServiceIncidentEscalation,
  type ServiceIncidentSeverity,
} from "@/lib/service-incidents";
import {
  incidentSummaryGroupViews,
  type ServiceIncidentSummaryGroupView,
  type ServiceIncidentsData,
} from "@/lib/service-incident-summary";
import {
  profileAllocationFromLookupPayload,
  serviceProfileAllocationLookupUrl,
} from "@/lib/service-profile-allocation";
import {
  canEmbedViewStream,
  viewStreamLabel,
  type ServiceViewStream,
} from "@/lib/service-view-streams";

type ControlPlaneSnapshot = {
  worker_state?: string;
  browser_health?: string;
  queue_depth?: number;
  queue_capacity?: number;
  service_job_timeout_ms?: number | null;
};

type ReconciliationSnapshot = {
  lastReconciledAt?: string | null;
  lastError?: string | null;
  browserCount?: number;
  changedBrowsers?: number;
};

type ServiceEvent = {
  id: string;
  timestamp: string;
  kind:
    | "reconciliation"
    | "browser_health_changed"
    | "tab_lifecycle_changed"
    | "reconciliation_error"
    | "incident_acknowledged"
    | "incident_resolved"
    | string;
  message: string;
  browserId?: string | null;
  profileId?: string | null;
  sessionId?: string | null;
  serviceName?: string | null;
  agentName?: string | null;
  taskName?: string | null;
  previousHealth?: string | null;
  currentHealth?: string | null;
  details?: unknown;
};

export type ServiceBrowser = {
  id: string;
  profileId?: string | null;
  host?: string;
  health?: string;
  pid?: number | null;
  cdpEndpoint?: string | null;
  viewStreams?: ServiceViewStream[];
  activeSessionIds?: string[];
  lastError?: string | null;
};

type ServicePanelProps = {
  onBrowserInspect?: (browser: ServiceBrowser) => void;
  onInspectSelection?: (selection: ServiceInspectorSelection) => void;
  onInspectorActionsChange?: (actions: ServiceInspectorActions) => void;
};

export type ServiceSession = {
  id: string;
  owner?: unknown;
  lease?: string;
  profileId?: string | null;
  serviceName?: string | null;
  agentName?: string | null;
  taskName?: string | null;
  browserIds?: string[];
  tabIds?: string[];
  createdAt?: string | null;
  expiresAt?: string | null;
};

export type ServiceProfileAllocation = {
  profileId: string;
  profileName?: string;
  allocation?: string;
  keyring?: string;
  targetServiceIds?: string[];
  authenticatedServiceIds?: string[];
  targetReadiness?: ServiceProfileTargetReadiness[];
  sharedServiceIds?: string[];
  holderSessionIds?: string[];
  holderCount?: number;
  exclusiveHolderSessionIds?: string[];
  waitingJobIds?: string[];
  waitingJobCount?: number;
  conflictSessionIds?: string[];
  leaseState?: string;
  recommendedAction?: string;
  serviceNames?: string[];
  agentNames?: string[];
  taskNames?: string[];
  browserIds?: string[];
  tabIds?: string[];
};

type ServiceProfileTargetReadiness = {
  targetServiceId?: string;
  loginId?: string | null;
  state?: string;
  manualSeedingRequired?: boolean;
  evidence?: string;
  recommendedAction?: string;
  lastVerifiedAt?: string | null;
  freshnessExpiresAt?: string | null;
};

export type ServiceTab = {
  id: string;
  browserId?: string;
  targetId?: string | null;
  sessionId?: string | null;
  lifecycle?: string;
  url?: string | null;
  title?: string | null;
  ownerSessionId?: string | null;
  latestSnapshotId?: string | null;
  latestScreenshotId?: string | null;
  challengeId?: string | null;
};

type SelectedViewStream = {
  stream: ServiceViewStream;
  browser: ServiceBrowser;
  tab?: ServiceTab | null;
  focusMessage?: string | null;
};

export type ServiceJob = {
  id: string;
  action?: string;
  state?: string;
  priority?: string;
  target?: unknown;
  owner?: unknown;
  serviceName?: string | null;
  agentName?: string | null;
  taskName?: string | null;
  namingWarnings?: string[];
  hasNamingWarning?: boolean;
  timeoutMs?: number | null;
  request?: unknown;
  response?: unknown;
  result?: unknown;
  submittedAt?: string | null;
  startedAt?: string | null;
  completedAt?: string | null;
  error?: string | null;
};

export type ServiceInspectorSelection =
  | { kind: "browser"; browser: ServiceBrowser }
  | { kind: "profile"; allocation: ServiceProfileAllocation }
  | { kind: "incident"; incident: IncidentRecord }
  | { kind: "session"; session: ServiceSession }
  | { kind: "tab"; tab: ServiceTab; viewStreamAvailable?: boolean }
  | { kind: "job"; job: ServiceJob };

export type ServiceInspectorActions = {
  actingIncidentId?: string | null;
  onAcknowledgeIncident?: (incident: IncidentRecord, note: string) => void;
  onResolveIncident?: (incident: IncidentRecord, note: string) => void;
  onCancelJob?: (job: ServiceJob) => void;
};

type RetainedCleanupKind = "prune" | "repair";

type RetainedCleanupResult = {
  pruned?: boolean;
  repaired?: boolean;
  dryRun?: boolean;
  recommendedNextStep?: string;
  candidateCounts?: Record<string, number>;
  skippedCounts?: Record<string, number>;
  removed?: Record<string, number>;
  repairedCounts?: Record<string, number>;
  before?: Record<string, number>;
  after?: Record<string, number>;
};

type ServiceState = {
  controlPlane?: {
    workerState?: string;
    browserHealth?: string;
    queueDepth?: number;
    queueCapacity?: number;
    serviceJobTimeoutMs?: number | null;
  };
  reconciliation?: ReconciliationSnapshot | null;
  events?: ServiceEvent[];
  incidents?: ServiceIncident[];
  browsers?: Record<string, ServiceBrowser>;
  profiles?: Record<string, unknown>;
  jobs?: Record<string, ServiceJob>;
  sessions?: Record<string, ServiceSession>;
  tabs?: Record<string, ServiceTab>;
  sitePolicies?: Record<string, unknown>;
  providers?: Record<string, unknown>;
};

type ServiceStatusData = {
  control_plane?: ControlPlaneSnapshot;
  service_state?: ServiceState;
  profileAllocations?: ServiceProfileAllocation[];
};

type ServiceProfileAllocationData = {
  profileAllocation?: ServiceProfileAllocation;
};

type ServiceEventsData = {
  events?: ServiceEvent[];
  count?: number;
  matched?: number;
  total?: number;
};

type ServiceJobsData = {
  job?: ServiceJob;
  jobs?: ServiceJob[];
  count?: number;
  matched?: number;
  total?: number;
};

type ServiceIncidentActivityData = {
  incident?: ServiceIncident;
  activity?: ServiceTraceTimelineItem[];
  count?: number;
};

type ApiResponse<T> = {
  success: boolean;
  data?: T;
  error?: string | null;
};

export type IncidentRecord = {
  id: string;
  browserId?: string | null;
  label: string;
  severity?: ServiceIncidentSeverity | null;
  escalation?: ServiceIncidentEscalation | null;
  recommendedAction?: string | null;
  latestTimestamp: string;
  latestMessage: string;
  latestKind: string;
  currentHealth?: string | null;
  acknowledgedAt?: string | null;
  acknowledgedBy?: string | null;
  acknowledgementNote?: string | null;
  resolvedAt?: string | null;
  resolvedBy?: string | null;
  resolutionNote?: string | null;
  serviceEvents: ServiceEvent[];
  transitionEvents: ServiceEvent[];
  jobEvents: ServiceEvent[];
};

type ServiceIncident = {
  id: string;
  browserId?: string | null;
  label: string;
  severity?: ServiceIncidentSeverity | null;
  escalation?: ServiceIncidentEscalation | null;
  recommendedAction?: string | null;
  latestTimestamp: string;
  latestMessage: string;
  latestKind: string;
  currentHealth?: string | null;
  acknowledgedAt?: string | null;
  acknowledgedBy?: string | null;
  acknowledgementNote?: string | null;
  resolvedAt?: string | null;
  resolvedBy?: string | null;
  resolutionNote?: string | null;
  eventIds?: string[];
  jobIds?: string[];
};

type EventKindFilter =
  | "all"
  | "reconciliation"
  | "browser_health_changed"
  | "tab_lifecycle_changed"
  | "reconciliation_error"
  | "incident_acknowledged"
  | "incident_resolved";
type EventWindowFilter = "all" | "15m" | "1h" | "24h";
type EventLimit = 8 | 20 | 50;
type ServiceRecordLimit = 12 | 24 | 50 | 100;
type IncidentHandlingFilter = "all" | "unacknowledged" | "acknowledged" | "resolved";
type ServiceWorkspaceTab = "profiles" | "incidents" | "sessions" | "jobs" | "events";
type TraceFilters = {
  serviceName: string;
  agentName: string;
  taskName: string;
  browserId: string;
  profileId: string;
  sessionId: string;
  since: string;
  limit: EventLimit;
};

const EVENT_KIND_OPTIONS: Array<{ value: EventKindFilter; label: string }> = [
  { value: "all", label: "All" },
  { value: "reconciliation", label: "Reconcile" },
  { value: "browser_health_changed", label: "Health" },
  { value: "tab_lifecycle_changed", label: "Tabs" },
  { value: "reconciliation_error", label: "Errors" },
  { value: "incident_acknowledged", label: "Ack" },
  { value: "incident_resolved", label: "Resolved" },
];

const EVENT_WINDOW_OPTIONS: Array<{ value: EventWindowFilter; label: string; milliseconds?: number }> = [
  { value: "all", label: "All time" },
  { value: "15m", label: "15m", milliseconds: 15 * 60 * 1000 },
  { value: "1h", label: "1h", milliseconds: 60 * 60 * 1000 },
  { value: "24h", label: "24h", milliseconds: 24 * 60 * 60 * 1000 },
];

const EVENT_LIMIT_OPTIONS: EventLimit[] = [8, 20, 50];
const SERVICE_RECORD_LIMIT_OPTIONS: ServiceRecordLimit[] = [12, 24, 50, 100];

const INCIDENT_HANDLING_OPTIONS: Array<{ value: IncidentHandlingFilter; label: string }> = [
  { value: "all", label: "All" },
  { value: "unacknowledged", label: "Unacknowledged" },
  { value: "acknowledged", label: "Acknowledged" },
  { value: "resolved", label: "Resolved" },
];

const OPERATOR_STORAGE_KEY = "agent-browser-dashboard-operator";

function serviceBase(port: number): string {
  if (typeof window === "undefined") return "/api/service";
  const { hostname, port: locationPort } = window.location;
  const isLoopback = hostname === "localhost" || hostname === "127.0.0.1" || hostname === "::1";

  // Production dashboard and ingress routes expose the service API on the same
  // origin. Only fall back to the selected session port for local Next.js dev.
  if (isLoopback && locationPort && locationPort !== "4848" && port > 0 && locationPort !== String(port)) {
    return `http://localhost:${port}/api/service`;
  }

  return "/api/service";
}

function initialOperatorIdentity(): string {
  if (typeof window === "undefined") return "";
  return window.localStorage.getItem(OPERATOR_STORAGE_KEY) ?? "";
}

function formatRelativeTime(value?: string | null): string {
  if (!value) return "never";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  const seconds = Math.max(0, Math.round((Date.now() - date.getTime()) / 1000));
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  return date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

function formatEventKind(kind: string): string {
  return kind.replaceAll("_", " ");
}

function countEntries(value?: Record<string, unknown>): number {
  return value ? Object.keys(value).length : 0;
}

function includesQuery(value: string, query: string): boolean {
  return value.toLowerCase().includes(query);
}

function sessionSearchText(session: ServiceSession): string {
  return [
    session.id,
    session.profileId,
    session.serviceName,
    session.agentName,
    session.taskName,
    session.lease,
    session.createdAt,
    session.expiresAt,
    ...(session.browserIds ?? []),
    ...(session.tabIds ?? []),
  ].filter(Boolean).join(" ").toLowerCase();
}

function tabSearchText(tab: ServiceTab): string {
  return [
    tab.id,
    tab.targetId,
    tab.browserId,
    tab.sessionId,
    tab.ownerSessionId,
    tab.lifecycle,
    tab.title,
    tab.url,
    tab.challengeId,
  ].filter(Boolean).join(" ").toLowerCase();
}

function profileAllocationSearchText(allocation: ServiceProfileAllocation): string {
  return [
    allocation.profileId,
    allocation.profileName,
    allocation.allocation,
    allocation.keyring,
    allocation.leaseState,
    allocation.recommendedAction,
    ...(allocation.targetServiceIds ?? []),
    ...(allocation.authenticatedServiceIds ?? []),
    ...(allocation.sharedServiceIds ?? []),
    ...(allocation.holderSessionIds ?? []),
    ...(allocation.exclusiveHolderSessionIds ?? []),
    ...(allocation.waitingJobIds ?? []),
    ...(allocation.conflictSessionIds ?? []),
    ...(allocation.serviceNames ?? []),
    ...(allocation.agentNames ?? []),
    ...(allocation.taskNames ?? []),
    ...(allocation.browserIds ?? []),
    ...(allocation.tabIds ?? []),
    ...(allocation.targetReadiness ?? []).flatMap((readiness) => [
      readiness.targetServiceId,
      readiness.loginId,
      readiness.state,
      readiness.evidence,
      readiness.recommendedAction,
    ]),
  ].filter(Boolean).join(" ").toLowerCase();
}

function incidentSearchText(incident: IncidentRecord): string {
  return [
    incident.id,
    incident.browserId,
    incident.label,
    incident.severity,
    incident.escalation,
    incident.recommendedAction,
    incident.latestMessage,
    incident.latestKind,
    incident.currentHealth,
    incident.acknowledgedBy,
    incident.acknowledgementNote,
    incident.resolvedBy,
    incident.resolutionNote,
    ...incident.serviceEvents.map((event) => event.message),
    ...incident.transitionEvents.map((event) => event.message),
    ...incident.jobEvents.map((event) => event.message),
  ].filter(Boolean).join(" ").toLowerCase();
}

function formatAbsoluteTime(value?: string | null): string {
  if (!value) return "unknown";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString(undefined, {
    dateStyle: "medium",
    timeStyle: "medium",
  });
}

function formatDurationMs(value?: number | null): string {
  if (value === undefined || value === null) return "unknown";
  if (value < 1000) return `${value} ms`;
  const seconds = value / 1000;
  if (seconds < 60) return `${seconds.toFixed(seconds >= 10 ? 0 : 1)} s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = Math.round(seconds % 60);
  return remainingSeconds > 0 ? `${minutes}m ${remainingSeconds}s` : `${minutes}m`;
}

function cleanupTotal(value?: Record<string, number>): number {
  if (!value) return 0;
  if (typeof value.total === "number" && Number.isFinite(value.total)) return value.total;
  return Object.entries(value)
    .filter(([key]) => key !== "total")
    .reduce((sum, [, count]) => sum + (Number.isFinite(count) ? count : 0), 0);
}

function cleanupCountSummary(value?: Record<string, number>): string {
  if (!value) return "none";
  const entries = Object.entries(value).filter(([key, count]) => key !== "total" && count > 0);
  if (entries.length === 0) return "none";
  return entries
    .map(([key, count]) => `${key.replaceAll(/([A-Z])/g, " $1").toLowerCase()}: ${count}`)
    .join(" / ");
}

function formatDetails(value: unknown): string {
  if (value === undefined || value === null) return "";
  if (typeof value === "string") return value;
  return JSON.stringify(value, null, 2);
}

function formatActor(value: unknown): string {
  if (!value) return "unknown";
  if (typeof value === "string") return value;
  if (typeof value !== "object") return String(value);
  const entries = Object.entries(value as Record<string, unknown>);
  if (entries.length === 0) return "unknown";
  const [kind, detail] = entries[0];
  return detail ? `${kind}: ${String(detail)}` : kind;
}

function traceContextLabel(item: {
  serviceName?: string | null;
  agentName?: string | null;
  taskName?: string | null;
  profileId?: string | null;
  sessionId?: string | null;
  browserId?: string | null;
}): string {
  const parts = [
    item.serviceName,
    item.agentName,
    item.taskName,
    item.profileId,
    item.sessionId,
    item.browserId,
  ]
    .filter((value): value is string => typeof value === "string" && value.length > 0)
    .slice(0, 4);
  return parts.length > 0 ? parts.join(" / ") : "No trace context";
}

function healthTone(value?: string): "good" | "warn" | "bad" | "neutral" {
  const normalized = (value ?? "").toLowerCase();
  if (["ready", "cdp_ready", "running"].includes(normalized)) return "good";
  if (["notstarted", "not_started", "idle", ""].includes(normalized)) return "neutral";
  if (["cdp_disconnected", "unreachable", "process_exited"].includes(normalized)) return "bad";
  return "warn";
}

function isActiveServiceJob(job: ServiceJob): boolean {
  const state = (job.state ?? "").toLowerCase();
  return state === "queued" || state === "running";
}

function isRetainedTerminalServiceJob(job: ServiceJob): boolean {
  const state = (job.state ?? "").toLowerCase();
  return ["succeeded", "failed", "timed_out", "cancelled"].includes(state);
}

function isActiveServiceSession(session: ServiceSession): boolean {
  return (session.browserIds?.length ?? 0) > 0 || (session.tabIds?.length ?? 0) > 0;
}

function isActiveServiceTab(tab: ServiceTab): boolean {
  const lifecycle = (tab.lifecycle ?? "").toLowerCase();
  return lifecycle === "ready" || lifecycle === "loading" || lifecycle === "active";
}

function profileAllocationTone(value?: string): "good" | "warn" | "bad" | "neutral" {
  const normalized = (value ?? "").toLowerCase();
  if (normalized === "available" || normalized === "shared") return "good";
  if (normalized === "exclusive" || normalized === "waiting") return "warn";
  if (normalized === "conflicted") return "bad";
  return "neutral";
}

function formatStringList(values?: string[], fallback = "none"): string {
  const normalized = values?.filter((value) => value.trim().length > 0) ?? [];
  return normalized.length > 0 ? normalized.join(", ") : fallback;
}

function formatHealthLabel(value?: string | null): string {
  return value ? value.replaceAll("_", " ") : "unknown";
}

function isBadHealth(value?: string | null): boolean {
  return ["process_exited", "cdp_disconnected", "unreachable"].includes((value ?? "").toLowerCase());
}

function isRecoveryHealth(previous?: string | null, current?: string | null): boolean {
  return isBadHealth(previous) && (current ?? "").toLowerCase() === "ready";
}

function isIncidentEvent(event: ServiceEvent): boolean {
  if (event.kind === "reconciliation_error") return true;
  if (event.kind === "browser_health_changed") {
    return isBadHealth(event.currentHealth) || isRecoveryHealth(event.previousHealth, event.currentHealth);
  }
  return event.kind === "service_job_timeout" || event.kind === "service_job_cancelled";
}

function incidentHandlingState(incident: IncidentRecord): Exclude<IncidentHandlingFilter, "all"> {
  if (incident.resolvedAt) return "resolved";
  if (incident.acknowledgedAt) return "acknowledged";
  return "unacknowledged";
}

function incidentHandlingLabel(incident: IncidentRecord): string {
  const state = incidentHandlingState(incident);
  if (state === "unacknowledged") return "needs ack";
  return state;
}

function deriveIncidentTimeline(incident: IncidentRecord): ServiceTraceTimelineItem[] {
  const eventItems = incident.serviceEvents.map((event) => ({
    id: event.id,
    timestamp: event.timestamp,
    kind: event.kind,
    title:
      event.kind === "browser_health_changed"
        ? `${formatHealthLabel(event.previousHealth)} to ${formatHealthLabel(event.currentHealth)}`
        : formatEventKind(event.kind),
    message: event.message,
  }));
  const jobItems = incident.jobEvents.map((event) => ({
    id: event.id,
    timestamp: event.timestamp,
    kind: event.kind,
    title: formatEventKind(event.kind),
    message: event.message,
  }));
  const operatorItems: ServiceTraceTimelineItem[] = [];
  if (incident.acknowledgedAt) {
    operatorItems.push({
      id: `${incident.id}-acknowledged`,
      timestamp: incident.acknowledgedAt,
      kind: "incident_acknowledged",
      title: `Acknowledged by ${incident.acknowledgedBy || "unknown"}`,
      message: incident.acknowledgementNote,
    });
  }
  if (incident.resolvedAt) {
    operatorItems.push({
      id: `${incident.id}-resolved`,
      timestamp: incident.resolvedAt,
      kind: "incident_resolved",
      title: `Resolved by ${incident.resolvedBy || "unknown"}`,
      message: incident.resolutionNote,
    });
  }
  return [...eventItems, ...jobItems, ...operatorItems].sort(
    (left, right) => new Date(left.timestamp).getTime() - new Date(right.timestamp).getTime(),
  );
}

function browserCapabilityLaunchKey(launch: ServiceTraceBrowserCapabilityLaunch): string {
  return [
    launch.sessionId ?? "",
    launch.browserId ?? "",
    launch.profileId ?? "",
    launch.browserBuild ?? "",
    launch.reason ?? "",
    launch.timestamp ?? "",
  ].join(":");
}

function BrowserCapabilityLaunchCard({ launch }: { launch: ServiceTraceBrowserCapabilityLaunch }) {
  const context = [launch.serviceName, launch.agentName, launch.taskName]
    .filter((value): value is string => typeof value === "string" && value.length > 0)
    .join(" / ");
  const ids = [
    launch.bindingId && `binding ${launch.bindingId}`,
    launch.hostId && `host ${launch.hostId}`,
    launch.executableId && `exec ${launch.executableId}`,
    launch.capabilityId && `capability ${launch.capabilityId}`,
  ].filter((value): value is string => typeof value === "string" && value.length > 0);

  return (
    <div className={cn("service-trace-launch-card", launch.applied && "service-trace-launch-card-applied")}>
      <div className="flex min-w-0 items-center gap-2">
        <span className="truncate text-xs font-black text-foreground">
          {launch.browserBuild ?? "unknown build"}
        </span>
        <Badge
          variant={launch.applied ? "outline" : "destructive"}
          className="ml-auto shrink-0 rounded-full px-1.5 py-0 text-[9px]"
        >
          {launch.applied ? "applied" : "skipped"}
        </Badge>
      </div>
      <p className="mt-1 truncate text-[11px] font-bold text-muted-foreground">
        {launch.reason ?? "no reason recorded"}
      </p>
      {context && (
        <p className="mt-1 truncate text-[10px] text-muted-foreground">
          {context}
        </p>
      )}
      <div className="service-trace-context-meta">
        {launch.sessionId && <span>session {launch.sessionId}</span>}
        {launch.browserId && <span>browser {launch.browserId}</span>}
        {launch.profileId && <span>profile {launch.profileId}</span>}
        <span>source {launch.source}</span>
        {launch.timestamp && <span>{formatRelativeTime(launch.timestamp)}</span>}
      </div>
      {ids.length > 0 && (
        <div className="service-trace-launch-ids">
          {ids.map((item) => (
            <span key={item}>{item}</span>
          ))}
        </div>
      )}
    </div>
  );
}

function deriveJobIncidentEvents(jobs: ServiceJob[]): ServiceEvent[] {
  return jobs
    .filter((job) => job.state === "timed_out" || job.state === "cancelled")
    .map((job) => ({
      id: `job-incident-${job.id}`,
      timestamp: job.completedAt ?? job.startedAt ?? job.submittedAt ?? new Date().toISOString(),
      kind: job.state === "timed_out" ? "service_job_timeout" : "service_job_cancelled",
      message:
        job.state === "timed_out"
          ? `${job.action ?? "Service job"} timed out`
          : `${job.action ?? "Service job"} was cancelled`,
      details: {
        jobId: job.id,
        action: job.action,
        state: job.state,
        submittedAt: job.submittedAt,
        startedAt: job.startedAt,
        completedAt: job.completedAt,
        error: job.error,
      },
    }));
}

function deriveIncidentRecords(
  incidents: ServiceIncident[],
  allEvents: ServiceEvent[],
  allJobs: ServiceJob[],
): IncidentRecord[] {
  const eventById = new Map(allEvents.map((event) => [event.id, event]));
  const jobById = new Map(allJobs.map((job) => [job.id, job]));

  return incidents
    .map((incident) => {
      const serviceEvents = (incident.eventIds ?? [])
        .map((id) => eventById.get(id))
        .filter((event): event is ServiceEvent => !!event)
        .sort((left, right) => new Date(right.timestamp).getTime() - new Date(left.timestamp).getTime());
      const transitionEvents = serviceEvents.filter((event) => event.kind === "browser_health_changed");
      const jobEvents = (incident.jobIds ?? [])
        .map((id) => jobById.get(id))
        .filter((job): job is ServiceJob => !!job)
        .map((job) => ({
          id: `job-incident-${job.id}`,
          timestamp: job.completedAt ?? job.startedAt ?? job.submittedAt ?? "",
          kind: job.state === "timed_out" ? "service_job_timeout" : "service_job_cancelled",
          message:
            job.state === "timed_out"
              ? `${job.action ?? "Service job"} timed out`
              : `${job.action ?? "Service job"} was cancelled`,
          details: {
            jobId: job.id,
            action: job.action,
            state: job.state,
            submittedAt: job.submittedAt,
            startedAt: job.startedAt,
            completedAt: job.completedAt,
            error: job.error,
          },
        }))
        .sort((left, right) => new Date(right.timestamp).getTime() - new Date(left.timestamp).getTime());

      return {
        id: incident.id,
        browserId: incident.browserId,
        label: incident.label,
        severity: incident.severity,
        escalation: incident.escalation,
        recommendedAction: incident.recommendedAction,
        latestTimestamp: incident.latestTimestamp,
        latestMessage: incident.latestMessage,
        latestKind: incident.latestKind,
        currentHealth: incident.currentHealth,
        acknowledgedAt: incident.acknowledgedAt,
        acknowledgedBy: incident.acknowledgedBy,
        acknowledgementNote: incident.acknowledgementNote,
        resolvedAt: incident.resolvedAt,
        resolvedBy: incident.resolvedBy,
        resolutionNote: incident.resolutionNote,
        serviceEvents,
        transitionEvents,
        jobEvents,
      };
    })
    .sort(
      (left, right) =>
        new Date(right.latestTimestamp).getTime() - new Date(left.latestTimestamp).getTime(),
    );
}

type ServiceStatusTone = "good" | "warn" | "bad" | "neutral";

function ServiceStatusLight({
  label,
  value,
  detail,
  icon: Icon,
  tone = "neutral",
}: {
  label: string;
  value: string;
  detail: string;
  icon: typeof ServerCog;
  tone?: ServiceStatusTone;
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <div
          className={cn("service-status-light", `service-health-${tone}`)}
          role="status"
          tabIndex={0}
          aria-label={`${label}: ${value}. ${detail}`}
        >
          <span className="service-health-icon">
            <Icon className="size-3.5" />
          </span>
          <span className="min-w-0">
            <span className="service-status-light-label">{label}</span>
            <span className="service-status-light-value">{value}</span>
          </span>
        </div>
      </TooltipTrigger>
      <TooltipContent>
        <p>{detail}</p>
      </TooltipContent>
    </Tooltip>
  );
}

function EventDot({ kind }: { kind: string }) {
  const isError = kind === "reconciliation_error";
  const isHealth = kind === "browser_health_changed";
  const isTab = kind === "tab_lifecycle_changed";
  const isJobIncident = kind === "service_job_timeout" || kind === "service_job_cancelled";
  const isOperatorIncident = kind === "incident_acknowledged" || kind === "incident_resolved";
  return (
    <span
      className={cn(
        "service-event-dot",
        (isError || isJobIncident) && "service-event-dot-error",
        (isHealth || isOperatorIncident) && "service-event-dot-health",
        isTab && "service-event-dot-tab",
      )}
    />
  );
}

function EventRow({ event, onSelect }: { event: ServiceEvent; onSelect: (event: ServiceEvent) => void }) {
  return (
    <button
      type="button"
      className="service-event-row"
      onClick={() => onSelect(event)}
      aria-label={`Inspect ${formatEventKind(event.kind)} event`}
    >
      <EventDot kind={event.kind} />
      <div className="min-w-0 flex-1">
        <div className="flex min-w-0 items-center gap-2">
          <span className="truncate text-xs font-bold text-foreground">
            {formatEventKind(event.kind)}
          </span>
          {event.browserId && (
            <Badge variant="outline" className="h-4 max-w-28 truncate px-1.5 text-[9px]">
              {event.browserId}
            </Badge>
          )}
          <span className="ml-auto shrink-0 text-[10px] text-muted-foreground">
            {formatRelativeTime(event.timestamp)}
          </span>
        </div>
        <p className="mt-1 line-clamp-2 text-xs leading-5 text-muted-foreground">
          {event.message || "No message"}
        </p>
      </div>
    </button>
  );
}

function HealthTransitionTimeline({
  events,
  onSelect,
}: {
  events: ServiceEvent[];
  onSelect: (event: ServiceEvent) => void;
}) {
  return (
    <div className="service-health-timeline-card">
      <div className="flex items-center gap-2 px-1">
        <RadioTower className="size-4 text-muted-foreground" />
        <div className="min-w-0">
          <p className="text-xs font-black uppercase tracking-[0.18em] text-muted-foreground">
            Browser health timeline
          </p>
          <p className="truncate text-[11px] text-muted-foreground">
            Crash and recovery transitions from retained service events
          </p>
        </div>
      </div>
      <div className="service-health-timeline">
        {events.length === 0 ? (
          <p className="rounded-2xl bg-foreground/[0.04] px-3 py-5 text-center text-xs text-muted-foreground">
            No browser health transitions recorded yet.
          </p>
        ) : (
          events.map((event) => {
            const tone = healthTone(event.currentHealth ?? undefined);
            return (
              <button
                key={event.id}
                type="button"
                className={cn("service-health-transition", `service-health-transition-${tone}`)}
                onClick={() => onSelect(event)}
                aria-label={`Inspect health transition for ${event.browserId ?? "browser"}`}
              >
                <span className="service-health-transition-rail" />
                <span className="service-health-transition-node" />
                <span className="min-w-0 flex-1">
                  <span className="flex min-w-0 items-center gap-2">
                    <span className="truncate text-xs font-black text-foreground">
                      {event.browserId ?? "browser"}
                    </span>
                    <Badge variant="outline" className="h-4 shrink-0 px-1.5 text-[9px]">
                      {formatRelativeTime(event.timestamp)}
                    </Badge>
                  </span>
                  <span className="mt-1 flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground">
                    <span className="truncate">{formatHealthLabel(event.previousHealth)}</span>
                    <span className="text-[10px] font-black text-muted-foreground">to</span>
                    <span className={cn("truncate font-black", tone === "bad" && "text-destructive", tone === "good" && "text-success", tone === "warn" && "text-warning")}>
                      {formatHealthLabel(event.currentHealth)}
                    </span>
                  </span>
                  {event.message && (
                    <span className="mt-1 block truncate text-[11px] text-muted-foreground">
                      {event.message}
                    </span>
                  )}
                </span>
              </button>
            );
          })
        )}
      </div>
    </div>
  );
}

function TraceExplorer({
  filters,
  trace,
  loading,
  error,
  timeline,
  onFiltersChange,
  onLoad,
  onClear,
}: {
  filters: TraceFilters;
  trace: ServiceTraceData | null;
  loading: boolean;
  error: string;
  timeline: ServiceTraceTimelineItem[];
  onFiltersChange: (filters: TraceFilters) => void;
  onLoad: () => void;
  onClear: () => void;
}) {
  const counts = trace?.counts;
  const matched = trace?.matched;
  const summaryCards = traceSummaryCards(trace);
  const browserCapabilityLaunches = traceBrowserCapabilityLaunches(trace);
  const browserCapabilityLaunchSummary = trace?.summary?.browserCapabilityLaunches;
  const profileLeaseWaits = traceProfileLeaseWaits(trace);
  const profileLeaseWaitSummary = trace?.summary?.profileLeaseWaits;
  const hasFilters =
    !!filters.serviceName.trim() ||
    !!filters.agentName.trim() ||
    !!filters.taskName.trim() ||
    !!filters.browserId.trim() ||
    !!filters.profileId.trim() ||
    !!filters.sessionId.trim() ||
    !!filters.since.trim();

  return (
    <div className="service-trace-card">
      <div className="flex items-center gap-2">
        <History className="size-4 text-muted-foreground" />
        <div className="min-w-0">
          <p className="text-xs font-black uppercase tracking-[0.18em] text-muted-foreground">
            Trace explorer
          </p>
          <p className="truncate text-[11px] text-muted-foreground">
            Combined events, jobs, incidents, and activity from the service trace API
          </p>
        </div>
        {loading && <Loader2 className="ml-auto size-3.5 animate-spin text-muted-foreground" />}
      </div>
      <div className="service-trace-grid" aria-label="Trace filters">
        <input
          aria-label="Trace service name"
          className="service-filter-input service-trace-input"
          placeholder="service name"
          value={filters.serviceName}
          onChange={(event) => onFiltersChange({ ...filters, serviceName: event.target.value })}
        />
        <input
          aria-label="Trace task name"
          className="service-filter-input service-trace-input"
          placeholder="task name"
          value={filters.taskName}
          onChange={(event) => onFiltersChange({ ...filters, taskName: event.target.value })}
        />
        <input
          aria-label="Trace agent name"
          className="service-filter-input service-trace-input"
          placeholder="agent name"
          value={filters.agentName}
          onChange={(event) => onFiltersChange({ ...filters, agentName: event.target.value })}
        />
        <input
          aria-label="Trace browser ID"
          className="service-filter-input service-trace-input"
          placeholder="browser id"
          value={filters.browserId}
          onChange={(event) => onFiltersChange({ ...filters, browserId: event.target.value })}
        />
        <input
          aria-label="Trace profile ID"
          className="service-filter-input service-trace-input"
          placeholder="profile id"
          value={filters.profileId}
          onChange={(event) => onFiltersChange({ ...filters, profileId: event.target.value })}
        />
        <input
          aria-label="Trace session ID"
          className="service-filter-input service-trace-input"
          placeholder="session id"
          value={filters.sessionId}
          onChange={(event) => onFiltersChange({ ...filters, sessionId: event.target.value })}
        />
        <input
          aria-label="Trace since timestamp"
          className="service-filter-input service-trace-input"
          placeholder="since RFC 3339"
          value={filters.since}
          onChange={(event) => onFiltersChange({ ...filters, since: event.target.value })}
        />
      </div>
      <div className="service-filter-bar">
        <div className="service-filter-group">
          {EVENT_LIMIT_OPTIONS.map((limit) => (
            <button
              key={limit}
              type="button"
              className={cn("service-filter-chip", filters.limit === limit && "service-filter-chip-active")}
              onClick={() => onFiltersChange({ ...filters, limit })}
            >
              {limit}
            </button>
          ))}
        </div>
        <Button
          type="button"
          size="sm"
          className="rounded-full"
          onClick={onLoad}
          disabled={loading || !hasFilters}
        >
          {loading ? <Loader2 className="size-3.5 animate-spin" /> : <History className="size-3.5" />}
          Load trace
        </Button>
        {(trace || hasFilters) && (
          <button type="button" className="service-filter-reset" onClick={onClear}>
            Clear trace
          </button>
        )}
      </div>
      {error && (
        <div className="service-browser-error">
          <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
          <span>{error}</span>
        </div>
      )}
      {trace && (
        <div className="service-trace-counts">
          <div>
            <strong>{counts?.events ?? 0}</strong>
            <span>events</span>
          </div>
          <div>
            <strong>{counts?.jobs ?? 0}</strong>
            <span>jobs</span>
          </div>
          <div>
            <strong>{counts?.incidents ?? 0}</strong>
            <span>incidents</span>
          </div>
          <div>
            <strong>{counts?.activity ?? 0}</strong>
            <span>activity</span>
          </div>
        </div>
      )}
      {trace && matched && (
        <p className="px-1 text-[10px] leading-4 text-muted-foreground">
          Matched {matched.events ?? 0} events, {matched.jobs ?? 0} jobs, {matched.incidents ?? 0} incidents,
          and {matched.activity ?? 0} activity entries before per-section limits.
        </p>
      )}
      {trace && (
        <p className="px-1 text-[10px] leading-4 text-muted-foreground">
          Contract data: events/jobs/incidents/activity. Returned filters: {traceFilterSummary(trace.filters)}
        </p>
      )}
      {trace && (
        <div className="service-trace-contexts" aria-label="Trace ownership summary">
          <div className="service-trace-contexts-header">
            <span>Ownership summary</span>
            <Badge variant="outline" className="rounded-full px-2 py-0 text-[9px] uppercase">
              {trace.summary?.contextCount ?? summaryCards.length} contexts
            </Badge>
            {(trace.summary?.namingWarningCount ?? 0) > 0 && (
              <Badge variant="destructive" className="rounded-full px-2 py-0 text-[9px] uppercase">
                {trace.summary?.namingWarningCount} naming warnings
              </Badge>
            )}
          </div>
          {summaryCards.length === 0 ? (
            <p className="rounded-2xl bg-foreground/[0.04] px-3 py-4 text-center text-xs text-muted-foreground">
              No ownership context was returned for this trace.
            </p>
          ) : (
            <div className="service-trace-context-grid">
              {summaryCards.map((card) => (
                <div key={card.key} className="service-trace-context-card">
                  <div className="flex min-w-0 items-center gap-2">
                    <span className="truncate text-xs font-black text-foreground">
                      {card.title}
                    </span>
                    <Badge variant="outline" className="ml-auto shrink-0 rounded-full px-1.5 py-0 text-[9px]">
                      {card.total} records
                    </Badge>
                  </div>
                  <p className="mt-1 truncate text-[11px] font-bold text-muted-foreground">
                    {card.subtitle}
                  </p>
                  {card.warning && (
                    <p className="mt-1 rounded-full bg-warning/10 px-2 py-1 text-[10px] font-black uppercase tracking-[0.12em] text-warning">
                      {card.warning}
                    </p>
                  )}
                  <div className="service-trace-context-meta">
                    {card.meta.map((item) => (
                      <span key={item}>{item}</span>
                    ))}
                  </div>
                  {card.targetServiceIds.length > 0 && (
                    <div className="service-trace-targets" aria-label={`${card.title} target identities`}>
                      {card.targetServiceIds.map((target) => (
                        <span key={target}>{target}</span>
                      ))}
                    </div>
                  )}
                  <div className="service-trace-context-counts">
                    {card.counts.map((item) => (
                      <span key={item}>{item}</span>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
      {trace && (
        <div className="service-trace-launches" aria-label="Browser capability launch decisions">
          <div className="service-trace-contexts-header">
            <span>Browser capability launches</span>
            <Badge variant="outline" className="rounded-full px-2 py-0 text-[9px] uppercase">
              {browserCapabilityLaunchSummary?.count ?? browserCapabilityLaunches.length} decisions
            </Badge>
            {(browserCapabilityLaunchSummary?.skippedCount ?? 0) > 0 && (
              <Badge variant="destructive" className="rounded-full px-2 py-0 text-[9px] uppercase">
                {browserCapabilityLaunchSummary?.skippedCount} skipped
              </Badge>
            )}
          </div>
          {browserCapabilityLaunches.length === 0 ? (
            <p className="rounded-2xl bg-foreground/[0.04] px-3 py-4 text-center text-xs text-muted-foreground">
              No browser binding decision was returned for this trace.
            </p>
          ) : (
            <div className="service-trace-launch-grid">
              {browserCapabilityLaunches.map((launch) => (
                <BrowserCapabilityLaunchCard key={browserCapabilityLaunchKey(launch)} launch={launch} />
              ))}
            </div>
          )}
        </div>
      )}
      {trace && (
        <div className="service-trace-waits" aria-label="Profile lease wait summary">
          <div className="service-trace-contexts-header">
            <span>Profile lease waits</span>
            <Badge variant="outline" className="rounded-full px-2 py-0 text-[9px] uppercase">
              {profileLeaseWaitSummary?.count ?? profileLeaseWaits.length} waits
            </Badge>
            {(profileLeaseWaitSummary?.activeCount ?? 0) > 0 && (
              <Badge variant="destructive" className="rounded-full px-2 py-0 text-[9px] uppercase">
                {profileLeaseWaitSummary?.activeCount} active
              </Badge>
            )}
          </div>
          {profileLeaseWaits.length === 0 ? (
            <p className="rounded-2xl bg-foreground/[0.04] px-3 py-4 text-center text-xs text-muted-foreground">
              No profile lease contention was returned for this trace.
            </p>
          ) : (
            <div className="service-trace-wait-grid">
              {profileLeaseWaits.map((wait) => {
                const timestamp = wait.endedAt ?? wait.startedAt;
                const active = !wait.endedAt;
                const conflictCount = wait.conflictSessionIds?.length ?? 0;
                return (
                  <div
                    key={`${wait.jobId}:${wait.startedAt ?? ""}:${wait.endedAt ?? ""}`}
                    className={cn("service-trace-wait-card", active && "service-trace-wait-card-active")}
                  >
                    <div className="flex min-w-0 items-center gap-2">
                      <span className="truncate text-xs font-black text-foreground">
                        {wait.profileId ?? "unknown profile"}
                      </span>
                      <Badge
                        variant={active ? "destructive" : "outline"}
                        className="ml-auto shrink-0 rounded-full px-1.5 py-0 text-[9px]"
                      >
                        {active ? "waiting" : wait.outcome}
                      </Badge>
                    </div>
                    <p className="mt-1 truncate text-[11px] font-bold text-muted-foreground">
                      {wait.serviceName ?? "unlabeled service"} / {wait.taskName ?? "untitled task"}
                    </p>
                    <div className="service-trace-wait-metrics">
                      <span>{formatDurationMs(wait.waitedMs)} waited</span>
                      <span>{conflictCount} conflicts</span>
                      {wait.retryAfterMs !== undefined && wait.retryAfterMs !== null && (
                        <span>{formatDurationMs(wait.retryAfterMs)} retry</span>
                      )}
                    </div>
                    <div className="service-trace-context-meta">
                      <span>job {wait.jobId}</span>
                      {wait.agentName && <span>agent {wait.agentName}</span>}
                      {timestamp && <span>{formatRelativeTime(timestamp)}</span>}
                    </div>
                    {conflictCount > 0 && (
                      <p className="mt-2 truncate text-[10px] text-muted-foreground">
                        Conflicts: {wait.conflictSessionIds?.join(", ")}
                      </p>
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </div>
      )}
      {trace && (
        <div className="service-trace-timeline">
          {timeline.length === 0 ? (
            <p className="rounded-2xl bg-foreground/[0.04] px-3 py-5 text-center text-xs text-muted-foreground">
              No trace records matched these filters.
            </p>
          ) : (
            timeline.map((item) => (
              <div key={item.id} className="service-incident-history-item">
                <EventDot kind={item.kind} />
                <div className="min-w-0 flex-1">
                  <div className="flex min-w-0 items-center gap-2">
                    <span className="truncate text-xs font-bold text-foreground">
                      {item.title || formatEventKind(item.kind)}
                    </span>
                    {item.source && (
                      <Badge variant="outline" className="rounded-full px-1.5 py-0 text-[9px] uppercase">
                        {item.source}
                      </Badge>
                    )}
                    <span className="ml-auto shrink-0 text-[10px] text-muted-foreground">
                      {formatAbsoluteTime(item.timestamp)}
                    </span>
                  </div>
                  <p className="mt-1 truncate text-[10px] text-muted-foreground">
                    {traceContextLabel(item)}
                  </p>
                  {item.message && (
                    <p className="mt-1 text-xs leading-5 text-muted-foreground">
                      {item.message}
                    </p>
                  )}
                </div>
              </div>
            ))
          )}
        </div>
      )}
      {!trace && !error && (
        <p className="rounded-2xl bg-foreground/[0.04] px-3 py-5 text-center text-xs text-muted-foreground">
          Enter a service, task, agent, browser, profile, or session filter to load one combined trace.
        </p>
      )}
    </div>
  );
}

function IncidentRow({
  incident,
  onSelect,
}: {
  incident: IncidentRecord;
  onSelect: (incident: IncidentRecord) => void;
}) {
  const tone = healthTone(incident.currentHealth ?? undefined);
  const priority = incidentPriorityView(incident);
  const incidentCount = incident.transitionEvents.length + incident.jobEvents.length;
  const handlingState = incidentHandlingState(incident);
  return (
    <button
      type="button"
      className="service-browser-row"
      onClick={() => onSelect(incident)}
      aria-label={priority.ariaLabel}
    >
      <span className={cn("service-browser-health-dot", `service-browser-health-${tone}`)} />
      <div className="min-w-0 flex-1">
        <div className="flex min-w-0 items-center gap-2">
          <span className="truncate text-xs font-bold text-foreground">{incident.label}</span>
          <Badge
            variant="outline"
            className={cn(
              "h-4 shrink-0 px-1.5 text-[9px]",
              `service-incident-severity-${priority.severityTone}`,
            )}
          >
            {priority.severityLabel}
          </Badge>
          <Badge variant="outline" className="h-4 max-w-32 truncate px-1.5 text-[9px]">
            {priority.escalationLabel}
          </Badge>
          <Badge variant="outline" className="h-4 max-w-28 truncate px-1.5 text-[9px]">
            {incidentCount} incidents
          </Badge>
          <Badge
            variant="outline"
            className={cn(
              "h-4 shrink-0 px-1.5 text-[9px]",
              handlingState === "unacknowledged" && "service-incident-badge-unacknowledged",
              handlingState === "acknowledged" && "service-incident-badge-acknowledged",
              handlingState === "resolved" && "service-incident-badge-resolved",
            )}
          >
            {incidentHandlingLabel(incident)}
          </Badge>
        </div>
        <p className="mt-1 truncate text-xs text-muted-foreground">
          {incident.latestMessage}
        </p>
        {priority.recommendedAction && (
          <p className="mt-1 truncate text-[11px] font-medium text-muted-foreground">
            Recommended: {priority.recommendedAction}
          </p>
        )}
      </div>
      <span className="text-[10px] font-bold text-muted-foreground">
        {formatRelativeTime(incident.latestTimestamp)}
      </span>
    </button>
  );
}

function IncidentSummaryGroupRow({ group }: { group: ServiceIncidentSummaryGroupView }) {
  return (
    <div className={cn("service-incident-summary-group", `service-incident-priority-${group.severityTone}`)}>
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-1.5">
          <Badge
            variant="outline"
            className={cn(
              "h-4 shrink-0 px-1.5 text-[9px]",
              `service-incident-severity-${group.severityTone}`,
            )}
          >
            {group.severityLabel}
          </Badge>
          <Badge variant="outline" className="h-4 max-w-36 truncate px-1.5 text-[9px]">
            {group.escalationLabel}
          </Badge>
          <Badge variant="outline" className="h-4 shrink-0 px-1.5 text-[9px]">
            {group.stateLabel}
          </Badge>
        </div>
        <p className="mt-2 text-xs font-semibold leading-5 text-foreground">
          {group.recommendedAction}
        </p>
        <p className="mt-1 truncate text-[10px] text-muted-foreground">
          incidents {group.incidentIdLabel}
        </p>
        <p className="mt-1 truncate text-[10px] text-muted-foreground">
          browsers {group.browserIdLabel}
        </p>
        {group.monitorIds.length > 0 && (
          <p className="mt-1 truncate text-[10px] text-muted-foreground">
            monitors {group.monitorIdLabel}
          </p>
        )}
        {group.remedyApplyCommand && (
          <p className="mt-1 truncate text-[10px] font-semibold text-foreground">
            {group.remedyApplyCommand}
          </p>
        )}
      </div>
      <div className="shrink-0 text-right">
        <p className="text-xl font-black tracking-[-0.05em] text-foreground">{group.count}</p>
        <p className="text-[10px] font-bold text-muted-foreground">
          latest {formatRelativeTime(group.latestTimestamp)}
        </p>
      </div>
    </div>
  );
}

function JobRow({ job, onSelect }: { job: ServiceJob; onSelect: (job: ServiceJob) => void }) {
  const failed = job.state === "failed" || job.state === "timed_out" || job.state === "cancelled";
  const namingWarning = serviceJobNamingWarningLabel(job.namingWarnings);
  return (
    <button
      type="button"
      className="service-event-row"
      onClick={() => onSelect(job)}
      aria-label={`Inspect job ${job.id}`}
    >
      <span
        className={cn(
          "service-event-dot",
          job.state === "succeeded" && "service-event-dot-health",
          failed && "service-event-dot-error",
        )}
      />
      <div className="min-w-0 flex-1">
        <div className="flex min-w-0 items-center gap-2">
          <span className="truncate text-xs font-bold text-foreground">
            {job.action ?? "unknown action"}
          </span>
          <Badge variant="outline" className="h-4 max-w-28 truncate px-1.5 text-[9px]">
            {job.state ?? "unknown"}
          </Badge>
          {namingWarning && (
            <Badge variant="destructive" className="h-4 max-w-32 truncate px-1.5 text-[9px]">
              {namingWarning}
            </Badge>
          )}
          <span className="ml-auto shrink-0 text-[10px] text-muted-foreground">
            {formatRelativeTime(job.submittedAt)}
          </span>
        </div>
        <p className="mt-1 line-clamp-2 text-xs leading-5 text-muted-foreground">
          {job.error || job.id || "No job details"}
        </p>
      </div>
    </button>
  );
}

function serviceJobNamingWarningLabel(warnings?: string[]): string | null {
  if (!warnings || warnings.length === 0) return null;
  const labels = warnings.map((warning) => {
    if (warning === "missing_service_name") return "service";
    if (warning === "missing_agent_name") return "agent";
    if (warning === "missing_task_name") return "task";
    return warning.replaceAll("_", " ");
  });
  return `Missing ${labels.join(", ")}`;
}

function EventDetailItem({ label, value }: { label: string; value?: string | null }) {
  if (!value) return null;
  return (
    <div className="service-event-detail-item">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function ViewStreamCard({
  stream,
  onInspect,
}: {
  stream: ServiceViewStream;
  onInspect?: (stream: ServiceViewStream) => void;
}) {
  const embeddable = canEmbedViewStream(stream);
  return (
    <div className="overflow-hidden rounded-2xl border border-border bg-card/70">
      <div className="flex flex-wrap items-center gap-2 border-b border-border bg-muted/40 px-3 py-2">
        <RadioTower className="size-3.5 text-muted-foreground" />
        <span className="text-xs font-black uppercase tracking-[0.16em] text-foreground">
          {viewStreamLabel(stream)}
        </span>
        {stream.id && (
          <Badge variant="outline" className="h-5 max-w-44 truncate px-1.5 text-[9px]">
            {stream.id}
          </Badge>
        )}
        <Badge variant={stream.readOnly ? "secondary" : "default"} className="h-5 px-1.5 text-[9px]">
          {stream.readOnly ? "view only" : "interactive"}
        </Badge>
        {embeddable && onInspect && (
          <Button size="sm" variant="default" className="ml-auto h-7 gap-1.5 px-2 text-[10px]" onClick={() => onInspect(stream)}>
            <Eye className="size-3" />
            Inspect
          </Button>
        )}
        {stream.url && (
          <Button size="sm" variant="outline" className={cn("h-7 gap-1.5 px-2 text-[10px]", !embeddable || !onInspect ? "ml-auto" : "")} asChild>
            <a href={stream.url} target="_blank" rel="noreferrer">
              <ExternalLink className="size-3" />
              Open
            </a>
          </Button>
        )}
      </div>
      {embeddable ? (
        <iframe
          title={`${viewStreamLabel(stream)} ${stream.id ?? ""}`.trim()}
          src={stream.url ?? undefined}
          className="h-[420px] w-full bg-black"
          sandbox="allow-same-origin allow-scripts allow-forms allow-pointer-lock allow-popups"
        />
      ) : (
        <div className="p-3">
          <pre className="service-event-details-json">{formatDetails(stream)}</pre>
        </div>
      )}
    </div>
  );
}

function browserPrimaryViewStream(browser?: ServiceBrowser | null): ServiceViewStream | null {
  return browser?.viewStreams?.find(canEmbedViewStream) ?? browser?.viewStreams?.[0] ?? null;
}

type BrowserSortKey = "health" | "id" | "profile" | "host" | "sessions" | "streams";
type SortDirection = "asc" | "desc";
type BrowserLifecycleFilter = "actionable" | "all" | "live" | "retained";
type BrowserTableColumnKey = "health" | "profile" | "host" | "sessions" | "streams" | "lastError";
type BrowserTableColumnId = BrowserTableColumnKey | "id" | "actions";
type BrowserTableDensity = "compact" | "standard" | "expanded";

const BROWSER_SORT_LABELS: Record<BrowserSortKey, string> = {
  health: "Health",
  id: "Browser",
  profile: "Profile",
  host: "Host",
  sessions: "Sessions",
  streams: "Streams",
};

const BROWSER_TABLE_COLUMNS: { key: BrowserTableColumnKey; label: string }[] = [
  { key: "health", label: "Health" },
  { key: "profile", label: "Profile" },
  { key: "host", label: "Host" },
  { key: "sessions", label: "Sessions" },
  { key: "streams", label: "Streams" },
  { key: "lastError", label: "Last error" },
];

const DEFAULT_BROWSER_TABLE_COLUMNS = BROWSER_TABLE_COLUMNS.map((column) => column.key);
const BROWSER_TABLE_LIFECYCLE_FILTER_STORAGE_KEY = "agent-browser-dashboard-browser-table-lifecycle-filter";
const BROWSER_TABLE_VISIBLE_COLUMNS_STORAGE_KEY = "agent-browser-dashboard-browser-table-visible-columns";
const BROWSER_TABLE_COLUMN_WIDTHS_STORAGE_KEY = "agent-browser-dashboard-browser-table-column-widths";
const BROWSER_TABLE_DENSITY_STORAGE_KEY = "agent-browser-dashboard-browser-table-density";
const BROWSER_TABLE_MIN_COLUMN_WIDTH = 72;
const BROWSER_TABLE_MAX_COLUMN_WIDTH = 420;
const DEFAULT_BROWSER_TABLE_COLUMN_WIDTHS: Record<BrowserTableColumnId, number> = {
  health: 132,
  id: 220,
  profile: 180,
  host: 190,
  sessions: 98,
  streams: 92,
  lastError: 260,
  actions: 108,
};
const BROWSER_TABLE_VIEW_STORAGE_KEYS = [
  BROWSER_TABLE_LIFECYCLE_FILTER_STORAGE_KEY,
  BROWSER_TABLE_VISIBLE_COLUMNS_STORAGE_KEY,
  BROWSER_TABLE_COLUMN_WIDTHS_STORAGE_KEY,
  BROWSER_TABLE_DENSITY_STORAGE_KEY,
];

function isBrowserLifecycleFilter(value: string | null): value is BrowserLifecycleFilter {
  return value === "actionable" || value === "all" || value === "live" || value === "retained";
}

function isBrowserTableColumnKey(value: unknown): value is BrowserTableColumnKey {
  return typeof value === "string" && BROWSER_TABLE_COLUMNS.some((column) => column.key === value);
}

function isBrowserTableColumnId(value: unknown): value is BrowserTableColumnId {
  return typeof value === "string" && value in DEFAULT_BROWSER_TABLE_COLUMN_WIDTHS;
}

function isBrowserTableDensity(value: string | null): value is BrowserTableDensity {
  return value === "compact" || value === "standard" || value === "expanded";
}

function clampBrowserTableColumnWidth(value: unknown): number | null {
  if (typeof value !== "number" || !Number.isFinite(value)) return null;
  return Math.max(BROWSER_TABLE_MIN_COLUMN_WIDTH, Math.min(BROWSER_TABLE_MAX_COLUMN_WIDTH, Math.round(value)));
}

function initialBrowserLifecycleFilter(): BrowserLifecycleFilter {
  if (typeof window === "undefined") return "actionable";
  try {
    const stored = window.localStorage.getItem(BROWSER_TABLE_LIFECYCLE_FILTER_STORAGE_KEY);
    return isBrowserLifecycleFilter(stored) ? stored : "actionable";
  } catch {
    return "actionable";
  }
}

function initialBrowserTableColumns(): BrowserTableColumnKey[] {
  if (typeof window === "undefined") return DEFAULT_BROWSER_TABLE_COLUMNS;
  try {
    const stored = window.localStorage.getItem(BROWSER_TABLE_VISIBLE_COLUMNS_STORAGE_KEY);
    if (!stored) return DEFAULT_BROWSER_TABLE_COLUMNS;
    const parsed = JSON.parse(stored);
    if (!Array.isArray(parsed)) return DEFAULT_BROWSER_TABLE_COLUMNS;
    const columns = parsed.filter(isBrowserTableColumnKey);
    return columns.length > 0 ? columns : DEFAULT_BROWSER_TABLE_COLUMNS;
  } catch {
    return DEFAULT_BROWSER_TABLE_COLUMNS;
  }
}

function initialBrowserTableColumnWidths(): Record<BrowserTableColumnId, number> {
  if (typeof window === "undefined") return DEFAULT_BROWSER_TABLE_COLUMN_WIDTHS;
  try {
    const stored = window.localStorage.getItem(BROWSER_TABLE_COLUMN_WIDTHS_STORAGE_KEY);
    if (!stored) return DEFAULT_BROWSER_TABLE_COLUMN_WIDTHS;
    const parsed = JSON.parse(stored);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return DEFAULT_BROWSER_TABLE_COLUMN_WIDTHS;
    const widths = { ...DEFAULT_BROWSER_TABLE_COLUMN_WIDTHS };
    for (const [key, value] of Object.entries(parsed)) {
      if (!isBrowserTableColumnId(key)) continue;
      const width = clampBrowserTableColumnWidth(value);
      if (width) widths[key] = width;
    }
    return widths;
  } catch {
    return DEFAULT_BROWSER_TABLE_COLUMN_WIDTHS;
  }
}

function initialBrowserTableDensity(): BrowserTableDensity {
  if (typeof window === "undefined") return "standard";
  try {
    const stored = window.localStorage.getItem(BROWSER_TABLE_DENSITY_STORAGE_KEY);
    return isBrowserTableDensity(stored) ? stored : "standard";
  } catch {
    return "standard";
  }
}

function browserSearchText(browser: ServiceBrowser): string {
  return [
    browser.id,
    browser.profileId,
    browser.host,
    browser.health,
    browser.pid ? `pid ${browser.pid}` : "retained",
    browser.cdpEndpoint,
    browser.lastError,
    ...(browser.activeSessionIds ?? []),
  ].filter(Boolean).join(" ").toLowerCase();
}

function browserSortValue(browser: ServiceBrowser, sortKey: BrowserSortKey): string | number {
  if (sortKey === "health") return browser.health ?? "";
  if (sortKey === "id") return browser.id ?? "";
  if (sortKey === "profile") return browser.profileId ?? "";
  if (sortKey === "host") return browser.host ?? "";
  if (sortKey === "sessions") return browser.activeSessionIds?.length ?? 0;
  return browser.viewStreams?.length ?? 0;
}

function compareBrowserValues(left: string | number, right: string | number): number {
  if (typeof left === "number" && typeof right === "number") return left - right;
  return String(left).localeCompare(String(right), undefined, { numeric: true, sensitivity: "base" });
}

function isLiveBrowserRecord(browser: ServiceBrowser): boolean {
  return Boolean(
    browser.pid ||
      browser.cdpEndpoint ||
      (browser.activeSessionIds?.length ?? 0) > 0 ||
      (browser.viewStreams?.length ?? 0) > 0 ||
      ["launching", "ready", "degraded", "cdp_disconnected", "reconnecting", "closing", "faulted"].includes(browser.health ?? ""),
  );
}

function isInertRetainedBrowserRecord(browser: ServiceBrowser): boolean {
  return (
    !browser.pid &&
    !browser.cdpEndpoint &&
    (browser.activeSessionIds?.length ?? 0) === 0 &&
    (browser.viewStreams?.length ?? 0) === 0 &&
    !browser.lastError &&
    (browser.health ?? "not_started") === "not_started"
  );
}

function browserMatchesLifecycleFilter(browser: ServiceBrowser, filter: BrowserLifecycleFilter): boolean {
  if (filter === "all") return true;
  if (filter === "live") return isLiveBrowserRecord(browser);
  if (filter === "retained") return !isLiveBrowserRecord(browser);
  return !isInertRetainedBrowserRecord(browser);
}

function browserDefaultRank(browser: ServiceBrowser): number {
  if ((browser.health ?? "") === "faulted") return 0;
  if (["degraded", "cdp_disconnected", "unreachable", "process_exited"].includes(browser.health ?? "")) return 1;
  if (isLiveBrowserRecord(browser)) return 2;
  if (!isInertRetainedBrowserRecord(browser)) return 3;
  return 4;
}

function BrowserSortButton({
  sortKey,
  activeSortKey,
  direction,
  onSort,
}: {
  sortKey: BrowserSortKey;
  activeSortKey: BrowserSortKey;
  direction: SortDirection;
  onSort: (sortKey: BrowserSortKey) => void;
}) {
  const active = sortKey === activeSortKey;
  return (
    <button
      type="button"
      className={cn("service-browser-table-sort", active && "service-browser-table-sort-active")}
      onClick={() => onSort(sortKey)}
      aria-label={`Sort browsers by ${BROWSER_SORT_LABELS[sortKey]}`}
    >
      {BROWSER_SORT_LABELS[sortKey]}
      <span aria-hidden="true">{active ? (direction === "asc" ? "↑" : "↓") : "↕"}</span>
    </button>
  );
}

function BrowserTableHeaderCell({
  column,
  width,
  onResizeStart,
  onResetWidth,
  children,
  label,
}: {
  column: BrowserTableColumnId;
  width: number;
  onResizeStart: (column: BrowserTableColumnId, event: ReactMouseEvent<HTMLButtonElement>) => void;
  onResetWidth: (column: BrowserTableColumnId) => void;
  children?: ReactNode;
  label: string;
}) {
  return (
    <th style={{ width }}>
      <div className="service-browser-table-header">
        {children}
        <button
          type="button"
          className="service-browser-table-resize"
          aria-label={`Resize ${label} column`}
          onMouseDown={(event) => onResizeStart(column, event)}
          onDoubleClick={() => onResetWidth(column)}
        />
      </div>
    </th>
  );
}

function BrowserTable({
  browsers,
  onSelect,
  selectedBrowserId,
}: {
  browsers: ServiceBrowser[];
  onSelect: (browser: ServiceBrowser) => void;
  selectedBrowserId?: string | null;
}) {
  const [filter, setFilter] = useState("");
  const [lifecycleFilter, setLifecycleFilter] = useState<BrowserLifecycleFilter>(initialBrowserLifecycleFilter);
  const [visibleColumns, setVisibleColumns] = useState<BrowserTableColumnKey[]>(initialBrowserTableColumns);
  const [columnWidths, setColumnWidths] = useState<Record<BrowserTableColumnId, number>>(initialBrowserTableColumnWidths);
  const [density, setDensity] = useState<BrowserTableDensity>(initialBrowserTableDensity);
  const [sortKey, setSortKey] = useState<BrowserSortKey>("health");
  const [sortDirection, setSortDirection] = useState<SortDirection>("asc");
  const resizeStateRef = useRef<{ column: BrowserTableColumnId; startX: number; startWidth: number } | null>(null);
  const visibleColumnSet = useMemo(() => new Set(visibleColumns), [visibleColumns]);
  const liveCount = useMemo(() => browsers.filter(isLiveBrowserRecord).length, [browsers]);
  const inertCount = useMemo(() => browsers.filter(isInertRetainedBrowserRecord).length, [browsers]);
  const activeTableColumns = useMemo(
    () => (["health", "id", "profile", "host", "sessions", "streams", "lastError", "actions"] as BrowserTableColumnId[])
      .filter((column) => column === "id" || column === "actions" || visibleColumnSet.has(column as BrowserTableColumnKey)),
    [visibleColumnSet],
  );
  const tableMinWidth = activeTableColumns.reduce((width, column) => width + columnWidths[column], 0);

  useEffect(() => {
    try {
      window.localStorage.setItem(BROWSER_TABLE_LIFECYCLE_FILTER_STORAGE_KEY, lifecycleFilter);
    } catch {
      // Restricted storage should not break the live service dashboard.
    }
  }, [lifecycleFilter]);

  useEffect(() => {
    try {
      window.localStorage.setItem(BROWSER_TABLE_VISIBLE_COLUMNS_STORAGE_KEY, JSON.stringify(visibleColumns));
    } catch {
      // Restricted storage should not break the live service dashboard.
    }
  }, [visibleColumns]);

  useEffect(() => {
    try {
      window.localStorage.setItem(BROWSER_TABLE_COLUMN_WIDTHS_STORAGE_KEY, JSON.stringify(columnWidths));
    } catch {
      // Restricted storage should not break the live service dashboard.
    }
  }, [columnWidths]);

  useEffect(() => {
    try {
      window.localStorage.setItem(BROWSER_TABLE_DENSITY_STORAGE_KEY, density);
    } catch {
      // Restricted storage should not break the live service dashboard.
    }
  }, [density]);

  const filteredBrowsers = useMemo(() => {
    const query = filter.trim().toLowerCase();
    const rows = browsers.filter((browser) => {
      if (!browserMatchesLifecycleFilter(browser, lifecycleFilter)) return false;
      return query ? browserSearchText(browser).includes(query) : true;
    });
    rows.sort((left, right) => {
      const defaultOrder = browserDefaultRank(left) - browserDefaultRank(right);
      if (defaultOrder !== 0) return defaultOrder;
      const order = compareBrowserValues(
        browserSortValue(left, sortKey),
        browserSortValue(right, sortKey),
      );
      return sortDirection === "asc" ? order : -order;
    });
    return rows;
  }, [browsers, filter, lifecycleFilter, sortDirection, sortKey]);

  const toggleSort = (nextSortKey: BrowserSortKey) => {
    if (nextSortKey === sortKey) {
      setSortDirection((current) => current === "asc" ? "desc" : "asc");
      return;
    }
    setSortKey(nextSortKey);
    setSortDirection("asc");
  };

  const toggleColumn = (column: BrowserTableColumnKey, nextVisible: boolean) => {
    setVisibleColumns((current) => {
      if (nextVisible) return current.includes(column) ? current : [...current, column];
      return current.filter((item) => item !== column);
    });
  };

  const resetColumns = () => setVisibleColumns(DEFAULT_BROWSER_TABLE_COLUMNS);
  const resetColumnWidths = () => setColumnWidths(DEFAULT_BROWSER_TABLE_COLUMN_WIDTHS);
  const resetTableView = () => {
    setLifecycleFilter("actionable");
    setVisibleColumns(DEFAULT_BROWSER_TABLE_COLUMNS);
    setColumnWidths(DEFAULT_BROWSER_TABLE_COLUMN_WIDTHS);
    setDensity("standard");
    try {
      BROWSER_TABLE_VIEW_STORAGE_KEYS.forEach((key) => window.localStorage.removeItem(key));
    } catch {
      // Restricted storage should not block the in-memory reset.
    }
  };
  const resetColumnWidth = (column: BrowserTableColumnId) => {
    setColumnWidths((current) => ({
      ...current,
      [column]: DEFAULT_BROWSER_TABLE_COLUMN_WIDTHS[column],
    }));
  };
  const startColumnResize = (column: BrowserTableColumnId, event: ReactMouseEvent<HTMLButtonElement>) => {
    event.preventDefault();
    resizeStateRef.current = { column, startX: event.clientX, startWidth: columnWidths[column] };

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const resizeState = resizeStateRef.current;
      if (!resizeState) return;
      const width = clampBrowserTableColumnWidth(resizeState.startWidth + moveEvent.clientX - resizeState.startX);
      if (!width) return;
      setColumnWidths((current) => ({ ...current, [resizeState.column]: width }));
    };

    const handleMouseUp = () => {
      resizeStateRef.current = null;
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  };
  const tableColumnSpan = visibleColumns.length + 2;

  return (
    <div className="service-browser-table-shell">
      <div className="service-browser-table-toolbar">
        <label className="service-browser-filter">
          <Filter className="size-3.5" />
          <span className="sr-only">Filter managed browsers</span>
          <input
            value={filter}
            onChange={(event) => setFilter(event.target.value)}
            placeholder="Filter browsers, profiles, hosts, sessions"
          />
        </label>
        <span className="service-browser-table-count">
          {filteredBrowsers.length} of {browsers.length} shown; {liveCount} live, {inertCount} inert retained
        </span>
        <div className="service-browser-table-controls" aria-label="Browser table controls">
          <div className="service-browser-table-control-group" aria-label="Browser record lifecycle filters">
            <span>Records</span>
            {[
              { value: "actionable", label: "Actionable" },
              { value: "live", label: "Live" },
              { value: "retained", label: "Retained" },
              { value: "all", label: "All records" },
            ].map((option) => (
              <button
                key={option.value}
                type="button"
                className={cn("service-filter-chip", lifecycleFilter === option.value && "service-filter-chip-active")}
                onClick={() => setLifecycleFilter(option.value as BrowserLifecycleFilter)}
              >
                {option.label}
              </button>
            ))}
          </div>
          <div className="service-browser-table-density" aria-label="Browser table density">
            <span>Density</span>
            {[
              { value: "compact", label: "Compact" },
              { value: "standard", label: "Standard" },
              { value: "expanded", label: "Expanded" },
            ].map((option) => (
              <button
                key={option.value}
                type="button"
                className={cn("service-filter-chip", density === option.value && "service-filter-chip-active")}
                onClick={() => setDensity(option.value as BrowserTableDensity)}
              >
                {option.label}
              </button>
            ))}
          </div>
          <div className="service-browser-table-control-group service-browser-table-column-menu">
            <span>Layout</span>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button size="sm" variant="outline" className="h-7 gap-1.5 rounded-full px-2 text-[11px]">
                  <MoreHorizontal className="size-3" />
                  Columns
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-44">
                <DropdownMenuLabel>Visible columns</DropdownMenuLabel>
                {BROWSER_TABLE_COLUMNS.map((column) => (
                  <DropdownMenuCheckboxItem
                    key={column.key}
                    checked={visibleColumnSet.has(column.key)}
                    onCheckedChange={(checked) => toggleColumn(column.key, checked === true)}
                  >
                    {column.label}
                  </DropdownMenuCheckboxItem>
                ))}
                <DropdownMenuSeparator />
                <DropdownMenuItem onClick={resetColumns}>Reset columns</DropdownMenuItem>
                <DropdownMenuItem onClick={resetColumnWidths}>Reset widths</DropdownMenuItem>
                <DropdownMenuItem onClick={resetTableView}>Reset table view</DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
      </div>
      <div className="service-browser-table-scroll">
        <table className={cn("service-browser-table", `service-browser-table-density-${density}`)} style={{ minWidth: tableMinWidth }}>
          <colgroup>
            {activeTableColumns.map((column) => (
              <col key={column} style={{ width: columnWidths[column] }} />
            ))}
          </colgroup>
          <thead>
            <tr>
              {visibleColumnSet.has("health") && (
                <BrowserTableHeaderCell column="health" width={columnWidths.health} label="Health" onResizeStart={startColumnResize} onResetWidth={resetColumnWidth}>
                  <BrowserSortButton sortKey="health" activeSortKey={sortKey} direction={sortDirection} onSort={toggleSort} />
                </BrowserTableHeaderCell>
              )}
              <BrowserTableHeaderCell column="id" width={columnWidths.id} label="Browser" onResizeStart={startColumnResize} onResetWidth={resetColumnWidth}>
                <BrowserSortButton sortKey="id" activeSortKey={sortKey} direction={sortDirection} onSort={toggleSort} />
              </BrowserTableHeaderCell>
              {visibleColumnSet.has("profile") && (
                <BrowserTableHeaderCell column="profile" width={columnWidths.profile} label="Profile" onResizeStart={startColumnResize} onResetWidth={resetColumnWidth}>
                  <BrowserSortButton sortKey="profile" activeSortKey={sortKey} direction={sortDirection} onSort={toggleSort} />
                </BrowserTableHeaderCell>
              )}
              {visibleColumnSet.has("host") && (
                <BrowserTableHeaderCell column="host" width={columnWidths.host} label="Host" onResizeStart={startColumnResize} onResetWidth={resetColumnWidth}>
                  <BrowserSortButton sortKey="host" activeSortKey={sortKey} direction={sortDirection} onSort={toggleSort} />
                </BrowserTableHeaderCell>
              )}
              {visibleColumnSet.has("sessions") && (
                <BrowserTableHeaderCell column="sessions" width={columnWidths.sessions} label="Sessions" onResizeStart={startColumnResize} onResetWidth={resetColumnWidth}>
                  <BrowserSortButton sortKey="sessions" activeSortKey={sortKey} direction={sortDirection} onSort={toggleSort} />
                </BrowserTableHeaderCell>
              )}
              {visibleColumnSet.has("streams") && (
                <BrowserTableHeaderCell column="streams" width={columnWidths.streams} label="Streams" onResizeStart={startColumnResize} onResetWidth={resetColumnWidth}>
                  <BrowserSortButton sortKey="streams" activeSortKey={sortKey} direction={sortDirection} onSort={toggleSort} />
                </BrowserTableHeaderCell>
              )}
              {visibleColumnSet.has("lastError") && (
                <BrowserTableHeaderCell column="lastError" width={columnWidths.lastError} label="Last error" onResizeStart={startColumnResize} onResetWidth={resetColumnWidth}>
                  Last error
                </BrowserTableHeaderCell>
              )}
              <BrowserTableHeaderCell column="actions" width={columnWidths.actions} label="Actions" onResizeStart={startColumnResize} onResetWidth={resetColumnWidth} />
            </tr>
          </thead>
          <tbody>
            {filteredBrowsers.length === 0 ? (
              <tr>
                <td colSpan={tableColumnSpan} className="service-browser-table-empty">
                  No browser records match the current filter.
                </td>
              </tr>
            ) : (
              filteredBrowsers.map((browser, index) => (
                <BrowserTableRow
                  key={browser.id || browser.cdpEndpoint || `browser-${index}`}
                  browser={browser}
                  selected={Boolean(browser.id && browser.id === selectedBrowserId)}
                  visibleColumns={visibleColumnSet}
                  onSelect={onSelect}
                  density={density}
                />
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function BrowserTableRow({
  browser,
  selected,
  visibleColumns,
  onSelect,
  density,
}: {
  browser: ServiceBrowser;
  selected: boolean;
  visibleColumns: Set<BrowserTableColumnKey>;
  onSelect: (browser: ServiceBrowser) => void;
  density: BrowserTableDensity;
}) {
  const tone = healthTone(browser.health);
  const sessionCount = browser.activeSessionIds?.length ?? 0;
  const viewStreamCount = browser.viewStreams?.length ?? 0;
  const processLabel = browser.pid ? `pid ${browser.pid}` : "retained";
  return (
    <tr className={cn("service-browser-table-row", selected && "service-browser-table-row-selected")} aria-selected={selected}>
      {visibleColumns.has("health") && (
        <td>
          <div className="service-browser-table-health">
            <span className={cn("service-browser-health-dot", `service-browser-health-${tone}`)} />
            <Badge variant="outline" className={cn("max-w-28 truncate px-1.5 text-[9px]", density === "compact" ? "h-3.5" : "h-4")}>
              {browser.health ?? "unknown"}
            </Badge>
          </div>
        </td>
      )}
      <td>
        <button
          type="button"
          className={cn("service-browser-table-id", selected && "service-browser-table-id-selected")}
          onClick={() => onSelect(browser)}
          aria-label={`Inspect browser ${browser.id}`}
          aria-current={selected ? "true" : undefined}
        >
          {browser.id || "unnamed browser"}
        </button>
      </td>
      {visibleColumns.has("profile") && (
        <td className="service-browser-table-cell-muted">
          {browser.profileId || "unassigned"}
        </td>
      )}
      {visibleColumns.has("host") && (
        <td>
          <div className="service-browser-table-host">
            <span>{browser.host ?? "unknown host"}</span>
            <span>{processLabel}</span>
          </div>
        </td>
      )}
      {visibleColumns.has("sessions") && <td className="service-browser-table-number">{sessionCount}</td>}
      {visibleColumns.has("streams") && <td className="service-browser-table-number">{viewStreamCount}</td>}
      {visibleColumns.has("lastError") && (
        <td className={cn("service-browser-table-error", !browser.lastError && "service-browser-table-cell-muted")}>
          {browser.lastError || "none"}
        </td>
      )}
      <td>
        <Button
          type="button"
          size="sm"
          variant="outline"
          className={cn("px-2 text-[10px]", density === "compact" ? "h-6" : "h-7")}
          onClick={() => onSelect(browser)}
        >
          Inspect
        </Button>
      </td>
    </tr>
  );
}

function ViewStreamInspectDialog({
  selection,
  fullscreen,
  onFullscreenChange,
  onOpenChange,
}: {
  selection: SelectedViewStream | null;
  fullscreen: boolean;
  onFullscreenChange: (fullscreen: boolean) => void;
  onOpenChange: (open: boolean) => void;
}) {
  const stream = selection?.stream;
  const embeddable = stream ? canEmbedViewStream(stream) : false;

  return (
    <Dialog open={!!selection} onOpenChange={onOpenChange}>
      <DialogContent className={cn("service-view-stream-dialog", fullscreen && "service-view-stream-dialog-fullscreen")}>
        {selection && stream && (
          <>
            <DialogHeader>
              <div className="flex items-start gap-3 pr-8">
                <div className="min-w-0 flex-1">
                  <DialogTitle className="truncate text-xl font-black tracking-[-0.04em]">
                    {selection.tab?.title || selection.browser.id || "Remote browser view"}
                  </DialogTitle>
                  <DialogDescription>
                    {viewStreamLabel(stream)} / {selection.browser.id}
                    {selection.tab?.id ? ` / ${selection.tab.id}` : ""}
                  </DialogDescription>
                </div>
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  className="h-8 gap-1.5 px-2 text-[10px]"
                  onClick={() => onFullscreenChange(!fullscreen)}
                >
                  {fullscreen ? <Minimize2 className="size-3" /> : <Maximize2 className="size-3" />}
                  {fullscreen ? "Window" : "Fullscreen"}
                </Button>
              </div>
            </DialogHeader>
            <div className="service-event-dialog-body">
              {selection.focusMessage && (
                <p className="service-browser-error">
                  <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
                  <span>{selection.focusMessage}</span>
                </p>
              )}
              {embeddable ? (
                <iframe
                  title={`${viewStreamLabel(stream)} ${stream.id ?? ""}`.trim()}
                  src={stream.url ?? undefined}
                  className="service-view-stream-frame"
                  sandbox="allow-same-origin allow-scripts allow-forms allow-pointer-lock allow-popups"
                />
              ) : (
                <pre className="service-event-details-json">{formatDetails(stream)}</pre>
              )}
              {stream.url && (
                <Button size="sm" variant="outline" className="w-fit gap-1.5" asChild>
                  <a href={stream.url} target="_blank" rel="noreferrer">
                    <ExternalLink className="size-3" />
                    Open stream directly
                  </a>
                </Button>
              )}
            </div>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}

function BrowserDetailDialog({
  browser,
  onInspectViewStream,
  onOpenChange,
}: {
  browser: ServiceBrowser | null;
  onInspectViewStream?: (stream: ServiceViewStream, browser: ServiceBrowser) => void;
  onOpenChange: (open: boolean) => void;
}) {
  return (
    <Dialog open={!!browser} onOpenChange={onOpenChange}>
      <DialogContent className="service-event-dialog">
        {browser && (
          <>
            <DialogHeader>
              <DialogTitle className="pr-8 text-xl font-black tracking-[-0.04em]">
                {browser.id || "Browser process"}
              </DialogTitle>
              <DialogDescription>
                {browser.host ?? "unknown host"} / {browser.health ?? "unknown health"}
              </DialogDescription>
            </DialogHeader>
            <BrowserDetailContent browser={browser} onInspectViewStream={onInspectViewStream} />
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}

function BrowserDetailContent({
  browser,
  onInspectViewStream,
}: {
  browser: ServiceBrowser;
  onInspectViewStream?: (stream: ServiceViewStream, browser: ServiceBrowser) => void;
}) {
  const viewStreamCount = browser.viewStreams?.length ?? 0;
  return (
    <div className="service-event-dialog-body">
      {browser.lastError && (
        <div className="service-browser-error">
          <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
          <span>{browser.lastError}</span>
        </div>
      )}
      <div className="service-event-detail-grid">
        <EventDetailItem label="Browser ID" value={browser.id} />
        <EventDetailItem label="Profile" value={browser.profileId} />
        <EventDetailItem label="Host" value={browser.host} />
        <EventDetailItem label="Health" value={browser.health} />
        <EventDetailItem label="PID" value={browser.pid ? String(browser.pid) : null} />
        <EventDetailItem label="CDP endpoint" value={browser.cdpEndpoint} />
        <EventDetailItem label="Active sessions" value={String(browser.activeSessionIds?.length ?? 0)} />
        <EventDetailItem label="View streams" value={String(viewStreamCount)} />
      </div>
      {!!browser.activeSessionIds?.length && (
        <div>
          <p className="mb-2 text-[10px] font-black uppercase tracking-[0.18em] text-muted-foreground">
            Attached sessions
          </p>
          <div className="service-token-list">
            {browser.activeSessionIds.map((sessionId) => (
              <Badge key={sessionId} variant="outline" className="max-w-full truncate">
                {sessionId}
              </Badge>
            ))}
          </div>
        </div>
      )}
      {!!browser.viewStreams?.length && (
        <div>
          <p className="mb-2 text-[10px] font-black uppercase tracking-[0.18em] text-muted-foreground">
            View streams
          </p>
          <div className="grid gap-3">
            {browser.viewStreams.map((stream, index) => (
              <ViewStreamCard
                key={`${stream.id ?? stream.provider ?? "stream"}-${index}`}
                stream={stream}
                onInspect={onInspectViewStream ? (selectedStream) => onInspectViewStream(selectedStream, browser) : undefined}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

export function ServiceDetailInspector({
  selection,
  actions = {},
}: {
  selection: ServiceInspectorSelection | null;
  actions?: ServiceInspectorActions;
}) {
  if (!selection) {
    return (
      <div className="service-inspector-empty">
        <div className="service-inspector-empty-card">
          <RadioTower className="size-6 text-muted-foreground" />
          <div>
            <p>Select a service record</p>
            <span>Choose a browser, profile, incident, session, tab, or job row to inspect operational details here.</span>
          </div>
        </div>
      </div>
    );
  }
  const header = serviceInspectorHeader(selection);

  return (
    <ScrollArea className="h-full">
      <div className="service-inspector">
        <div className="service-inspector-header">
          <div className="min-w-0">
            <p className="service-inspector-kicker">{header.kicker}</p>
            <h2>{header.title}</h2>
            <span>{header.description}</span>
          </div>
          <span className={cn("service-browser-health-dot", `service-browser-health-${header.tone}`)} />
        </div>
        {selection.kind === "browser" && <BrowserDetailContent browser={selection.browser} />}
        {selection.kind === "profile" && <ProfileAllocationDetailContent allocation={selection.allocation} />}
        {selection.kind === "incident" && (
          <IncidentDetailContent
            incident={selection.incident}
            acting={actions.actingIncidentId === selection.incident.id}
            onAcknowledge={actions.onAcknowledgeIncident}
            onResolve={actions.onResolveIncident}
          />
        )}
        {selection.kind === "session" && <SessionDetailContent session={selection.session} />}
        {selection.kind === "tab" && (
          <TabDetailContent tab={selection.tab} viewStreamAvailable={selection.viewStreamAvailable} />
        )}
        {selection.kind === "job" && <JobDetailContent job={selection.job} onCancel={actions.onCancelJob} />}
      </div>
    </ScrollArea>
  );
}

function serviceInspectorHeader(selection: ServiceInspectorSelection): {
  kicker: string;
  title: string;
  description: string;
  tone: ReturnType<typeof healthTone>;
} {
  if (selection.kind === "browser") {
    return {
      kicker: "Browser inspector",
      title: selection.browser.id || "Browser process",
      description: `${selection.browser.host ?? "unknown host"} / ${selection.browser.health ?? "unknown health"}`,
      tone: healthTone(selection.browser.health),
    };
  }
  if (selection.kind === "session") {
    return {
      kicker: "Session inspector",
      title: selection.session.id || "Service session",
      description: `${selection.session.lease ?? "shared"} / ${formatActor(selection.session.owner)}`,
      tone: "good",
    };
  }
  if (selection.kind === "profile") {
    return {
      kicker: "Profile inspector",
      title: selection.allocation.profileName || selection.allocation.profileId || "Profile allocation",
      description: `${selection.allocation.leaseState ?? "unknown"} / ${selection.allocation.recommendedAction ?? "inspect"}`,
      tone: profileAllocationTone(selection.allocation.leaseState),
    };
  }
  if (selection.kind === "incident") {
    const priority = incidentPriorityView(selection.incident);
    return {
      kicker: "Incident inspector",
      title: selection.incident.label,
      description: `${priority.severityLabel} / ${priority.escalationLabel} / ${incidentHandlingLabel(selection.incident)}`,
      tone: healthTone(selection.incident.currentHealth ?? undefined),
    };
  }
  if (selection.kind === "tab") {
    return {
      kicker: "Tab inspector",
      title: selection.tab.title || selection.tab.id || "Browser tab",
      description: `${selection.tab.lifecycle ?? "unknown"} / ${selection.tab.browserId ?? "unknown browser"}`,
      tone: selection.tab.lifecycle === "crashed" ? "bad" : selection.tab.lifecycle === "ready" ? "good" : "neutral",
    };
  }
  return {
    kicker: "Job inspector",
    title: selection.job.action ?? "Service job",
    description: `${selection.job.state ?? "unknown"} / ${formatRelativeTime(selection.job.submittedAt)}`,
    tone: selection.job.state === "succeeded"
      ? "good"
      : selection.job.state === "failed" || selection.job.state === "timed_out" || selection.job.state === "cancelled"
        ? "bad"
        : "neutral",
  };
}

function JobDetailDialog({
  job,
  onOpenChange,
  onCancel,
}: {
  job: ServiceJob | null;
  onOpenChange: (open: boolean) => void;
  onCancel: (job: ServiceJob) => void;
}) {
  return (
    <Dialog open={!!job} onOpenChange={onOpenChange}>
      <DialogContent className="service-event-dialog">
        {job && (
          <>
            <DialogHeader>
              <DialogTitle className="pr-8 text-xl font-black tracking-[-0.04em]">
                {job.action ?? "Service job"}
              </DialogTitle>
              <DialogDescription>
                {job.state ?? "unknown"} / {formatRelativeTime(job.submittedAt)}
              </DialogDescription>
            </DialogHeader>
            <JobDetailContent job={job} onCancel={onCancel} />
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}

function JobDetailContent({
  job,
  onCancel,
}: {
  job: ServiceJob;
  onCancel?: (job: ServiceJob) => void;
}) {
  const request = formatDetails(job.request);
  const response = formatDetails(job.response ?? job.result);
  const target = formatDetails(job.target);
  const canCancel = job.state === "queued" || job.state === "running";
  const namingWarning = serviceJobNamingWarningLabel(job.namingWarnings);
  return (
    <div className="service-event-dialog-body">
      {job.error && (
        <div className="service-browser-error">
          <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
          <span>{job.error}</span>
        </div>
      )}
      {namingWarning && (
        <div className="service-browser-error">
          <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
          <span>{namingWarning} name metadata. Add serviceName, agentName, and taskName for traceable jobs.</span>
        </div>
      )}
      <div className="service-event-detail-grid">
        <EventDetailItem label="Job ID" value={job.id} />
        <EventDetailItem label="Action" value={job.action} />
        <EventDetailItem label="State" value={job.state} />
        <EventDetailItem label="Priority" value={job.priority} />
        <EventDetailItem label="Owner" value={job.owner ? formatActor(job.owner) : null} />
        <EventDetailItem label="Timeout" value={job.timeoutMs ? `${job.timeoutMs} ms` : null} />
        <EventDetailItem label="Submitted" value={job.submittedAt ? formatAbsoluteTime(job.submittedAt) : null} />
        <EventDetailItem label="Started" value={job.startedAt ? formatAbsoluteTime(job.startedAt) : null} />
        <EventDetailItem label="Completed" value={job.completedAt ? formatAbsoluteTime(job.completedAt) : null} />
      </div>
      {target && (
        <div>
          <p className="mb-2 text-[10px] font-black uppercase tracking-[0.18em] text-muted-foreground">
            Target
          </p>
          <pre className="service-event-details-json">{target}</pre>
        </div>
      )}
      {request && (
        <div>
          <p className="mb-2 text-[10px] font-black uppercase tracking-[0.18em] text-muted-foreground">
            Request
          </p>
          <pre className="service-event-details-json">{request}</pre>
        </div>
      )}
      {response && (
        <div>
          <p className="mb-2 text-[10px] font-black uppercase tracking-[0.18em] text-muted-foreground">
            Response
          </p>
          <pre className="service-event-details-json">{response}</pre>
        </div>
      )}
      {canCancel && onCancel && (
        <Button
          type="button"
          variant="destructive"
          className="rounded-full"
          onClick={() => onCancel(job)}
        >
          {job.state === "running" ? "Cancel running job" : "Cancel queued job"}
        </Button>
      )}
    </div>
  );
}

function ProfileAllocationRow({
  allocation,
  onSelect,
}: {
  allocation: ServiceProfileAllocation;
  onSelect: (allocation: ServiceProfileAllocation) => void;
}) {
  const tone = profileAllocationTone(allocation.leaseState);
  const holderCount = allocation.holderCount ?? allocation.holderSessionIds?.length ?? 0;
  const waitingCount = allocation.waitingJobCount ?? allocation.waitingJobIds?.length ?? 0;
  return (
    <button
      type="button"
      className="service-browser-row"
      onClick={() => onSelect(allocation)}
      aria-label={`Inspect profile allocation ${allocation.profileId}`}
    >
      <span className={cn("service-browser-health-dot", `service-browser-health-${tone}`)} />
      <div className="min-w-0 flex-1">
        <div className="flex min-w-0 items-center gap-2">
          <span className="truncate text-xs font-bold text-foreground">
            {allocation.profileName || allocation.profileId || "unnamed profile"}
          </span>
          <Badge variant="outline" className="h-4 max-w-28 truncate px-1.5 text-[9px]">
            {allocation.leaseState ?? "unknown"}
          </Badge>
          <Badge variant="outline" className="h-4 max-w-32 truncate px-1.5 text-[9px]">
            {allocation.recommendedAction ?? "inspect"}
          </Badge>
        </div>
        <p className="mt-1 truncate text-xs text-muted-foreground">
          {holderCount} holders / {waitingCount} waiting / {allocation.browserIds?.length ?? 0} browsers / {allocation.tabIds?.length ?? 0} tabs
        </p>
        <div className="mt-2 grid gap-1 text-[10px] text-muted-foreground sm:grid-cols-2">
          <span className="truncate">service: {formatStringList(allocation.serviceNames)}</span>
          <span className="truncate">task: {formatStringList(allocation.taskNames)}</span>
          <span className="truncate">holders: {formatStringList(allocation.holderSessionIds)}</span>
          <span className="truncate">conflicts: {formatStringList(allocation.conflictSessionIds)}</span>
        </div>
      </div>
    </button>
  );
}

function ProfileAllocationDetailDialog({
  allocation,
  loading,
  error,
  onOpenChange,
}: {
  allocation: ServiceProfileAllocation | null;
  loading: boolean;
  error: string;
  onOpenChange: (open: boolean) => void;
}) {
  return (
    <Dialog open={!!allocation} onOpenChange={onOpenChange}>
      <DialogContent className="service-event-dialog">
        {allocation && (
          <>
            <DialogHeader>
              <DialogTitle className="pr-8 text-xl font-black tracking-[-0.04em]">
                {allocation.profileName || allocation.profileId || "Profile allocation"}
              </DialogTitle>
              <DialogDescription>
                {loading
                  ? "Refreshing allocation from service API"
                  : `${allocation.leaseState ?? "unknown"} / ${allocation.recommendedAction ?? "inspect"}`}
              </DialogDescription>
            </DialogHeader>
            <ProfileAllocationDetailContent allocation={allocation} loading={loading} error={error} />
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}

function ProfileAllocationDetailContent({
  allocation,
  loading = false,
  error = "",
}: {
  allocation: ServiceProfileAllocation;
  loading?: boolean;
  error?: string;
}) {
  const raw = formatDetails(allocation);
  return (
    <div className="service-event-dialog-body">
      {loading && (
        <div className="flex items-center gap-2 rounded-2xl border border-border/70 bg-foreground/[0.03] px-3 py-2 text-xs text-muted-foreground">
          <Loader2 className="size-3.5 animate-spin" />
          Loading current allocation row
        </div>
      )}
      {error && (
        <div className="flex items-start gap-2 rounded-2xl border border-destructive/25 bg-destructive/10 px-3 py-2 text-xs text-destructive">
          <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
          <span>{error}</span>
        </div>
      )}
      <div className="service-event-detail-grid">
        <EventDetailItem label="Profile ID" value={allocation.profileId} />
        <EventDetailItem label="Profile name" value={allocation.profileName} />
        <EventDetailItem label="Lease state" value={allocation.leaseState} />
        <EventDetailItem label="Recommended action" value={allocation.recommendedAction} />
        <EventDetailItem label="Allocation" value={allocation.allocation} />
        <EventDetailItem label="Keyring" value={allocation.keyring} />
        <EventDetailItem label="Holder count" value={String(allocation.holderCount ?? allocation.holderSessionIds?.length ?? 0)} />
        <EventDetailItem label="Waiting job count" value={String(allocation.waitingJobCount ?? allocation.waitingJobIds?.length ?? 0)} />
        <EventDetailItem label="Browser count" value={String(allocation.browserIds?.length ?? 0)} />
        <EventDetailItem label="Tab count" value={String(allocation.tabIds?.length ?? 0)} />
      </div>
      <ProfileAllocationTokenSection title="Holder sessions" values={allocation.holderSessionIds} />
      <ProfileAllocationTokenSection title="Exclusive holders" values={allocation.exclusiveHolderSessionIds} />
      <ProfileAllocationTokenSection title="Waiting jobs" values={allocation.waitingJobIds} />
      <ProfileAllocationTokenSection title="Conflicts" values={allocation.conflictSessionIds} />
      <ProfileAllocationTokenSection title="Services" values={allocation.serviceNames} />
      <ProfileAllocationTokenSection title="Agents" values={allocation.agentNames} />
      <ProfileAllocationTokenSection title="Tasks" values={allocation.taskNames} />
      <ProfileAllocationTokenSection title="Target services" values={allocation.targetServiceIds} />
      <ProfileAllocationTokenSection title="Authenticated services" values={allocation.authenticatedServiceIds} />
      <ProfileReadinessSection rows={allocation.targetReadiness} />
      <ProfileAllocationTokenSection title="Shared services" values={allocation.sharedServiceIds} />
      <ProfileAllocationTokenSection title="Browsers" values={allocation.browserIds} />
      <ProfileAllocationTokenSection title="Tabs" values={allocation.tabIds} />
      {raw && (
        <div>
          <p className="mb-2 text-[10px] font-black uppercase tracking-[0.18em] text-muted-foreground">
            Raw allocation
          </p>
          <pre className="service-event-details-json">{raw}</pre>
        </div>
      )}
    </div>
  );
}

function ProfileReadinessSection({ rows }: { rows?: ServiceProfileTargetReadiness[] }) {
  const items = rows?.filter((row) => row.targetServiceId?.trim()) ?? [];
  if (items.length === 0) return null;
  return (
    <div>
      <p className="mb-2 text-[10px] font-black uppercase tracking-[0.18em] text-muted-foreground">
        Target readiness
      </p>
      <div className="flex flex-col gap-2">
        {items.map((row, index) => (
          <div
            key={`${row.targetServiceId}-${row.loginId ?? "default"}-${index}`}
            className="rounded-2xl border border-border/70 bg-foreground/[0.03] px-3 py-2 text-xs"
          >
            <div className="font-black text-foreground">
              {row.targetServiceId}
              {row.loginId ? ` / ${row.loginId}` : ""}
            </div>
            <div className="mt-1 text-muted-foreground">
              {row.state ?? "unknown"} / {row.recommendedAction ?? "inspect"}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function ProfileAllocationTokenSection({ title, values }: { title: string; values?: string[] }) {
  const items = values?.filter((value) => value.trim().length > 0) ?? [];
  if (items.length === 0) return null;
  return (
    <div>
      <p className="mb-2 text-[10px] font-black uppercase tracking-[0.18em] text-muted-foreground">
        {title}
      </p>
      <div className="service-token-list">
        {items.map((value) => (
          <Badge key={value} variant="outline" className="max-w-full truncate">
            {value}
          </Badge>
        ))}
      </div>
    </div>
  );
}

function ServiceSessionRow({
  session,
  onSelect,
}: {
  session: ServiceSession;
  onSelect: (session: ServiceSession) => void;
}) {
  return (
    <button
      type="button"
      className="service-browser-row"
      onClick={() => onSelect(session)}
      aria-label={`Inspect session ${session.id}`}
    >
      <span className="service-browser-health-dot service-browser-health-good" />
      <div className="min-w-0 flex-1">
        <div className="flex min-w-0 items-center gap-2">
          <span className="truncate text-xs font-bold text-foreground">{session.id || "unnamed session"}</span>
          <Badge variant="outline" className="h-4 max-w-28 truncate px-1.5 text-[9px]">
            {session.lease ?? "shared"}
          </Badge>
        </div>
        <p className="mt-1 truncate text-xs text-muted-foreground">
          {formatActor(session.owner)} / {session.browserIds?.length ?? 0} browsers / {session.tabIds?.length ?? 0} tabs
        </p>
      </div>
    </button>
  );
}

function ServiceTabRow({
  tab,
  viewStreamAvailable,
  onInspect,
  onSelect,
}: {
  tab: ServiceTab;
  viewStreamAvailable?: boolean;
  onInspect?: (tab: ServiceTab) => void;
  onSelect: (tab: ServiceTab) => void;
}) {
  const tone = tab.lifecycle === "crashed" ? "bad" : tab.lifecycle === "ready" ? "good" : "neutral";
  return (
    <div className="service-browser-row service-browser-row-composite">
      <button
        type="button"
        className="flex min-w-0 flex-1 items-center gap-3 text-left"
        onClick={() => onSelect(tab)}
        aria-label={`Inspect tab ${tab.id}`}
      >
        <span className={cn("service-browser-health-dot", `service-browser-health-${tone}`)} />
        <div className="min-w-0 flex-1">
          <div className="flex min-w-0 items-center gap-2">
            <span className="truncate text-xs font-bold text-foreground">{tab.title || tab.id || "untitled tab"}</span>
            <Badge variant="outline" className="h-4 max-w-28 truncate px-1.5 text-[9px]">
              {tab.lifecycle ?? "unknown"}
            </Badge>
          </div>
          <p className="mt-1 truncate text-xs text-muted-foreground">
            {tab.url || tab.targetId || tab.browserId || "no target"}
          </p>
        </div>
      </button>
      {viewStreamAvailable && onInspect && (
        <Button
          type="button"
          size="sm"
          variant="outline"
          className="h-7 shrink-0 gap-1.5 px-2 text-[10px]"
          onClick={() => onInspect(tab)}
        >
          <Eye className="size-3" />
          View
        </Button>
      )}
    </div>
  );
}

function SessionDetailDialog({
  session,
  onOpenChange,
}: {
  session: ServiceSession | null;
  onOpenChange: (open: boolean) => void;
}) {
  return (
    <Dialog open={!!session} onOpenChange={onOpenChange}>
      <DialogContent className="service-event-dialog">
        {session && (
          <>
            <DialogHeader>
              <DialogTitle className="pr-8 text-xl font-black tracking-[-0.04em]">
                {session.id || "Service session"}
              </DialogTitle>
              <DialogDescription>
                {session.lease ?? "shared"} / {formatActor(session.owner)}
              </DialogDescription>
            </DialogHeader>
            <SessionDetailContent session={session} />
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}

function SessionDetailContent({ session }: { session: ServiceSession }) {
  return (
    <div className="service-event-dialog-body">
      <div className="service-event-detail-grid">
        <EventDetailItem label="Session ID" value={session.id} />
        <EventDetailItem label="Owner" value={formatActor(session.owner)} />
        <EventDetailItem label="Lease" value={session.lease} />
        <EventDetailItem label="Created" value={session.createdAt ? formatAbsoluteTime(session.createdAt) : null} />
        <EventDetailItem label="Expires" value={session.expiresAt ? formatAbsoluteTime(session.expiresAt) : null} />
        <EventDetailItem label="Browsers" value={String(session.browserIds?.length ?? 0)} />
        <EventDetailItem label="Tabs" value={String(session.tabIds?.length ?? 0)} />
      </div>
      {!!session.browserIds?.length && (
        <div>
          <p className="mb-2 text-[10px] font-black uppercase tracking-[0.18em] text-muted-foreground">
            Browser IDs
          </p>
          <div className="service-token-list">
            {session.browserIds.map((browserId) => (
              <Badge key={browserId} variant="outline" className="max-w-full truncate">
                {browserId}
              </Badge>
            ))}
          </div>
        </div>
      )}
      {!!session.tabIds?.length && (
        <div>
          <p className="mb-2 text-[10px] font-black uppercase tracking-[0.18em] text-muted-foreground">
            Tab IDs
          </p>
          <div className="service-token-list">
            {session.tabIds.map((tabId) => (
              <Badge key={tabId} variant="outline" className="max-w-full truncate">
                {tabId}
              </Badge>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function TabDetailDialog({
  tab,
  viewStreamAvailable,
  onInspect,
  onOpenChange,
}: {
  tab: ServiceTab | null;
  viewStreamAvailable?: boolean;
  onInspect?: (tab: ServiceTab) => void;
  onOpenChange: (open: boolean) => void;
}) {
  return (
    <Dialog open={!!tab} onOpenChange={onOpenChange}>
      <DialogContent className="service-event-dialog">
        {tab && (
          <>
            <DialogHeader>
              <DialogTitle className="pr-8 text-xl font-black tracking-[-0.04em]">
                {tab.title || tab.id || "Browser tab"}
              </DialogTitle>
              <DialogDescription>
                {tab.lifecycle ?? "unknown"} / {tab.browserId ?? "unknown browser"}
              </DialogDescription>
            </DialogHeader>
            <TabDetailContent tab={tab} viewStreamAvailable={viewStreamAvailable} onInspect={onInspect} />
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}

function TabDetailContent({
  tab,
  viewStreamAvailable,
  onInspect,
}: {
  tab: ServiceTab;
  viewStreamAvailable?: boolean;
  onInspect?: (tab: ServiceTab) => void;
}) {
  return (
    <div className="service-event-dialog-body">
      {tab.url && <p className="service-event-dialog-message">{tab.url}</p>}
      <div className="service-event-detail-grid">
        <EventDetailItem label="Tab ID" value={tab.id} />
        <EventDetailItem label="Browser ID" value={tab.browserId} />
        <EventDetailItem label="Target ID" value={tab.targetId} />
        <EventDetailItem label="Session ID" value={tab.sessionId} />
        <EventDetailItem label="Owner session" value={tab.ownerSessionId} />
        <EventDetailItem label="Lifecycle" value={tab.lifecycle} />
        <EventDetailItem label="Snapshot" value={tab.latestSnapshotId} />
        <EventDetailItem label="Screenshot" value={tab.latestScreenshotId} />
        <EventDetailItem label="Challenge" value={tab.challengeId} />
      </div>
      {viewStreamAvailable && onInspect && (
        <Button className="w-fit gap-1.5" size="sm" onClick={() => onInspect(tab)}>
          <Eye className="size-3.5" />
          Inspect remote view
        </Button>
      )}
    </div>
  );
}

function EventDetailDialog({
  event,
  onOpenChange,
}: {
  event: ServiceEvent | null;
  onOpenChange: (open: boolean) => void;
}) {
  const details = formatDetails(event?.details);
  return (
    <Dialog open={!!event} onOpenChange={onOpenChange}>
      <DialogContent className="service-event-dialog">
        {event && (
          <>
            <DialogHeader>
              <DialogTitle className="pr-8 text-xl font-black tracking-[-0.04em]">
                {formatEventKind(event.kind)}
              </DialogTitle>
              <DialogDescription>
                {formatAbsoluteTime(event.timestamp)} / {formatRelativeTime(event.timestamp)}
              </DialogDescription>
            </DialogHeader>
            <div className="service-event-dialog-body">
              <p className="service-event-dialog-message">{event.message || "No message"}</p>
              <div className="service-event-detail-grid">
                <EventDetailItem label="Event ID" value={event.id} />
                <EventDetailItem label="Kind" value={event.kind} />
                <EventDetailItem label="Browser" value={event.browserId} />
                <EventDetailItem label="Previous health" value={event.previousHealth} />
                <EventDetailItem label="Current health" value={event.currentHealth} />
              </div>
              {details && (
                <div>
                  <p className="mb-2 text-[10px] font-black uppercase tracking-[0.18em] text-muted-foreground">
                    Raw details
                  </p>
                  <pre className="service-event-details-json">{details}</pre>
                </div>
              )}
            </div>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}

function IncidentDetailDialog({
  incident,
  activity,
  activityLoading,
  activityError,
  onOpenChange,
  onAcknowledge,
  onResolve,
  acting,
}: {
  incident: IncidentRecord | null;
  activity: ServiceTraceTimelineItem[] | null;
  activityLoading: boolean;
  activityError: string;
  onOpenChange: (open: boolean) => void;
  onAcknowledge: (incident: IncidentRecord, note: string) => void;
  onResolve: (incident: IncidentRecord, note: string) => void;
  acting: boolean;
}) {
  const [actionNote, setActionNote] = useState("");
  const fallbackTimeline = useMemo(
    () => (incident ? deriveIncidentTimeline(incident) : []),
    [incident],
  );
  const timeline = activity && activity.length > 0 ? activity : fallbackTimeline;
  const incidentCount = timeline.length;
  const serviceOnlyEvents = incident?.serviceEvents.filter((event) => event.kind !== "browser_health_changed") ?? [];
  const handlingState = incident ? incidentHandlingState(incident) : "unacknowledged";
  const priority = incident
    ? incidentPriorityView(incident)
    : {
        severityTone: "info" as const,
        severityLabel: "unknown",
        escalationLabel: "unknown",
        recommendedAction: null,
        ariaLabel: "",
      };

  useEffect(() => {
    setActionNote("");
  }, [incident?.id]);

  return (
    <Dialog open={!!incident} onOpenChange={onOpenChange}>
      <DialogContent className="service-event-dialog">
        {incident && (
          <>
            <DialogHeader>
              <DialogTitle className="pr-8 text-xl font-black tracking-[-0.04em]">
                {incident.label}
              </DialogTitle>
              <DialogDescription>
                {incidentCount} incident entries / {incidentHandlingLabel(incident)} / latest {formatRelativeTime(incident.latestTimestamp)}
              </DialogDescription>
            </DialogHeader>
            <div className="service-event-dialog-body">
              <p className="service-event-dialog-message">{incident.latestMessage}</p>
              <div className="service-incident-priority-row">
                <div className={cn("service-incident-priority-card", `service-incident-priority-${priority.severityTone}`)}>
                  <span>Severity</span>
                  <strong>{priority.severityLabel}</strong>
                </div>
                <div className="service-incident-priority-card service-incident-priority-escalation">
                  <span>Escalation</span>
                  <strong>{priority.escalationLabel}</strong>
                </div>
              </div>
              {priority.recommendedAction && (
                <div className={cn("service-incident-recommended-action", `service-incident-priority-${priority.severityTone}`)}>
                  <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
                  <div>
                    <span>Recommended action</span>
                    <p>{priority.recommendedAction}</p>
                  </div>
                </div>
              )}
              <div className="service-event-detail-grid">
                <EventDetailItem label="Browser" value={incident.browserId} />
                <EventDetailItem label="Severity" value={priority.severityLabel} />
                <EventDetailItem label="Escalation" value={priority.escalationLabel} />
                <EventDetailItem label="Latest kind" value={formatEventKind(incident.latestKind)} />
                <EventDetailItem label="Current health" value={incident.currentHealth} />
                <EventDetailItem label="Handling state" value={incidentHandlingLabel(incident)} />
                <EventDetailItem label="Incident count" value={String(incidentCount)} />
                <EventDetailItem label="Acknowledged by" value={incident.acknowledgedBy} />
                <EventDetailItem label="Acknowledged" value={incident.acknowledgedAt ? formatAbsoluteTime(incident.acknowledgedAt) : null} />
                <EventDetailItem label="Resolved by" value={incident.resolvedBy} />
                <EventDetailItem label="Resolved" value={incident.resolvedAt ? formatAbsoluteTime(incident.resolvedAt) : null} />
              </div>
              {(incident.acknowledgementNote || incident.resolutionNote) && (
                <div className="service-incident-notes">
                  {incident.acknowledgementNote && (
                    <p>
                      <span>Acknowledgement note</span>
                      {incident.acknowledgementNote}
                    </p>
                  )}
                  {incident.resolutionNote && (
                    <p>
                      <span>Resolution note</span>
                      {incident.resolutionNote}
                    </p>
                  )}
                </div>
              )}
              {handlingState !== "resolved" && (
                <div className="service-incident-action-note">
                  <label htmlFor="service-incident-action-note">
                    Operator note
                  </label>
                  <textarea
                    id="service-incident-action-note"
                    value={actionNote}
                    onChange={(event) => setActionNote(event.target.value)}
                    placeholder="Optional context for the acknowledgement or resolution"
                    rows={3}
                  />
                </div>
              )}
              <div className="service-incident-actions">
                {handlingState === "unacknowledged" && (
                  <Button
                    type="button"
                    variant="outline"
                    className="rounded-full"
                    disabled={acting}
                    onClick={() => onAcknowledge(incident, actionNote)}
                  >
                    {acting ? <Loader2 className="size-3.5 animate-spin" /> : <CheckCircle2 className="size-3.5" />}
                    Mark acknowledged
                  </Button>
                )}
                {handlingState !== "resolved" && (
                  <Button
                    type="button"
                    className="rounded-full"
                    disabled={acting}
                    onClick={() => onResolve(incident, actionNote)}
                  >
                    {acting ? <Loader2 className="size-3.5 animate-spin" /> : <ShieldCheck className="size-3.5" />}
                    Mark resolved
                  </Button>
                )}
              </div>
              {timeline.length > 0 && (
                <div>
                  <div className="mb-2 flex items-center justify-between gap-3">
                    <p className="text-[10px] font-black uppercase tracking-[0.18em] text-muted-foreground">
                      Incident history
                    </p>
                    {activityLoading && (
                      <span className="inline-flex items-center gap-1 text-[10px] font-bold text-muted-foreground">
                        <Loader2 className="size-3 animate-spin" />
                        Loading service timeline
                      </span>
                    )}
                  </div>
                  {activityError && (
                    <p className="mb-2 text-[10px] leading-4 text-muted-foreground">
                      Using local fallback timeline: {activityError}
                    </p>
                  )}
                  <div className="service-incident-history">
                    {timeline.map((item) => (
                      <div key={item.id} className="service-incident-history-item">
                        <EventDot kind={item.kind} />
                        <div className="min-w-0 flex-1">
                          <div className="flex min-w-0 items-center gap-2">
                            <span className="truncate text-xs font-bold text-foreground">
                              {item.title}
                            </span>
                            {item.source && (
                              <Badge variant="outline" className="rounded-full px-1.5 py-0 text-[9px] uppercase">
                                {item.source}
                              </Badge>
                            )}
                            <span className="ml-auto shrink-0 text-[10px] text-muted-foreground">
                              {formatAbsoluteTime(item.timestamp)}
                            </span>
                          </div>
                          {item.message && (
                            <p className="mt-1 text-xs leading-5 text-muted-foreground">
                              {item.message}
                            </p>
                          )}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}
              {incident.transitionEvents.length > 0 && (
                <div>
                  <p className="mb-2 text-[10px] font-black uppercase tracking-[0.18em] text-muted-foreground">
                    Health transitions
                  </p>
                  <div className="space-y-1">
                    {incident.transitionEvents.map((event) => (
                      <div key={event.id} className="service-incident-entry">
                        <div className="flex items-center gap-2">
                          <EventDot kind={event.kind} />
                          <span className="truncate text-xs font-bold text-foreground">
                            {formatHealthLabel(event.previousHealth)} to {formatHealthLabel(event.currentHealth)}
                          </span>
                          <span className="ml-auto shrink-0 text-[10px] text-muted-foreground">
                            {formatAbsoluteTime(event.timestamp)}
                          </span>
                        </div>
                        <p className="mt-1 text-xs leading-5 text-muted-foreground">{event.message}</p>
                      </div>
                    ))}
                  </div>
                </div>
              )}
              {serviceOnlyEvents.length > 0 && (
                <div>
                  <p className="mb-2 text-[10px] font-black uppercase tracking-[0.18em] text-muted-foreground">
                    Related service events
                  </p>
                  <div className="space-y-1">
                    {serviceOnlyEvents.map((event) => (
                      <div key={event.id} className="service-incident-entry">
                        <div className="flex items-center gap-2">
                          <EventDot kind={event.kind} />
                          <span className="truncate text-xs font-bold text-foreground">
                            {formatEventKind(event.kind)}
                          </span>
                          <span className="ml-auto shrink-0 text-[10px] text-muted-foreground">
                            {formatAbsoluteTime(event.timestamp)}
                          </span>
                        </div>
                        <p className="mt-1 text-xs leading-5 text-muted-foreground">{event.message}</p>
                      </div>
                    ))}
                  </div>
                </div>
              )}
              {incident.jobEvents.length > 0 && (
                <div>
                  <p className="mb-2 text-[10px] font-black uppercase tracking-[0.18em] text-muted-foreground">
                    Related jobs
                  </p>
                  <div className="space-y-1">
                    {incident.jobEvents.map((event) => (
                      <div key={event.id} className="service-incident-entry">
                        <div className="flex items-center gap-2">
                          <EventDot kind={event.kind} />
                          <span className="truncate text-xs font-bold text-foreground">
                            {formatEventKind(event.kind)}
                          </span>
                          <span className="ml-auto shrink-0 text-[10px] text-muted-foreground">
                            {formatAbsoluteTime(event.timestamp)}
                          </span>
                        </div>
                        <p className="mt-1 text-xs leading-5 text-muted-foreground">{event.message}</p>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}

function IncidentDetailContent({
  incident,
  acting = false,
  onAcknowledge,
  onResolve,
}: {
  incident: IncidentRecord;
  acting?: boolean;
  onAcknowledge?: (incident: IncidentRecord, note: string) => void;
  onResolve?: (incident: IncidentRecord, note: string) => void;
}) {
  const [actionNote, setActionNote] = useState("");
  const timeline = deriveIncidentTimeline(incident);
  const incidentCount = timeline.length;
  const serviceOnlyEvents = incident.serviceEvents.filter((event) => event.kind !== "browser_health_changed");
  const handlingState = incidentHandlingState(incident);
  const priority = incidentPriorityView(incident);
  const actionsAvailable = Boolean(onAcknowledge || onResolve);

  useEffect(() => {
    setActionNote("");
  }, [incident.id]);

  return (
    <div className="service-event-dialog-body">
      <p className="service-event-dialog-message">{incident.latestMessage}</p>
      <div className="service-incident-priority-row">
        <div className={cn("service-incident-priority-card", `service-incident-priority-${priority.severityTone}`)}>
          <span>Severity</span>
          <strong>{priority.severityLabel}</strong>
        </div>
        <div className="service-incident-priority-card service-incident-priority-escalation">
          <span>Escalation</span>
          <strong>{priority.escalationLabel}</strong>
        </div>
      </div>
      {priority.recommendedAction && (
        <div className={cn("service-incident-recommended-action", `service-incident-priority-${priority.severityTone}`)}>
          <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
          <div>
            <span>Recommended action</span>
            <p>{priority.recommendedAction}</p>
          </div>
        </div>
      )}
      <div className="service-event-detail-grid">
        <EventDetailItem label="Browser" value={incident.browserId} />
        <EventDetailItem label="Severity" value={priority.severityLabel} />
        <EventDetailItem label="Escalation" value={priority.escalationLabel} />
        <EventDetailItem label="Latest kind" value={formatEventKind(incident.latestKind)} />
        <EventDetailItem label="Current health" value={incident.currentHealth} />
        <EventDetailItem label="Handling state" value={incidentHandlingLabel(incident)} />
        <EventDetailItem label="Incident count" value={String(incidentCount)} />
        <EventDetailItem label="Acknowledged by" value={incident.acknowledgedBy} />
        <EventDetailItem label="Acknowledged" value={incident.acknowledgedAt ? formatAbsoluteTime(incident.acknowledgedAt) : null} />
        <EventDetailItem label="Resolved by" value={incident.resolvedBy} />
        <EventDetailItem label="Resolved" value={incident.resolvedAt ? formatAbsoluteTime(incident.resolvedAt) : null} />
      </div>
      {(incident.acknowledgementNote || incident.resolutionNote) && (
        <div className="service-incident-notes">
          {incident.acknowledgementNote && (
            <p>
              <span>Acknowledgement note</span>
              {incident.acknowledgementNote}
            </p>
          )}
          {incident.resolutionNote && (
            <p>
              <span>Resolution note</span>
              {incident.resolutionNote}
            </p>
          )}
        </div>
      )}
      {actionsAvailable && handlingState !== "resolved" && (
        <div className="service-incident-action-note">
          <label htmlFor={`service-incident-action-note-${incident.id}`}>
            Operator note
          </label>
          <textarea
            id={`service-incident-action-note-${incident.id}`}
            value={actionNote}
            onChange={(event) => setActionNote(event.target.value)}
            placeholder="Optional context for the acknowledgement or resolution"
            rows={3}
          />
        </div>
      )}
      {actionsAvailable && (
        <div className="service-incident-actions">
          {handlingState === "unacknowledged" && onAcknowledge && (
            <Button
              type="button"
              variant="outline"
              className="rounded-full"
              disabled={acting}
              onClick={() => onAcknowledge(incident, actionNote)}
            >
              {acting ? <Loader2 className="size-3.5 animate-spin" /> : <CheckCircle2 className="size-3.5" />}
              Mark acknowledged
            </Button>
          )}
          {handlingState !== "resolved" && onResolve && (
            <Button
              type="button"
              className="rounded-full"
              disabled={acting}
              onClick={() => onResolve(incident, actionNote)}
            >
              {acting ? <Loader2 className="size-3.5 animate-spin" /> : <ShieldCheck className="size-3.5" />}
              Mark resolved
            </Button>
          )}
        </div>
      )}
      {timeline.length > 0 && (
        <div>
          <p className="mb-2 text-[10px] font-black uppercase tracking-[0.18em] text-muted-foreground">
            Incident history
          </p>
          <div className="service-incident-history">
            {timeline.map((item) => (
              <div key={item.id} className="service-incident-history-item">
                <EventDot kind={item.kind} />
                <div className="min-w-0 flex-1">
                  <div className="flex min-w-0 items-center gap-2">
                    <span className="truncate text-xs font-bold text-foreground">
                      {item.title}
                    </span>
                    {item.source && (
                      <Badge variant="outline" className="rounded-full px-1.5 py-0 text-[9px] uppercase">
                        {item.source}
                      </Badge>
                    )}
                    <span className="ml-auto shrink-0 text-[10px] text-muted-foreground">
                      {formatAbsoluteTime(item.timestamp)}
                    </span>
                  </div>
                  {item.message && (
                    <p className="mt-1 text-xs leading-5 text-muted-foreground">
                      {item.message}
                    </p>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
      {incident.transitionEvents.length > 0 && (
        <div>
          <p className="mb-2 text-[10px] font-black uppercase tracking-[0.18em] text-muted-foreground">
            Health transitions
          </p>
          <div className="space-y-1">
            {incident.transitionEvents.map((event) => (
              <div key={event.id} className="service-incident-entry">
                <div className="flex items-center gap-2">
                  <EventDot kind={event.kind} />
                  <span className="truncate text-xs font-bold text-foreground">
                    {formatHealthLabel(event.previousHealth)} to {formatHealthLabel(event.currentHealth)}
                  </span>
                  <span className="ml-auto shrink-0 text-[10px] text-muted-foreground">
                    {formatAbsoluteTime(event.timestamp)}
                  </span>
                </div>
                <p className="mt-1 text-xs leading-5 text-muted-foreground">{event.message}</p>
              </div>
            ))}
          </div>
        </div>
      )}
      {serviceOnlyEvents.length > 0 && (
        <div>
          <p className="mb-2 text-[10px] font-black uppercase tracking-[0.18em] text-muted-foreground">
            Related service events
          </p>
          <div className="space-y-1">
            {serviceOnlyEvents.map((event) => (
              <div key={event.id} className="service-incident-entry">
                <div className="flex items-center gap-2">
                  <EventDot kind={event.kind} />
                  <span className="truncate text-xs font-bold text-foreground">
                    {formatEventKind(event.kind)}
                  </span>
                  <span className="ml-auto shrink-0 text-[10px] text-muted-foreground">
                    {formatAbsoluteTime(event.timestamp)}
                  </span>
                </div>
                <p className="mt-1 text-xs leading-5 text-muted-foreground">{event.message}</p>
              </div>
            ))}
          </div>
        </div>
      )}
      {incident.jobEvents.length > 0 && (
        <div>
          <p className="mb-2 text-[10px] font-black uppercase tracking-[0.18em] text-muted-foreground">
            Related jobs
          </p>
          <div className="space-y-1">
            {incident.jobEvents.map((event) => (
              <div key={event.id} className="service-incident-entry">
                <div className="flex items-center gap-2">
                  <EventDot kind={event.kind} />
                  <span className="truncate text-xs font-bold text-foreground">
                    {formatEventKind(event.kind)}
                  </span>
                  <span className="ml-auto shrink-0 text-[10px] text-muted-foreground">
                    {formatAbsoluteTime(event.timestamp)}
                  </span>
                </div>
                <p className="mt-1 text-xs leading-5 text-muted-foreground">{event.message}</p>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

export function ServicePanel({
  onBrowserInspect,
  onInspectSelection,
  onInspectorActionsChange,
}: ServicePanelProps = {}) {
  const activePort = useAtomValue(activePortAtom);
  const activeSession = useAtomValue(activeSessionNameAtom);
  const [status, setStatus] = useState<ServiceStatusData | null>(null);
  const [events, setEvents] = useState<ServiceEventsData | null>(null);
  const [jobs, setJobs] = useState<ServiceJobsData | null>(null);
  const [incidents, setIncidents] = useState<ServiceIncidentsData | null>(null);
  const [trace, setTrace] = useState<ServiceTraceData | null>(null);
  const [traceLoading, setTraceLoading] = useState(false);
  const [traceError, setTraceError] = useState("");
  const [traceFilters, setTraceFilters] = useState<TraceFilters>({
    serviceName: "",
    agentName: "",
    taskName: "",
    browserId: "",
    profileId: "",
    sessionId: "",
    since: "",
    limit: 20,
  });
  const [loading, setLoading] = useState(false);
  const [reconciling, setReconciling] = useState(false);
  const [error, setError] = useState("");
  const [eventKind, setEventKind] = useState<EventKindFilter>("all");
  const [eventWindow, setEventWindow] = useState<EventWindowFilter>("all");
  const [eventLimit, setEventLimit] = useState<EventLimit>(8);
  const [eventBrowserId, setEventBrowserId] = useState("");
  const [sessionTabQuery, setSessionTabQuery] = useState("");
  const [sessionLimit, setSessionLimit] = useState<ServiceRecordLimit>(24);
  const [tabLimit, setTabLimit] = useState<ServiceRecordLimit>(24);
  const [profileAllocationQuery, setProfileAllocationQuery] = useState("");
  const [profileAllocationLimit, setProfileAllocationLimit] = useState<ServiceRecordLimit>(24);
  const [incidentQuery, setIncidentQuery] = useState("");
  const [incidentLimit, setIncidentLimit] = useState<ServiceRecordLimit>(24);
  const [workspaceTab, setWorkspaceTab] = useState<ServiceWorkspaceTab>("incidents");
  const [cleanupLoading, setCleanupLoading] = useState<RetainedCleanupKind | null>(null);
  const [cleanupApplying, setCleanupApplying] = useState<RetainedCleanupKind | null>(null);
  const [cleanupKind, setCleanupKind] = useState<RetainedCleanupKind | null>(null);
  const [cleanupResult, setCleanupResult] = useState<RetainedCleanupResult | null>(null);
  const [cleanupError, setCleanupError] = useState("");
  const [incidentOnly, setIncidentOnly] = useState(false);
  const [incidentHandlingFilter, setIncidentHandlingFilter] = useState<IncidentHandlingFilter>("all");
  const [actingIncidentId, setActingIncidentId] = useState<string | null>(null);
  const [operatorIdentity, setOperatorIdentity] = useState(initialOperatorIdentity);
  const [selectedIncident, setSelectedIncident] = useState<IncidentRecord | null>(null);
  const [selectedIncidentActivity, setSelectedIncidentActivity] = useState<ServiceTraceTimelineItem[] | null>(null);
  const [selectedIncidentActivityLoading, setSelectedIncidentActivityLoading] = useState(false);
  const [selectedIncidentActivityError, setSelectedIncidentActivityError] = useState("");
  const [selectedEvent, setSelectedEvent] = useState<ServiceEvent | null>(null);
  const [selectedBrowser, setSelectedBrowser] = useState<ServiceBrowser | null>(null);
  const [selectedBrowserId, setSelectedBrowserId] = useState<string | null>(null);
  const [selectedSession, setSelectedSession] = useState<ServiceSession | null>(null);
  const [selectedTab, setSelectedTab] = useState<ServiceTab | null>(null);
  const [selectedViewStream, setSelectedViewStream] = useState<SelectedViewStream | null>(null);
  const [viewStreamFullscreen, setViewStreamFullscreen] = useState(false);
  const [selectedJob, setSelectedJob] = useState<ServiceJob | null>(null);
  const [selectedProfileAllocation, setSelectedProfileAllocation] = useState<ServiceProfileAllocation | null>(null);
  const [selectedProfileAllocationLoading, setSelectedProfileAllocationLoading] = useState(false);
  const [selectedProfileAllocationError, setSelectedProfileAllocationError] = useState("");
  const profileAllocationLookupId = useRef(0);

  const canFetch = typeof window !== "undefined";
  const activeFilterCount =
    (eventKind === "all" ? 0 : 1) +
    (eventWindow === "all" ? 0 : 1) +
    (eventBrowserId.trim() ? 1 : 0) +
    (eventLimit === 8 ? 0 : 1) +
    (incidentOnly ? 1 : 0);

  const fetchService = useCallback(async (showSpinner: boolean) => {
    if (!canFetch) return;
    if (showSpinner) setLoading(true);
    setError("");
    try {
      const params = new URLSearchParams({ limit: String(eventLimit) });
      if (eventKind !== "all") params.set("kind", eventKind);
      if (eventBrowserId.trim()) params.set("browser-id", eventBrowserId.trim());
      const windowOption = EVENT_WINDOW_OPTIONS.find((option) => option.value === eventWindow);
      if (windowOption?.milliseconds) {
        params.set("since", new Date(Date.now() - windowOption.milliseconds).toISOString());
      }
      const [statusResp, jobsResp, eventsResp, incidentsResp] = await Promise.all([
        fetch(`${serviceBase(activePort)}/status`),
        fetch(`${serviceBase(activePort)}/jobs?limit=8`),
        fetch(`${serviceBase(activePort)}/events?${params.toString()}`),
        fetch(`${serviceBase(activePort)}/incidents?summary=true&limit=50`),
      ]);
      const statusJson = (await statusResp.json()) as ApiResponse<ServiceStatusData>;
      const jobsJson = (await jobsResp.json()) as ApiResponse<ServiceJobsData>;
      const eventsJson = (await eventsResp.json()) as ApiResponse<ServiceEventsData>;
      const incidentsJson = (await incidentsResp.json()) as ApiResponse<ServiceIncidentsData>;
      if (!statusJson.success) throw new Error(statusJson.error || "Service status failed");
      if (!jobsJson.success) throw new Error(jobsJson.error || "Service jobs failed");
      if (!eventsJson.success) throw new Error(eventsJson.error || "Service events failed");
      setStatus(statusJson.data ?? null);
      setJobs(jobsJson.data ?? null);
      setEvents(eventsJson.data ?? null);
      setIncidents(incidentsJson.success ? incidentsJson.data ?? null : null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Service API unavailable");
    } finally {
      if (showSpinner) setLoading(false);
    }
  }, [activePort, canFetch, eventBrowserId, eventKind, eventLimit, eventWindow]);

  useEffect(() => {
    setStatus(null);
    setJobs(null);
    setEvents(null);
    setIncidents(null);
    setTrace(null);
    setTraceError("");
    setError("");
    if (!canFetch) return;
    fetchService(true);
    const timer = setInterval(() => {
      if (document.visibilityState === "visible") fetchService(false);
    }, 7000);
    return () => clearInterval(timer);
  }, [canFetch, fetchService]);

  useEffect(() => {
    const trimmed = operatorIdentity.trim();
    if (trimmed) {
      window.localStorage.setItem(OPERATOR_STORAGE_KEY, trimmed);
    } else {
      window.localStorage.removeItem(OPERATOR_STORAGE_KEY);
    }
  }, [operatorIdentity]);

  const loadTrace = useCallback(async () => {
    if (!canFetch || traceLoading) return;
    setTraceLoading(true);
    setTraceError("");
    try {
      const params = new URLSearchParams({ limit: String(traceFilters.limit) });
      if (traceFilters.serviceName.trim()) params.set("service-name", traceFilters.serviceName.trim());
      if (traceFilters.agentName.trim()) params.set("agent-name", traceFilters.agentName.trim());
      if (traceFilters.taskName.trim()) params.set("task-name", traceFilters.taskName.trim());
      if (traceFilters.browserId.trim()) params.set("browser-id", traceFilters.browserId.trim());
      if (traceFilters.profileId.trim()) params.set("profile-id", traceFilters.profileId.trim());
      if (traceFilters.sessionId.trim()) params.set("session-id", traceFilters.sessionId.trim());
      if (traceFilters.since.trim()) params.set("since", traceFilters.since.trim());
      const resp = await fetch(`${serviceBase(activePort)}/trace?${params.toString()}`);
      const json = (await resp.json()) as ApiResponse<ServiceTraceData | ServiceTraceToolPayload>;
      if (!json.success) throw new Error(json.error || "Service trace failed");
      setTrace(normalizeServiceTraceData(json.data));
    } catch (err) {
      setTraceError(err instanceof Error ? err.message : "Service trace unavailable");
    } finally {
      setTraceLoading(false);
    }
  }, [activePort, canFetch, traceFilters, traceLoading]);

  const clearTrace = useCallback(() => {
    setTrace(null);
    setTraceError("");
    setTraceFilters({
      serviceName: "",
      agentName: "",
      taskName: "",
      browserId: "",
      profileId: "",
      sessionId: "",
      since: "",
      limit: 20,
    });
  }, []);

  useEffect(() => {
    setSelectedIncidentActivity(null);
    setSelectedIncidentActivityError("");
    if (!canFetch || !selectedIncident?.id) {
      setSelectedIncidentActivityLoading(false);
      return;
    }

    let cancelled = false;
    setSelectedIncidentActivityLoading(true);
    fetch(`${serviceBase(activePort)}/incidents/${encodeURIComponent(selectedIncident.id)}/activity`)
      .then(async (resp) => {
        const json = (await resp.json()) as ApiResponse<ServiceIncidentActivityData>;
        if (!json.success) throw new Error(json.error || "Service incident activity failed");
        if (!cancelled) {
          setSelectedIncidentActivity(json.data?.activity ?? []);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setSelectedIncidentActivityError(
            err instanceof Error ? err.message : "Service incident activity unavailable",
          );
        }
      })
      .finally(() => {
        if (!cancelled) setSelectedIncidentActivityLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [activePort, canFetch, selectedIncident?.id]);

  const reconcile = useCallback(async () => {
    if (!canFetch || reconciling) return;
    setReconciling(true);
    setError("");
    try {
      const resp = await fetch(`${serviceBase(activePort)}/reconcile`, { method: "POST" });
      const json = (await resp.json()) as ApiResponse<{ service_state?: ServiceState }>;
      if (!json.success) throw new Error(json.error || "Service reconcile failed");
      await fetchService(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Service reconcile unavailable");
    } finally {
      setReconciling(false);
    }
  }, [activePort, canFetch, fetchService, reconciling]);

  const inspectJob = useCallback(async (job: ServiceJob) => {
    if (!canFetch || !job.id) {
      if (onInspectSelection) {
        onInspectSelection({ kind: "job", job });
      } else {
        setSelectedJob(job);
      }
      return;
    }
    setError("");
    try {
      const resp = await fetch(`${serviceBase(activePort)}/jobs/${encodeURIComponent(job.id)}`);
      const json = (await resp.json()) as ApiResponse<ServiceJobsData>;
      if (!json.success) throw new Error(json.error || "Service job lookup failed");
      const selected = json.data?.job ?? job;
      if (onInspectSelection) {
        onInspectSelection({ kind: "job", job: selected });
      } else {
        setSelectedJob(selected);
      }
    } catch (err) {
      if (onInspectSelection) {
        onInspectSelection({ kind: "job", job });
      } else {
        setSelectedJob(job);
      }
      setError(err instanceof Error ? err.message : "Service job lookup unavailable");
    }
  }, [activePort, canFetch, onInspectSelection]);

  const inspectProfileAllocation = useCallback(async (allocation: ServiceProfileAllocation) => {
    const lookupId = profileAllocationLookupId.current + 1;
    profileAllocationLookupId.current = lookupId;
    if (onInspectSelection) {
      onInspectSelection({ kind: "profile", allocation });
    } else {
      setSelectedProfileAllocation(allocation);
    }
    setSelectedProfileAllocationError("");
    if (!canFetch || !allocation.profileId) return;
    if (!onInspectSelection) setSelectedProfileAllocationLoading(true);
    try {
      const resp = await fetch(serviceProfileAllocationLookupUrl(serviceBase(activePort), allocation.profileId));
      const json = (await resp.json()) as ApiResponse<ServiceProfileAllocationData>;
      if (profileAllocationLookupId.current === lookupId) {
        const selected = profileAllocationFromLookupPayload(json, allocation);
        if (onInspectSelection) {
          onInspectSelection({ kind: "profile", allocation: selected });
        } else {
          setSelectedProfileAllocation(selected);
        }
      }
    } catch (err) {
      if (profileAllocationLookupId.current === lookupId) {
        if (onInspectSelection) {
          onInspectSelection({ kind: "profile", allocation });
        } else {
          setSelectedProfileAllocation(allocation);
        }
        setSelectedProfileAllocationError(
          err instanceof Error ? err.message : "Service profile allocation lookup unavailable",
        );
      }
    } finally {
      if (profileAllocationLookupId.current === lookupId) {
        setSelectedProfileAllocationLoading(false);
      }
    }
  }, [activePort, canFetch, onInspectSelection]);

  const inspectIncident = useCallback((incident: IncidentRecord) => {
    if (onInspectSelection) {
      onInspectSelection({ kind: "incident", incident });
      return;
    }
    setSelectedIncident(incident);
  }, [onInspectSelection]);

  const cancelJob = useCallback(async (job: ServiceJob): Promise<ServiceJob | null> => {
    if (!canFetch || !job.id) return null;
    setError("");
    try {
      const resp = await fetch(`${serviceBase(activePort)}/jobs/${encodeURIComponent(job.id)}/cancel`, {
        method: "POST",
      });
      const json = (await resp.json()) as ApiResponse<ServiceJobsData & { cancelled?: boolean }>;
      if (!json.success) throw new Error(json.error || "Service job cancel failed");
      const selected = json.data?.job ?? { ...job, error: "Cancellation requested" };
      setSelectedJob(selected);
      await fetchService(false);
      return selected;
    } catch (err) {
      setError(err instanceof Error ? err.message : "Service job cancel unavailable");
      return null;
    }
  }, [activePort, canFetch, fetchService]);

  const runRetainedCleanup = useCallback(async (kind: RetainedCleanupKind, apply: boolean) => {
    if (!canFetch) return;
    setCleanupError("");
    if (apply) {
      setCleanupApplying(kind);
    } else {
      setCleanupLoading(kind);
    }
    try {
      const params = kind === "prune"
        ? {
            apply,
            closedTabs: true,
            notStartedBrowsers: true,
            processExitedBrowsers: false,
            releasedSessions: false,
            abandonedSessions: false,
          }
        : {
            apply,
            missingLeaseObservedAt: true,
          };
      const resp = await fetch(`${serviceBase(activePort)}/request`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: kind === "prune" ? "service_prune_retained" : "service_repair_retained",
          serviceName: "agent-browser-dashboard",
          agentName: operatorIdentity.trim() || activeSession || "operator",
          taskName: apply ? `apply-retained-${kind}` : `dry-run-retained-${kind}`,
          params,
          jobTimeoutMs: 10000,
        }),
      });
      const json = (await resp.json()) as ApiResponse<RetainedCleanupResult>;
      if (!json.success) throw new Error(json.error || `Retained ${kind} request failed`);
      setCleanupKind(kind);
      setCleanupResult(json.data ?? null);
      if (apply) {
        await fetchService(false);
      }
    } catch (err) {
      setCleanupError(err instanceof Error ? err.message : `Retained ${kind} request failed`);
    } finally {
      setCleanupLoading(null);
      setCleanupApplying(null);
    }
  }, [activePort, activeSession, canFetch, fetchService, operatorIdentity]);

  const handleIncident = useCallback(async (
    incident: IncidentRecord,
    action: "acknowledge" | "resolve",
    note: string,
    closeDialog = true,
  ) => {
    if (!canFetch || !incident.id) return false;
    setActingIncidentId(incident.id);
    setError("");
    try {
      const params = new URLSearchParams({
        by: operatorIdentity.trim() || activeSession || "dashboard",
      });
      const trimmedNote = note.trim();
      if (trimmedNote) params.set("note", trimmedNote);
      const resp = await fetch(
        `${serviceBase(activePort)}/incidents/${encodeURIComponent(incident.id)}/${action}?${params.toString()}`,
        { method: "POST" },
      );
      const json = (await resp.json()) as ApiResponse<{ incident?: ServiceIncident }>;
      if (!json.success) throw new Error(json.error || `Service incident ${action} failed`);
      if (closeDialog) setSelectedIncident(null);
      await fetchService(false);
      return true;
    } catch (err) {
      setError(err instanceof Error ? err.message : `Service incident ${action} unavailable`);
      return false;
    } finally {
      setActingIncidentId(null);
    }
  }, [activePort, activeSession, canFetch, fetchService, operatorIdentity]);

  const cancelInspectorJob = useCallback(async (job: ServiceJob) => {
    const selected = await cancelJob(job);
    if (selected && onInspectSelection) {
      onInspectSelection({ kind: "job", job: selected });
    }
  }, [cancelJob, onInspectSelection]);

  const updateInspectorIncident = useCallback((
    incident: IncidentRecord,
    action: "acknowledge" | "resolve",
    note: string,
  ) => {
    const now = new Date().toISOString();
    const actor = operatorIdentity.trim() || activeSession || "dashboard";
    const trimmedNote = note.trim();
    const selected = action === "acknowledge"
      ? {
          ...incident,
          acknowledgedAt: incident.acknowledgedAt ?? now,
          acknowledgedBy: incident.acknowledgedBy ?? actor,
          acknowledgementNote: trimmedNote || incident.acknowledgementNote,
        }
      : {
          ...incident,
          resolvedAt: incident.resolvedAt ?? now,
          resolvedBy: incident.resolvedBy ?? actor,
          resolutionNote: trimmedNote || incident.resolutionNote,
        };
    onInspectSelection?.({ kind: "incident", incident: selected });
  }, [activeSession, onInspectSelection, operatorIdentity]);

  const acknowledgeInspectorIncident = useCallback(async (incident: IncidentRecord, note: string) => {
    if (await handleIncident(incident, "acknowledge", note, false)) {
      updateInspectorIncident(incident, "acknowledge", note);
    }
  }, [handleIncident, updateInspectorIncident]);

  const resolveInspectorIncident = useCallback(async (incident: IncidentRecord, note: string) => {
    if (await handleIncident(incident, "resolve", note, false)) {
      updateInspectorIncident(incident, "resolve", note);
    }
  }, [handleIncident, updateInspectorIncident]);

  useEffect(() => {
    if (!onInspectorActionsChange) return;
    onInspectorActionsChange({
      actingIncidentId,
      onAcknowledgeIncident: acknowledgeInspectorIncident,
      onResolveIncident: resolveInspectorIncident,
      onCancelJob: cancelInspectorJob,
    });
  }, [
    acknowledgeInspectorIncident,
    actingIncidentId,
    cancelInspectorJob,
    onInspectorActionsChange,
    resolveInspectorIncident,
  ]);

  const serviceState = status?.service_state;
  const control = status?.control_plane;
  const serviceJobTimeoutMs =
    control?.service_job_timeout_ms ?? serviceState?.controlPlane?.serviceJobTimeoutMs ?? null;
  const reconciliation = serviceState?.reconciliation;
  const retainedServiceJobs = useMemo(
    () => Object.values(serviceState?.jobs ?? {}),
    [serviceState?.jobs],
  );
  const recentJobs = jobs?.jobs ?? retainedServiceJobs.slice(-8);
  const recentEvents = events?.events ?? serviceState?.events?.slice(-8) ?? [];
  const jobIncidentEvents = useMemo(() => deriveJobIncidentEvents(recentJobs), [recentJobs]);
  const visibleEvents = useMemo(() => {
    const merged = [...recentEvents, ...jobIncidentEvents]
      .sort((left, right) => new Date(right.timestamp).getTime() - new Date(left.timestamp).getTime())
      .slice(0, eventLimit);
    return incidentOnly ? merged.filter(isIncidentEvent) : merged;
  }, [eventLimit, incidentOnly, jobIncidentEvents, recentEvents]);
  const healthTransitionEvents = useMemo(
    () =>
      (serviceState?.events ?? events?.events ?? [])
        .filter((event) => event.kind === "browser_health_changed")
        .slice(-6)
        .reverse(),
    [events?.events, serviceState?.events],
  );
  const jobSummary =
    jobs?.matched !== undefined && jobs?.total !== undefined
      ? `${jobs.matched} of ${jobs.total} matched`
      : `Last ${recentJobs.length} retained service jobs`;
  const jobActivitySummary = useMemo(() => {
    const active = retainedServiceJobs.filter(isActiveServiceJob).length;
    const terminal = retainedServiceJobs.filter(isRetainedTerminalServiceJob).length;
    const retained = Math.max(0, retainedServiceJobs.length - active);
    return { active, terminal, retained };
  }, [retainedServiceJobs]);
  const eventSummary =
    incidentOnly
      ? `${visibleEvents.length} incident events shown`
      : events?.matched !== undefined && events?.total !== undefined
        ? `${events.matched} of ${events.total} matched`
        : `Last ${visibleEvents.length} retained service events`;
  const browserRecords = useMemo(
    () => Object.values(serviceState?.browsers ?? {}),
    [serviceState?.browsers],
  );
  const traceTimeline = useMemo(() => traceTimelineItems(trace), [trace]);
  const incidentRecords = useMemo(
    () =>
      deriveIncidentRecords(
        serviceState?.incidents ?? [],
        serviceState?.events ?? [],
        retainedServiceJobs,
      ),
    [retainedServiceJobs, serviceState?.events, serviceState?.incidents],
  );
  const incidentQueryText = incidentQuery.trim().toLowerCase();
  const filteredIncidentRecords = useMemo(() => {
    const handlingFiltered = incidentHandlingFilter === "all"
      ? incidentRecords
      : incidentRecords.filter((incident) => incidentHandlingState(incident) === incidentHandlingFilter);
    return incidentQueryText
      ? handlingFiltered.filter((incident) => includesQuery(incidentSearchText(incident), incidentQueryText))
      : handlingFiltered;
  }, [incidentHandlingFilter, incidentQueryText, incidentRecords]);
  const visibleIncidentRecords = useMemo(
    () => filteredIncidentRecords.slice(0, incidentLimit),
    [filteredIncidentRecords, incidentLimit],
  );
  const hiddenIncidentCount = Math.max(0, filteredIncidentRecords.length - visibleIncidentRecords.length);
  const incidentHandlingSummary = useMemo(() => ({
    unacknowledged: incidentRecords.filter((incident) => incidentHandlingState(incident) === "unacknowledged").length,
    acknowledged: incidentRecords.filter((incident) => incidentHandlingState(incident) === "acknowledged").length,
    resolved: incidentRecords.filter((incident) => incidentHandlingState(incident) === "resolved").length,
  }), [incidentRecords]);
  const incidentSummaryGroups = useMemo(
    () => incidentSummaryGroupViews(incidents?.summary?.groups ?? []),
    [incidents?.summary?.groups],
  );
  const sessionRecords = useMemo(
    () => Object.values(serviceState?.sessions ?? {}),
    [serviceState?.sessions],
  );
  const profileAllocations = useMemo(
    () => status?.profileAllocations ?? [],
    [status?.profileAllocations],
  );
  const profileAllocationQueryText = profileAllocationQuery.trim().toLowerCase();
  const filteredProfileAllocations = useMemo(
    () =>
      profileAllocationQueryText
        ? profileAllocations.filter((allocation) => includesQuery(profileAllocationSearchText(allocation), profileAllocationQueryText))
        : profileAllocations,
    [profileAllocationQueryText, profileAllocations],
  );
  const visibleProfileAllocations = useMemo(
    () => filteredProfileAllocations.slice(0, profileAllocationLimit),
    [filteredProfileAllocations, profileAllocationLimit],
  );
  const hiddenProfileAllocationCount = Math.max(0, filteredProfileAllocations.length - visibleProfileAllocations.length);
  const tabRecords = useMemo(
    () => Object.values(serviceState?.tabs ?? {}),
    [serviceState?.tabs],
  );
  const sessionTabQueryText = sessionTabQuery.trim().toLowerCase();
  const filteredSessionRecords = useMemo(
    () =>
      sessionTabQueryText
        ? sessionRecords.filter((session) => includesQuery(sessionSearchText(session), sessionTabQueryText))
        : sessionRecords,
    [sessionRecords, sessionTabQueryText],
  );
  const filteredTabRecords = useMemo(
    () =>
      sessionTabQueryText
        ? tabRecords.filter((tab) => includesQuery(tabSearchText(tab), sessionTabQueryText))
        : tabRecords,
    [tabRecords, sessionTabQueryText],
  );
  const visibleSessionRecords = useMemo(
    () => filteredSessionRecords.slice(0, sessionLimit),
    [filteredSessionRecords, sessionLimit],
  );
  const visibleTabRecords = useMemo(
    () => filteredTabRecords.slice(0, tabLimit),
    [filteredTabRecords, tabLimit],
  );
  const hiddenSessionCount = Math.max(0, filteredSessionRecords.length - visibleSessionRecords.length);
  const hiddenTabCount = Math.max(0, filteredTabRecords.length - visibleTabRecords.length);
  const sessionActivitySummary = useMemo(() => {
    const activeSessions = filteredSessionRecords.filter(isActiveServiceSession).length;
    const activeTabs = filteredTabRecords.filter(isActiveServiceTab).length;
    return {
      activeSessions,
      retainedSessions: Math.max(0, filteredSessionRecords.length - activeSessions),
      activeTabs,
      retainedTabs: Math.max(0, filteredTabRecords.length - activeTabs),
    };
  }, [filteredSessionRecords, filteredTabRecords]);
  const browserById = useMemo(
    () => new Map(browserRecords.map((browser) => [browser.id, browser])),
    [browserRecords],
  );
  const tabIndexById = useMemo(() => {
    const grouped = new Map<string, ServiceTab[]>();
    for (const tab of tabRecords) {
      if (!tab.browserId) continue;
      grouped.set(tab.browserId, [...(grouped.get(tab.browserId) ?? []), tab]);
    }
    const indexes = new Map<string, number>();
    for (const tabs of grouped.values()) {
      tabs.forEach((tab, index) => indexes.set(tab.id, index));
    }
    return indexes;
  }, [tabRecords]);
  const openViewStream = useCallback(
    (stream: ServiceViewStream, browser: ServiceBrowser, tab?: ServiceTab | null, focusMessage?: string | null) => {
      setSelectedViewStream({ stream, browser, tab, focusMessage });
      setViewStreamFullscreen(false);
    },
    [],
  );
  const inspectBrowser = useCallback((browser: ServiceBrowser) => {
    setSelectedBrowserId(browser.id || null);
    if (onInspectSelection) {
      onInspectSelection({ kind: "browser", browser });
      return;
    }
    if (onBrowserInspect) {
      onBrowserInspect(browser);
      return;
    }
    setSelectedBrowser(browser);
  }, [onBrowserInspect, onInspectSelection]);
  const inspectSession = useCallback((session: ServiceSession) => {
    if (onInspectSelection) {
      onInspectSelection({ kind: "session", session });
      return;
    }
    setSelectedSession(session);
  }, [onInspectSelection]);
  const inspectTab = useCallback((tab: ServiceTab) => {
    const viewStreamAvailable = Boolean(tab.browserId && browserPrimaryViewStream(browserById.get(tab.browserId)));
    if (onInspectSelection) {
      onInspectSelection({ kind: "tab", tab, viewStreamAvailable });
      return;
    }
    setSelectedTab(tab);
  }, [browserById, onInspectSelection]);
  const inspectTabViewStream = useCallback(async (tab: ServiceTab) => {
    const browser = tab.browserId ? browserById.get(tab.browserId) : null;
    const stream = browserPrimaryViewStream(browser);
    if (!browser || !stream) {
      setError("No view stream is registered for this tab's browser.");
      return;
    }

    let focusMessage: string | null = null;
    const tabIndex = tabIndexById.get(tab.id);
    if (canFetch && tabIndex !== undefined) {
      try {
        const resp = await fetch(`${serviceBase(activePort)}/request`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            action: "view_focus",
            serviceName: "agent-browser-dashboard",
            agentName: operatorIdentity.trim() || activeSession || "operator",
            taskName: "inspect-hidden-rdp-tab",
            params: { index: tabIndex, maximize: true },
            jobTimeoutMs: 5000,
          }),
        });
        const json = (await resp.json()) as ApiResponse<ServiceJobsData>;
        if (!json.success) {
          focusMessage = json.error || "Remote-view focus request was not accepted; opening the stream anyway.";
        }
      } catch (err) {
        focusMessage = err instanceof Error
          ? `Remote-view focus request failed: ${err.message}`
          : "Remote-view focus request failed; opening the stream anyway.";
      }
    } else {
      focusMessage = "No stable tab index was available; opening the stream without a queued focus request.";
    }

    openViewStream(stream, browser, tab, focusMessage);
  }, [activePort, activeSession, browserById, canFetch, openViewStream, operatorIdentity, tabIndexById]);
  const entityCounts = useMemo(() => ({
    browsers: countEntries(serviceState?.browsers),
    profiles: countEntries(serviceState?.profiles),
    jobs: countEntries(serviceState?.jobs),
    sessions: countEntries(serviceState?.sessions),
    tabs: countEntries(serviceState?.tabs),
    policies: countEntries(serviceState?.sitePolicies),
    providers: countEntries(serviceState?.providers),
  }), [serviceState]);
  const retainedStateCleanupNeeded =
    entityCounts.browsers > 100 ||
    entityCounts.profiles > 100 ||
    entityCounts.sessions > 100 ||
    entityCounts.tabs > 100 ||
    entityCounts.jobs > 100;
  const managedRecordDetail = useMemo(() => [
    `${entityCounts.browsers} retained browser records`,
    `${entityCounts.profiles} managed profile records`,
    `${entityCounts.sessions} service sessions`,
    `${entityCounts.tabs} tracked tabs`,
    `${jobs?.total ?? entityCounts.jobs} jobs (${jobs?.count ?? recentJobs.length} recent shown)`,
    `${entityCounts.policies} site policies`,
    `${entityCounts.providers} providers`,
  ].join("; "), [entityCounts, jobs?.count, jobs?.total, recentJobs.length]);
  const cleanupCandidateTotal = cleanupTotal(cleanupResult?.candidateCounts);
  const cleanupApplyEnabled = Boolean(cleanupKind && cleanupResult?.dryRun && cleanupCandidateTotal > 0);
  const cleanupAppliedTotal = cleanupKind === "repair"
    ? cleanupTotal(cleanupResult?.repairedCounts)
    : cleanupTotal(cleanupResult?.removed);
  const cleanupApplyLabel = cleanupKind ? `Apply reviewed ${cleanupKind}` : "Apply reviewed cleanup";
  const cleanupDialogTitle = cleanupKind === "repair"
    ? "Apply retained-state repair?"
    : "Apply retained-state prune?";
  const cleanupDialogDescription = cleanupKind === "repair"
    ? "This will mutate retained session evidence based on the reviewed dry-run result. It does not launch Chrome, but it changes service state."
    : "This will remove inert retained records based on the reviewed dry-run result. Failure evidence is preserved unless the dry-run selected removable candidates.";
  const workspaceTabs = useMemo(() => [
    {
      value: "profiles" as const,
      label: "Profiles",
      count: filteredProfileAllocations.length,
      detail: "profile routing rows",
    },
    {
      value: "incidents" as const,
      label: "Incidents",
      count: filteredIncidentRecords.length,
      detail: "grouped incident records",
      tone: incidentHandlingSummary.unacknowledged > 0 ? "bad" : "neutral",
    },
    {
      value: "sessions" as const,
      label: "Sessions",
      count: sessionActivitySummary.activeSessions + sessionActivitySummary.activeTabs,
      detail: `${sessionActivitySummary.retainedSessions + sessionActivitySummary.retainedTabs} retained`,
    },
    {
      value: "jobs" as const,
      label: "Jobs",
      count: jobActivitySummary.active,
      detail: `${jobActivitySummary.retained} retained`,
      tone: jobActivitySummary.active > 0 ? "warn" : "neutral",
    },
    {
      value: "events" as const,
      label: "Events",
      count: visibleEvents.length,
      detail: "events, health, and trace",
      tone: incidentOnly ? "warn" : "neutral",
    },
  ], [
    filteredIncidentRecords.length,
    filteredProfileAllocations.length,
    incidentHandlingSummary.unacknowledged,
    incidentOnly,
    jobActivitySummary.active,
    jobActivitySummary.retained,
    sessionActivitySummary.activeSessions,
    sessionActivitySummary.activeTabs,
    sessionActivitySummary.retainedSessions,
    sessionActivitySummary.retainedTabs,
    visibleEvents.length,
  ]);

  if (!canFetch) {
    return (
      <div className="service-panel-empty">
        <div className="service-panel-empty-card">
          <ShieldCheck className="size-7 text-muted-foreground" />
          <p className="text-sm font-black text-foreground">No active service session</p>
          <p className="text-xs leading-5 text-muted-foreground">
            Start or select a browser session to inspect service health, events, and reconciliation state.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <EventDetailDialog
        event={selectedEvent}
        onOpenChange={(open) => {
          if (!open) setSelectedEvent(null);
        }}
      />
      <IncidentDetailDialog
        incident={selectedIncident}
        activity={selectedIncidentActivity}
        activityLoading={selectedIncidentActivityLoading}
        activityError={selectedIncidentActivityError}
        onOpenChange={(open) => {
          if (!open) setSelectedIncident(null);
        }}
        onAcknowledge={(incident, note) => handleIncident(incident, "acknowledge", note)}
        onResolve={(incident, note) => handleIncident(incident, "resolve", note)}
        acting={!!selectedIncident && actingIncidentId === selectedIncident.id}
      />
      <BrowserDetailDialog
        browser={selectedBrowser}
        onInspectViewStream={(stream, browser) => openViewStream(stream, browser)}
        onOpenChange={(open) => {
          if (!open) setSelectedBrowser(null);
        }}
      />
      <ViewStreamInspectDialog
        selection={selectedViewStream}
        fullscreen={viewStreamFullscreen}
        onFullscreenChange={setViewStreamFullscreen}
        onOpenChange={(open) => {
          if (!open) setSelectedViewStream(null);
        }}
      />
      <SessionDetailDialog
        session={selectedSession}
        onOpenChange={(open) => {
          if (!open) setSelectedSession(null);
        }}
      />
      <TabDetailDialog
        tab={selectedTab}
        viewStreamAvailable={Boolean(selectedTab?.browserId && browserPrimaryViewStream(browserById.get(selectedTab.browserId)))}
        onInspect={inspectTabViewStream}
        onOpenChange={(open) => {
          if (!open) setSelectedTab(null);
        }}
      />
      <JobDetailDialog
        job={selectedJob}
        onOpenChange={(open) => {
          if (!open) setSelectedJob(null);
        }}
        onCancel={cancelJob}
      />
      <ProfileAllocationDetailDialog
        allocation={selectedProfileAllocation}
        loading={selectedProfileAllocationLoading}
        error={selectedProfileAllocationError}
        onOpenChange={(open) => {
          if (!open) {
            profileAllocationLookupId.current += 1;
            setSelectedProfileAllocation(null);
            setSelectedProfileAllocationError("");
            setSelectedProfileAllocationLoading(false);
          }
        }}
      />
      <div className="service-panel-hero">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <span className="service-panel-mark">
              <ShieldCheck className="size-4" />
            </span>
            <p className="truncate text-sm font-black tracking-[-0.03em] text-foreground">
              Service control plane
            </p>
          </div>
          <p className="mt-1 text-xs leading-5 text-muted-foreground">
            Service-scoped telemetry from the dashboard API
            {activePort > 0 ? `; active browser port ${activePort}` : ""}
          </p>
        </div>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              size="icon"
              variant="ghost"
              className="service-panel-action-button"
              aria-label="Service actions"
              title="Service actions"
            >
              {reconciling ? <Loader2 className="size-4 animate-spin" /> : <MoreHorizontal className="size-4" />}
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-56">
            <DropdownMenuLabel>Service actions</DropdownMenuLabel>
            <div className="service-operator-menu" onKeyDown={(event) => event.stopPropagation()}>
              <label htmlFor="service-audit-actor">Audit actor</label>
              <input
                id="service-audit-actor"
                aria-label="Audit actor for incident actions"
                className="service-operator-input"
                placeholder={activeSession || "dashboard"}
                value={operatorIdentity}
                onChange={(event) => setOperatorIdentity(event.target.value)}
              />
              <p>Incident and cleanup actions are recorded as this actor.</p>
            </div>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={reconcile} disabled={reconciling || loading}>
              <RefreshCw className="size-3.5" />
              Reconcile retained state
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      {error && (
        <div className="mx-3 mt-3 flex items-start gap-2 rounded-2xl border border-destructive/25 bg-destructive/10 px-3 py-2 text-xs text-destructive">
          <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
          <span>{error}</span>
        </div>
      )}

      <div className="service-panel-workbench">
        <div className="service-panel-body">
          <div className="service-status-strip">
            <ServiceStatusLight
              label="Worker"
              value={control?.worker_state ?? serviceState?.controlPlane?.workerState ?? "unknown"}
              detail={`Queue ${control?.queue_depth ?? serviceState?.controlPlane?.queueDepth ?? 0} of ${control?.queue_capacity ?? serviceState?.controlPlane?.queueCapacity ?? 0}; job timeout ${serviceJobTimeoutMs ? `${serviceJobTimeoutMs} ms` : "off"}`}
              icon={ServerCog}
              tone={healthTone(control?.worker_state ?? serviceState?.controlPlane?.workerState)}
            />
            <ServiceStatusLight
              label="Browser"
              value={control?.browser_health ?? serviceState?.controlPlane?.browserHealth ?? "unknown"}
              detail={`${entityCounts.browsers} tracked browser records`}
              icon={RadioTower}
              tone={healthTone(control?.browser_health ?? serviceState?.controlPlane?.browserHealth)}
            />
            <ServiceStatusLight
              label="Reconciled"
              value={formatRelativeTime(reconciliation?.lastReconciledAt)}
              detail={`${reconciliation?.changedBrowsers ?? 0} changed of ${reconciliation?.browserCount ?? 0} browsers`}
              icon={Clock3}
              tone={reconciliation?.lastError ? "bad" : "good"}
            />
            <ServiceStatusLight
              label="Events"
              value={String(events?.total ?? serviceState?.events?.length ?? 0)}
              detail={`${events?.count ?? recentEvents.length} shown in this view`}
              icon={History}
              tone="neutral"
            />
            <ServiceStatusLight
              label="Jobs"
              value={String(jobs?.total ?? entityCounts.jobs)}
              detail={`${jobs?.count ?? recentJobs.length} recent control jobs`}
              icon={ServerCog}
              tone="neutral"
            />
            <ServiceStatusLight
              label="Records"
              value={`${entityCounts.browsers} browsers`}
              detail={`Retained service-state counts: ${managedRecordDetail}`}
              icon={GitBranch}
              tone={retainedStateCleanupNeeded ? "warn" : "neutral"}
            />
          </div>

          {(reconciliation?.lastError || retainedStateCleanupNeeded) && (
            <div className="service-state-alerts" aria-label="Managed state attention items">
              {reconciliation?.lastError && (
                <div className="service-state-alert service-state-alert-error">
                  <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
                  <span>{reconciliation.lastError}</span>
                </div>
              )}
              {retainedStateCleanupNeeded && (
                <div className="service-state-alert service-retained-state-hint">
                  <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
                  <div className="min-w-0 flex-1">
                    <p className="font-black text-foreground">Retained state is large.</p>
                    <p className="mt-1 leading-5">
                      Review the Records status detail, then run a dry-run prune or repair. Apply is enabled only after a reviewed result.
                    </p>
                    <div className="service-retained-state-actions">
                      <Button
                        size="sm"
                        variant="outline"
                        className="h-7 rounded-lg px-2 text-[11px]"
                        disabled={!!cleanupLoading || !!cleanupApplying}
                        onClick={() => runRetainedCleanup("prune", false)}
                      >
                        {cleanupLoading === "prune" ? <Loader2 className="size-3 animate-spin" /> : null}
                        Dry-run prune
                      </Button>
                      <Button
                        size="sm"
                        variant="outline"
                        className="h-7 rounded-lg px-2 text-[11px]"
                        disabled={!!cleanupLoading || !!cleanupApplying}
                        onClick={() => runRetainedCleanup("repair", false)}
                      >
                        {cleanupLoading === "repair" ? <Loader2 className="size-3 animate-spin" /> : null}
                        Dry-run repair
                      </Button>
                      <AlertDialog>
                        <AlertDialogTrigger asChild>
                          <Button
                            size="sm"
                            className="h-7 rounded-lg px-2 text-[11px]"
                            disabled={!cleanupApplyEnabled || !!cleanupLoading || !!cleanupApplying || !cleanupKind}
                          >
                            {cleanupApplying ? <Loader2 className="size-3 animate-spin" /> : null}
                            {cleanupApplyLabel}
                          </Button>
                        </AlertDialogTrigger>
                        <AlertDialogContent>
                          <AlertDialogHeader>
                            <AlertDialogTitle>{cleanupDialogTitle}</AlertDialogTitle>
                            <AlertDialogDescription>{cleanupDialogDescription}</AlertDialogDescription>
                          </AlertDialogHeader>
                          <div className="service-retained-cleanup-confirm">
                            <p>
                              Candidates: {cleanupCountSummary(cleanupResult?.candidateCounts)}
                            </p>
                            <p>
                              Skipped: {cleanupCountSummary(cleanupResult?.skippedCounts)}
                            </p>
                            <p>
                              Actor: {operatorIdentity.trim() || activeSession || "operator"}
                            </p>
                          </div>
                          <AlertDialogFooter>
                            <AlertDialogCancel disabled={!!cleanupApplying}>Cancel</AlertDialogCancel>
                            <AlertDialogAction
                              className="bg-destructive/10 text-destructive hover:bg-destructive/20 focus-visible:border-destructive/40 focus-visible:ring-destructive/20"
                              disabled={!cleanupKind || !!cleanupApplying}
                              onClick={() => cleanupKind && runRetainedCleanup(cleanupKind, true)}
                            >
                              {cleanupApplying ? <Loader2 className="size-3 animate-spin" /> : null}
                              Apply cleanup
                            </AlertDialogAction>
                          </AlertDialogFooter>
                        </AlertDialogContent>
                      </AlertDialog>
                    </div>
                  {cleanupError && <p className="service-retained-cleanup-error">{cleanupError}</p>}
                  {cleanupResult && (
                    <div className="service-retained-cleanup-result">
                      <div className="service-retained-cleanup-result-header">
                        <Badge variant="secondary">{cleanupKind ?? "cleanup"}</Badge>
                        <span>{cleanupResult.dryRun ? "Dry-run" : "Applied"}</span>
                        <span>{cleanupCandidateTotal} candidates</span>
                        {!cleanupResult.dryRun && <span>{cleanupAppliedTotal} changed</span>}
                      </div>
                      <p>
                        Candidates: {cleanupCountSummary(cleanupResult.candidateCounts)}
                      </p>
                      <p>
                        Skipped: {cleanupCountSummary(cleanupResult.skippedCounts)}
                      </p>
                      {!cleanupResult.dryRun && (
                        <p>
                          Changed: {cleanupCountSummary(cleanupKind === "repair"
                            ? cleanupResult.repairedCounts
                            : cleanupResult.removed)}
                        </p>
                      )}
                      {cleanupResult.recommendedNextStep && (
                        <p className="service-retained-cleanup-next">
                          Next: {cleanupResult.recommendedNextStep}
                        </p>
                      )}
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>
          )}

          <div className="service-summary-card">
            <div className="flex items-center gap-2">
              <RadioTower className="size-4 text-muted-foreground" />
              <div className="min-w-0">
                <p className="text-xs font-black uppercase tracking-[0.18em] text-muted-foreground">
                  Managed browsers
                </p>
                <p className="truncate text-[11px] text-muted-foreground">
                  {browserRecords.length} persisted browser records
                </p>
              </div>
            </div>
            <div className="service-browser-table-region">
              {browserRecords.length === 0 ? (
                <p className="rounded-2xl bg-foreground/[0.04] px-3 py-6 text-center text-xs text-muted-foreground">
                  No browser records yet.
                </p>
              ) : (
                <BrowserTable browsers={browserRecords} onSelect={inspectBrowser} selectedBrowserId={selectedBrowserId} />
              )}
            </div>
          </div>

          <Tabs
            value={workspaceTab}
            onValueChange={(value) => setWorkspaceTab(value as ServiceWorkspaceTab)}
            className="service-workspace-card"
          >
            <div className="service-workspace-header">
              <div className="min-w-0">
                <p className="service-workspace-title">Operational records</p>
                <p className="service-workspace-detail">
                  Inspect profile routing, incidents, leases, queue history, and trace without leaving the browser table.
                </p>
              </div>
              <TabsList className="service-workspace-tabs" variant="line">
                {workspaceTabs.map((tab) => (
                  <TabsTrigger
                    key={tab.value}
                    value={tab.value}
                    className={cn(
                      "service-workspace-tab",
                      tab.tone === "bad" && "service-workspace-tab-bad",
                      tab.tone === "warn" && "service-workspace-tab-warn",
                    )}
                  >
                    <span>{tab.label}</span>
                    <span className="service-workspace-tab-count">{tab.count}</span>
                    <span className="service-workspace-tab-detail">{tab.detail}</span>
                  </TabsTrigger>
                ))}
              </TabsList>
            </div>

            <TabsContent value="profiles" className="service-workspace-content">
              <div className="service-workspace-pane-heading">
                <ShieldCheck className="size-3.5 text-muted-foreground" />
                <span>{filteredProfileAllocations.length} of {profileAllocations.length} profile allocation rows</span>
              </div>
              <div className="service-record-controls">
                <label className="service-browser-filter service-record-filter">
                  <Filter className="size-3.5" />
                  <span className="sr-only">Filter profile allocation rows</span>
                  <input
                    value={profileAllocationQuery}
                    onChange={(event) => setProfileAllocationQuery(event.target.value)}
                    placeholder="Filter profiles, services, holders, tasks"
                  />
                </label>
                <div className="service-record-limit-groups">
                  <div className="service-filter-group" aria-label="Profile allocation display limit">
                    <span className="service-record-limit-label">Profiles</span>
                    {SERVICE_RECORD_LIMIT_OPTIONS.map((limit) => (
                      <button
                        key={`profile-allocation-limit-${limit}`}
                        type="button"
                        className={cn("service-filter-chip", profileAllocationLimit === limit && "service-filter-chip-active")}
                        onClick={() => setProfileAllocationLimit(limit)}
                      >
                        {limit}
                      </button>
                    ))}
                  </div>
                </div>
              </div>
              <div className="service-section-list">
                <p className="service-record-list-heading">
                  Profile rows: {visibleProfileAllocations.length} shown
                  {hiddenProfileAllocationCount > 0 ? ` / ${hiddenProfileAllocationCount} hidden` : ""}
                </p>
                {filteredProfileAllocations.length === 0 ? (
                  <p className="rounded-2xl bg-foreground/[0.04] px-3 py-5 text-center text-xs text-muted-foreground">
                    {profileAllocations.length === 0 ? "No profile allocation rows yet." : "No profile allocation rows match the current filter."}
                  </p>
                ) : (
                  visibleProfileAllocations.map((allocation, index) => (
                    <ProfileAllocationRow
                      key={allocation.profileId || `profile-allocation-${index}`}
                      allocation={allocation}
                      onSelect={inspectProfileAllocation}
                    />
                  ))
                )}
              </div>
            </TabsContent>

            <TabsContent value="incidents" className="service-workspace-content">
              <div className="service-workspace-pane-heading">
                <AlertTriangle className="size-3.5 text-muted-foreground" />
                <span>{incidentHandlingSummary.unacknowledged} unacknowledged / {incidentHandlingSummary.acknowledged} acknowledged / {incidentHandlingSummary.resolved} resolved</span>
              </div>
              <div className="service-incident-summary">
                <div className="flex items-center justify-between gap-2">
                  <p className="text-[10px] font-black uppercase tracking-[0.16em] text-muted-foreground">
                    Remedy groups
                  </p>
                  <span className="text-[10px] font-bold text-muted-foreground">
                    {incidents?.summary?.groupCount ?? incidentSummaryGroups.length} groups / {incidents?.matched ?? 0} matched
                  </span>
                </div>
                <div className="mt-2 space-y-1.5">
                  {incidentSummaryGroups.length === 0 ? (
                    <p className="rounded-2xl bg-foreground/[0.04] px-3 py-4 text-center text-xs text-muted-foreground">
                      No incident remedy groups yet.
                    </p>
                  ) : (
                    incidentSummaryGroups.slice(0, 4).map((group) => (
                      <IncidentSummaryGroupRow
                        key={group.key}
                        group={group}
                      />
                    ))
                  )}
                </div>
              </div>
              <div className="service-filter-bar" aria-label="Incident handling filters">
                <div className="service-filter-group">
                  <Filter className="size-3.5 text-muted-foreground" />
                  {INCIDENT_HANDLING_OPTIONS.map((option) => (
                    <button
                      key={option.value}
                      type="button"
                      className={cn(
                        "service-filter-chip",
                        incidentHandlingFilter === option.value && "service-filter-chip-active",
                        option.value === "unacknowledged" && incidentHandlingFilter === option.value && "service-filter-chip-incident",
                      )}
                      onClick={() => setIncidentHandlingFilter(option.value)}
                    >
                      {option.label}
                    </button>
                  ))}
                </div>
                <label className="service-browser-filter service-record-filter">
                  <span className="sr-only">Filter incidents</span>
                  <input
                    value={incidentQuery}
                    onChange={(event) => setIncidentQuery(event.target.value)}
                    placeholder="filter incidents, browsers, remedies"
                  />
                </label>
                <div className="service-filter-group" aria-label="Incident display limit">
                  <span className="service-record-limit-label">Show</span>
                  {SERVICE_RECORD_LIMIT_OPTIONS.map((limit) => (
                    <button
                      key={`incident-limit-${limit}`}
                      type="button"
                      className={cn("service-filter-chip", incidentLimit === limit && "service-filter-chip-active")}
                      onClick={() => setIncidentLimit(limit)}
                    >
                      {limit}
                    </button>
                  ))}
                </div>
              </div>
              <div className="service-section-list">
                <p className="service-record-list-heading">
                  Incidents: {visibleIncidentRecords.length} shown
                  {hiddenIncidentCount > 0 ? ` / ${hiddenIncidentCount} hidden` : ""}
                </p>
                {filteredIncidentRecords.length === 0 ? (
                  <p className="rounded-2xl bg-foreground/[0.04] px-3 py-5 text-center text-xs text-muted-foreground">
                    {incidentRecords.length === 0
                      ? "No grouped incidents for this session yet."
                      : "No grouped incidents matched the current handling filter."}
                  </p>
                ) : (
                  visibleIncidentRecords.map((incident) => (
                    <IncidentRow
                      key={incident.id}
                      incident={incident}
                      onSelect={inspectIncident}
                    />
                  ))
                )}
              </div>
            </TabsContent>

            <TabsContent value="sessions" className="service-workspace-content">
              <div className="service-workspace-pane-heading">
                <GitBranch className="size-3.5 text-muted-foreground" />
                <span>
                  {sessionActivitySummary.activeSessions} active sessions / {sessionActivitySummary.retainedSessions} retained;
                  {" "}
                  {sessionActivitySummary.activeTabs} active tabs / {sessionActivitySummary.retainedTabs} retained
                </span>
              </div>
              <div className="service-record-controls">
                <label className="service-browser-filter service-record-filter">
                  <Filter className="size-3.5" />
                  <span className="sr-only">Filter sessions and tabs</span>
                  <input
                    value={sessionTabQuery}
                    onChange={(event) => setSessionTabQuery(event.target.value)}
                    placeholder="Filter sessions, tabs, profiles, URLs"
                  />
                </label>
                <div className="service-record-limit-groups">
                  <div className="service-filter-group" aria-label="Session display limit">
                    <span className="service-record-limit-label">Sessions</span>
                    {SERVICE_RECORD_LIMIT_OPTIONS.map((limit) => (
                      <button
                        key={`session-limit-${limit}`}
                        type="button"
                        className={cn("service-filter-chip", sessionLimit === limit && "service-filter-chip-active")}
                        onClick={() => setSessionLimit(limit)}
                      >
                        {limit}
                      </button>
                    ))}
                  </div>
                  <div className="service-filter-group" aria-label="Tab display limit">
                    <span className="service-record-limit-label">Tabs</span>
                    {SERVICE_RECORD_LIMIT_OPTIONS.map((limit) => (
                      <button
                        key={`tab-limit-${limit}`}
                        type="button"
                        className={cn("service-filter-chip", tabLimit === limit && "service-filter-chip-active")}
                        onClick={() => setTabLimit(limit)}
                      >
                        {limit}
                      </button>
                    ))}
                  </div>
                </div>
              </div>
              <div className="service-split-records">
                <div className="service-section-list">
                  <p className="service-record-list-heading">
                    Sessions: {sessionActivitySummary.activeSessions} active / {sessionActivitySummary.retainedSessions} retained; {visibleSessionRecords.length} shown
                    {hiddenSessionCount > 0 ? ` / ${hiddenSessionCount} hidden` : ""}
                  </p>
                  {filteredSessionRecords.length === 0 ? (
                    <p className="rounded-2xl bg-foreground/[0.04] px-3 py-5 text-center text-xs text-muted-foreground">
                      {sessionRecords.length === 0 ? "No service sessions yet." : "No sessions match the current filter."}
                    </p>
                  ) : (
                    visibleSessionRecords.map((session, index) => (
                      <ServiceSessionRow
                        key={session.id || `session-${index}`}
                        session={session}
                        onSelect={inspectSession}
                      />
                    ))
                  )}
                </div>
                <div className="service-section-list">
                  <p className="service-record-list-heading">
                    Tabs: {sessionActivitySummary.activeTabs} active / {sessionActivitySummary.retainedTabs} retained; {visibleTabRecords.length} shown
                    {hiddenTabCount > 0 ? ` / ${hiddenTabCount} hidden` : ""}
                  </p>
                  {filteredTabRecords.length === 0 ? (
                    <p className="rounded-2xl bg-foreground/[0.04] px-3 py-5 text-center text-xs text-muted-foreground">
                      {tabRecords.length === 0 ? "No service tabs yet." : "No tabs match the current filter."}
                    </p>
                  ) : (
                    visibleTabRecords.map((tab, index) => (
                      <ServiceTabRow
                        key={tab.id || tab.targetId || `tab-${index}`}
                        tab={tab}
                        viewStreamAvailable={Boolean(tab.browserId && browserPrimaryViewStream(browserById.get(tab.browserId)))}
                        onInspect={inspectTabViewStream}
                        onSelect={inspectTab}
                      />
                    ))
                  )}
                </div>
              </div>
            </TabsContent>

            <TabsContent value="jobs" className="service-workspace-content">
              <div className="service-workspace-pane-heading">
                <ServerCog className="size-3.5 text-muted-foreground" />
                <span>{jobActivitySummary.active} active / {jobActivitySummary.retained} retained; {jobSummary}</span>
                {loading && <Loader2 className="size-3.5 animate-spin text-muted-foreground" />}
              </div>
              <div className="service-workspace-summary-chips" aria-label="Service job activity summary">
                <span>{jobActivitySummary.active} queued or running</span>
                <span>{jobActivitySummary.terminal} terminal</span>
                <span>{recentJobs.length} recent shown</span>
              </div>
              <div className="service-section-list">
                {recentJobs.length === 0 ? (
                  <p className="rounded-2xl bg-foreground/[0.04] px-3 py-6 text-center text-xs text-muted-foreground">
                    No service jobs yet.
                  </p>
                ) : (
                  recentJobs.map((job) => <JobRow key={job.id} job={job} onSelect={inspectJob} />)
                )}
              </div>
            </TabsContent>

            <TabsContent value="events" className="service-workspace-content">
              <div className="service-workspace-pane-grid">
                <HealthTransitionTimeline
                  events={healthTransitionEvents}
                  onSelect={setSelectedEvent}
                />
                <TraceExplorer
                  filters={traceFilters}
                  trace={trace}
                  loading={traceLoading}
                  error={traceError}
                  timeline={traceTimeline}
                  onFiltersChange={setTraceFilters}
                  onLoad={loadTrace}
                  onClear={clearTrace}
                />
              </div>
              <div className="service-workspace-pane-heading">
                <CheckCircle2 className="size-3.5 text-muted-foreground" />
                <span>{eventSummary}</span>
                {loading && <Loader2 className="size-3.5 animate-spin text-muted-foreground" />}
              </div>
              <div className="service-filter-bar" aria-label="Event filters">
              <div className="service-filter-group">
                <Filter className="size-3.5 text-muted-foreground" />
                <button
                  type="button"
                  className={cn("service-filter-chip", incidentOnly && "service-filter-chip-active service-filter-chip-incident")}
                  onClick={() => setIncidentOnly((value) => !value)}
                >
                  Incidents
                </button>
                {EVENT_KIND_OPTIONS.map((option) => (
                  <button
                    key={option.value}
                    type="button"
                    className={cn("service-filter-chip", eventKind === option.value && "service-filter-chip-active")}
                    onClick={() => setEventKind(option.value)}
                  >
                    {option.label}
                  </button>
                ))}
              </div>
              <div className="service-filter-group">
                {EVENT_WINDOW_OPTIONS.map((option) => (
                  <button
                    key={option.value}
                    type="button"
                    className={cn("service-filter-chip", eventWindow === option.value && "service-filter-chip-active")}
                    onClick={() => setEventWindow(option.value)}
                  >
                    {option.label}
                  </button>
                ))}
              </div>
              <div className="service-filter-group">
                {EVENT_LIMIT_OPTIONS.map((limit) => (
                  <button
                    key={limit}
                    type="button"
                    className={cn("service-filter-chip", eventLimit === limit && "service-filter-chip-active")}
                    onClick={() => setEventLimit(limit)}
                  >
                    {limit}
                  </button>
                ))}
              </div>
              <input
                aria-label="Filter events by browser ID"
                className="service-filter-input"
                placeholder="browser id"
                value={eventBrowserId}
                onChange={(event) => setEventBrowserId(event.target.value)}
              />
              {activeFilterCount > 0 && (
                <button
                  type="button"
                  className="service-filter-reset"
                  onClick={() => {
                    setEventKind("all");
                    setEventWindow("all");
                    setEventLimit(8);
                    setEventBrowserId("");
                    setIncidentOnly(false);
                  }}
                >
                  Reset {activeFilterCount}
                </button>
              )}
            </div>
            <div className="service-section-list">
              {visibleEvents.length === 0 ? (
                <p className="rounded-2xl bg-foreground/[0.04] px-3 py-6 text-center text-xs text-muted-foreground">
                  {incidentOnly ? "No incident events matched the current filters." : "No service events yet."}
                </p>
              ) : (
                visibleEvents.map((event) => (
                  <EventRow key={event.id} event={event} onSelect={setSelectedEvent} />
                ))
              )}
            </div>
            </TabsContent>
          </Tabs>
        </div>
      </div>
    </div>
  );
}
