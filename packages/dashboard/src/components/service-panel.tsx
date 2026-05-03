"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { useAtomValue } from "jotai/react";
import {
  AlertTriangle,
  CheckCircle2,
  Clock3,
  Filter,
  GitBranch,
  History,
  Loader2,
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
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Separator } from "@/components/ui/separator";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  normalizeServiceTraceData,
  traceProfileLeaseWaits,
  traceFilterSummary,
  traceSummaryCards,
  traceTimelineItems,
  type ServiceTraceData,
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

type ServiceBrowser = {
  id: string;
  profileId?: string | null;
  host?: string;
  health?: string;
  pid?: number | null;
  cdpEndpoint?: string | null;
  viewStreams?: unknown[];
  activeSessionIds?: string[];
  lastError?: string | null;
};

type ServiceSession = {
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

type ServiceProfileAllocation = {
  profileId: string;
  profileName?: string;
  allocation?: string;
  keyring?: string;
  targetServiceIds?: string[];
  authenticatedServiceIds?: string[];
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

type ServiceTab = {
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

type ServiceJob = {
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

type IncidentRecord = {
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
type IncidentHandlingFilter = "all" | "unacknowledged" | "acknowledged" | "resolved";
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

const INCIDENT_HANDLING_OPTIONS: Array<{ value: IncidentHandlingFilter; label: string }> = [
  { value: "all", label: "All" },
  { value: "unacknowledged", label: "Unacknowledged" },
  { value: "acknowledged", label: "Acknowledged" },
  { value: "resolved", label: "Resolved" },
];

const OPERATOR_STORAGE_KEY = "agent-browser-dashboard-operator";

function serviceBase(port: number): string {
  return `http://localhost:${port}/api/service`;
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

function HealthCard({
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
  tone?: "good" | "warn" | "bad" | "neutral";
}) {
  return (
    <div className={cn("service-health-card", `service-health-${tone}`)}>
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-[10px] font-bold uppercase tracking-[0.18em] text-muted-foreground">
            {label}
          </p>
          <p className="mt-1 truncate text-lg font-black tracking-[-0.04em] text-foreground">
            {value}
          </p>
        </div>
        <span className="service-health-icon">
          <Icon className="size-4" />
        </span>
      </div>
      <p className="mt-3 text-xs leading-5 text-muted-foreground">{detail}</p>
    </div>
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
          {group.incidentIdLabel}
        </p>
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

function BrowserRow({ browser, onSelect }: { browser: ServiceBrowser; onSelect: (browser: ServiceBrowser) => void }) {
  const tone = healthTone(browser.health);
  return (
    <button
      type="button"
      className="service-browser-row"
      onClick={() => onSelect(browser)}
      aria-label={`Inspect browser ${browser.id}`}
    >
      <span className={cn("service-browser-health-dot", `service-browser-health-${tone}`)} />
      <div className="min-w-0 flex-1">
        <div className="flex min-w-0 items-center gap-2">
          <span className="truncate text-xs font-bold text-foreground">{browser.id || "unnamed browser"}</span>
          <Badge variant="outline" className="h-4 max-w-28 truncate px-1.5 text-[9px]">
            {browser.health ?? "unknown"}
          </Badge>
        </div>
        <p className="mt-1 truncate text-xs text-muted-foreground">
          {browser.host ?? "unknown host"}
          {browser.pid ? ` / pid ${browser.pid}` : ""}
        </p>
      </div>
      <span className="text-[10px] font-bold text-muted-foreground">
        {browser.activeSessionIds?.length ?? 0} sessions
      </span>
    </button>
  );
}

function BrowserDetailDialog({
  browser,
  onOpenChange,
}: {
  browser: ServiceBrowser | null;
  onOpenChange: (open: boolean) => void;
}) {
  const viewStreamCount = browser?.viewStreams?.length ?? 0;
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
                  <pre className="service-event-details-json">{formatDetails(browser.viewStreams)}</pre>
                </div>
              )}
            </div>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
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
  const request = formatDetails(job?.request);
  const response = formatDetails(job?.response ?? job?.result);
  const target = formatDetails(job?.target);
  const canCancel = job?.state === "queued" || job?.state === "running";
  const namingWarning = serviceJobNamingWarningLabel(job?.namingWarnings);
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
              {canCancel && (
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
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}

function ProfileAllocationRow({ allocation }: { allocation: ServiceProfileAllocation }) {
  const tone = profileAllocationTone(allocation.leaseState);
  const holderCount = allocation.holderCount ?? allocation.holderSessionIds?.length ?? 0;
  const waitingCount = allocation.waitingJobCount ?? allocation.waitingJobIds?.length ?? 0;
  return (
    <div className="service-browser-row service-profile-allocation-row">
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

function ServiceTabRow({ tab, onSelect }: { tab: ServiceTab; onSelect: (tab: ServiceTab) => void }) {
  const tone = tab.lifecycle === "crashed" ? "bad" : tab.lifecycle === "ready" ? "good" : "neutral";
  return (
    <button
      type="button"
      className="service-browser-row"
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
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}

function TabDetailDialog({
  tab,
  onOpenChange,
}: {
  tab: ServiceTab | null;
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
            </div>
          </>
        )}
      </DialogContent>
    </Dialog>
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

export function ServicePanel() {
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
  const [selectedSession, setSelectedSession] = useState<ServiceSession | null>(null);
  const [selectedTab, setSelectedTab] = useState<ServiceTab | null>(null);
  const [selectedJob, setSelectedJob] = useState<ServiceJob | null>(null);

  const canFetch = activePort > 0 && !!activeSession;
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
      setSelectedJob(job);
      return;
    }
    setError("");
    try {
      const resp = await fetch(`${serviceBase(activePort)}/jobs/${encodeURIComponent(job.id)}`);
      const json = (await resp.json()) as ApiResponse<ServiceJobsData>;
      if (!json.success) throw new Error(json.error || "Service job lookup failed");
      setSelectedJob(json.data?.job ?? job);
    } catch (err) {
      setSelectedJob(job);
      setError(err instanceof Error ? err.message : "Service job lookup unavailable");
    }
  }, [activePort, canFetch]);

  const cancelJob = useCallback(async (job: ServiceJob) => {
    if (!canFetch || !job.id) return;
    setError("");
    try {
      const resp = await fetch(`${serviceBase(activePort)}/jobs/${encodeURIComponent(job.id)}/cancel`, {
        method: "POST",
      });
      const json = (await resp.json()) as ApiResponse<ServiceJobsData & { cancelled?: boolean }>;
      if (!json.success) throw new Error(json.error || "Service job cancel failed");
      setSelectedJob(json.data?.job ?? { ...job, error: "Cancellation requested" });
      await fetchService(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Service job cancel unavailable");
    }
  }, [activePort, canFetch, fetchService]);

  const handleIncident = useCallback(async (
    incident: IncidentRecord,
    action: "acknowledge" | "resolve",
    note: string,
  ) => {
    if (!canFetch || !incident.id) return;
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
      setSelectedIncident(null);
      await fetchService(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : `Service incident ${action} unavailable`);
    } finally {
      setActingIncidentId(null);
    }
  }, [activePort, activeSession, canFetch, fetchService, operatorIdentity]);

  const serviceState = status?.service_state;
  const control = status?.control_plane;
  const serviceJobTimeoutMs =
    control?.service_job_timeout_ms ?? serviceState?.controlPlane?.serviceJobTimeoutMs ?? null;
  const reconciliation = serviceState?.reconciliation;
  const recentJobs = jobs?.jobs ?? Object.values(serviceState?.jobs ?? {}).slice(-8);
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
  const retainedServiceJobs = useMemo(
    () => Object.values(serviceState?.jobs ?? {}),
    [serviceState?.jobs],
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
  const visibleIncidentRecords = useMemo(
    () =>
      incidentHandlingFilter === "all"
        ? incidentRecords
        : incidentRecords.filter((incident) => incidentHandlingState(incident) === incidentHandlingFilter),
    [incidentHandlingFilter, incidentRecords],
  );
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
  const tabRecords = useMemo(
    () => Object.values(serviceState?.tabs ?? {}),
    [serviceState?.tabs],
  );
  const entityCounts = useMemo(() => ({
    browsers: countEntries(serviceState?.browsers),
    profiles: countEntries(serviceState?.profiles),
    jobs: countEntries(serviceState?.jobs),
    sessions: countEntries(serviceState?.sessions),
    tabs: countEntries(serviceState?.tabs),
    policies: countEntries(serviceState?.sitePolicies),
    providers: countEntries(serviceState?.providers),
  }), [serviceState]);

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
        onOpenChange={(open) => {
          if (!open) setSelectedBrowser(null);
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
            Session-scoped service telemetry from port {activePort}
          </p>
        </div>
        <Button
          size="sm"
          variant="outline"
          onClick={reconcile}
          disabled={reconciling || loading}
          className="rounded-full"
        >
          {reconciling ? <Loader2 className="size-3.5 animate-spin" /> : <RefreshCw className="size-3.5" />}
          Reconcile
        </Button>
      </div>

      <div className="service-operator-card">
        <div className="min-w-0">
          <p className="text-[10px] font-black uppercase tracking-[0.18em] text-muted-foreground">
            Operator identity
          </p>
          <p className="mt-1 text-xs leading-5 text-muted-foreground">
            Incident actions are recorded as this actor.
          </p>
        </div>
        <input
          aria-label="Operator identity for incident actions"
          className="service-operator-input"
          placeholder={activeSession || "dashboard"}
          value={operatorIdentity}
          onChange={(event) => setOperatorIdentity(event.target.value)}
        />
      </div>

      {error && (
        <div className="mx-3 mt-3 flex items-start gap-2 rounded-2xl border border-destructive/25 bg-destructive/10 px-3 py-2 text-xs text-destructive">
          <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
          <span>{error}</span>
        </div>
      )}

      <ScrollArea className="min-h-0 flex-1">
        <div className="space-y-3 p-3">
          <div className="grid gap-2 sm:grid-cols-2">
            <HealthCard
              label="Worker"
              value={control?.worker_state ?? serviceState?.controlPlane?.workerState ?? "unknown"}
              detail={`Queue ${control?.queue_depth ?? serviceState?.controlPlane?.queueDepth ?? 0} of ${control?.queue_capacity ?? serviceState?.controlPlane?.queueCapacity ?? 0}; job timeout ${serviceJobTimeoutMs ? `${serviceJobTimeoutMs} ms` : "off"}`}
              icon={ServerCog}
              tone={healthTone(control?.worker_state ?? serviceState?.controlPlane?.workerState)}
            />
            <HealthCard
              label="Browser"
              value={control?.browser_health ?? serviceState?.controlPlane?.browserHealth ?? "unknown"}
              detail={`${entityCounts.browsers} tracked browser records`}
              icon={RadioTower}
              tone={healthTone(control?.browser_health ?? serviceState?.controlPlane?.browserHealth)}
            />
            <HealthCard
              label="Reconciled"
              value={formatRelativeTime(reconciliation?.lastReconciledAt)}
              detail={`${reconciliation?.changedBrowsers ?? 0} changed of ${reconciliation?.browserCount ?? 0} browsers`}
              icon={Clock3}
              tone={reconciliation?.lastError ? "bad" : "good"}
            />
            <HealthCard
              label="Events"
              value={String(events?.total ?? serviceState?.events?.length ?? 0)}
              detail={`${events?.count ?? recentEvents.length} shown in this view`}
              icon={History}
              tone="neutral"
            />
            <HealthCard
              label="Jobs"
              value={String(jobs?.total ?? entityCounts.jobs)}
              detail={`${jobs?.count ?? recentJobs.length} recent control jobs`}
              icon={ServerCog}
              tone="neutral"
            />
          </div>

          <div className="service-summary-card">
            <div className="flex items-center gap-2">
              <GitBranch className="size-4 text-muted-foreground" />
              <p className="text-xs font-black uppercase tracking-[0.18em] text-muted-foreground">
                Managed entities
              </p>
            </div>
            <div className="mt-3 grid grid-cols-3 gap-2">
              {Object.entries(entityCounts).map(([label, value]) => (
                <div key={label} className="rounded-2xl bg-foreground/[0.04] px-3 py-2">
                  <p className="text-lg font-black tracking-[-0.04em] text-foreground">{value}</p>
                  <p className="text-[10px] capitalize text-muted-foreground">{label}</p>
                </div>
              ))}
            </div>
            {reconciliation?.lastError && (
              <div className="mt-3 flex items-start gap-2 rounded-2xl bg-destructive/10 px-3 py-2 text-xs text-destructive">
                <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
                <span>{reconciliation.lastError}</span>
              </div>
            )}
          </div>

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
            <div className="mt-3 space-y-1">
              {browserRecords.length === 0 ? (
                <p className="rounded-2xl bg-foreground/[0.04] px-3 py-6 text-center text-xs text-muted-foreground">
                  No browser records yet.
                </p>
              ) : (
                browserRecords.map((browser, index) => (
                  <BrowserRow
                    key={browser.id || browser.cdpEndpoint || `browser-${index}`}
                    browser={browser}
                    onSelect={setSelectedBrowser}
                  />
                ))
              )}
            </div>
          </div>

          <div className="service-summary-card">
            <div className="flex items-center gap-2">
              <ShieldCheck className="size-4 text-muted-foreground" />
              <div className="min-w-0">
                <p className="text-xs font-black uppercase tracking-[0.18em] text-muted-foreground">
                  Profile allocation
                </p>
                <p className="truncate text-[11px] text-muted-foreground">
                  {profileAllocations.length} backend-owned allocation rows
                </p>
              </div>
            </div>
            <div className="mt-3 space-y-1">
              {profileAllocations.length === 0 ? (
                <p className="rounded-2xl bg-foreground/[0.04] px-3 py-5 text-center text-xs text-muted-foreground">
                  No profile allocation rows yet.
                </p>
              ) : (
                profileAllocations.map((allocation, index) => (
                  <ProfileAllocationRow
                    key={allocation.profileId || `profile-allocation-${index}`}
                    allocation={allocation}
                  />
                ))
              )}
            </div>
          </div>

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

          <div className="service-summary-card">
            <div className="flex items-center gap-2">
              <AlertTriangle className="size-4 text-destructive" />
              <div className="min-w-0">
                <p className="text-xs font-black uppercase tracking-[0.18em] text-muted-foreground">
                  Incident browsers
                </p>
                <p className="truncate text-[11px] text-muted-foreground">
                  {incidentHandlingSummary.unacknowledged} unacknowledged / {incidentHandlingSummary.acknowledged} acknowledged / {incidentHandlingSummary.resolved} resolved
                </p>
              </div>
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
            </div>
            <div className="mt-3 space-y-1">
              {visibleIncidentRecords.length === 0 ? (
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
                    onSelect={setSelectedIncident}
                  />
                ))
              )}
            </div>
          </div>

          <div className="service-summary-card">
            <div className="flex items-center gap-2">
              <GitBranch className="size-4 text-muted-foreground" />
              <div className="min-w-0">
                <p className="text-xs font-black uppercase tracking-[0.18em] text-muted-foreground">
                  Sessions and tabs
                </p>
                <p className="truncate text-[11px] text-muted-foreground">
                  {sessionRecords.length} sessions / {tabRecords.length} tabs
                </p>
              </div>
            </div>
            <div className="mt-3 grid gap-3 lg:grid-cols-2">
              <div className="space-y-1">
                <p className="px-1 text-[10px] font-black uppercase tracking-[0.16em] text-muted-foreground">
                  Sessions
                </p>
                {sessionRecords.length === 0 ? (
                  <p className="rounded-2xl bg-foreground/[0.04] px-3 py-5 text-center text-xs text-muted-foreground">
                    No service sessions yet.
                  </p>
                ) : (
                  sessionRecords.map((session, index) => (
                    <ServiceSessionRow
                      key={session.id || `session-${index}`}
                      session={session}
                      onSelect={setSelectedSession}
                    />
                  ))
                )}
              </div>
              <div className="space-y-1">
                <p className="px-1 text-[10px] font-black uppercase tracking-[0.16em] text-muted-foreground">
                  Tabs
                </p>
                {tabRecords.length === 0 ? (
                  <p className="rounded-2xl bg-foreground/[0.04] px-3 py-5 text-center text-xs text-muted-foreground">
                    No service tabs yet.
                  </p>
                ) : (
                  tabRecords.map((tab, index) => (
                    <ServiceTabRow
                      key={tab.id || tab.targetId || `tab-${index}`}
                      tab={tab}
                      onSelect={setSelectedTab}
                    />
                  ))
                )}
              </div>
            </div>
          </div>

          <div className="service-timeline-card">
            <div className="flex items-center gap-2 px-1">
              <ServerCog className="size-4 text-muted-foreground" />
              <div className="min-w-0">
                <p className="text-xs font-black uppercase tracking-[0.18em] text-muted-foreground">
                  Recent jobs
                </p>
                <p className="truncate text-[11px] text-muted-foreground">{jobSummary}</p>
              </div>
              {loading && <Loader2 className="ml-auto size-3.5 animate-spin text-muted-foreground" />}
            </div>
            <Separator className="my-3" />
            <div className="space-y-1">
              {recentJobs.length === 0 ? (
                <p className="rounded-2xl bg-foreground/[0.04] px-3 py-6 text-center text-xs text-muted-foreground">
                  No service jobs yet.
                </p>
              ) : (
                recentJobs.map((job) => <JobRow key={job.id} job={job} onSelect={inspectJob} />)
              )}
            </div>
          </div>

          <div className="service-timeline-card">
            <div className="flex items-center gap-2 px-1">
              <CheckCircle2 className="size-4 text-success" />
              <div className="min-w-0">
                <p className="text-xs font-black uppercase tracking-[0.18em] text-muted-foreground">
                  Recent events
                </p>
                <p className="truncate text-[11px] text-muted-foreground">{eventSummary}</p>
              </div>
              {loading && <Loader2 className="ml-auto size-3.5 animate-spin text-muted-foreground" />}
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
            <Separator className="my-3" />
            <div className="space-y-1">
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
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}
