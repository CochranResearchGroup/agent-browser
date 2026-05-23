"use client";

import type { KeyboardEvent as ReactKeyboardEvent, MouseEvent as ReactMouseEvent, ReactNode } from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  createServiceIncidentHandoff,
  createServiceTraceHandoff,
} from "@agent-browser/client/service-observability";
import { useAtomValue } from "jotai/react";
import {
  AlertTriangle,
  ChevronDown,
  CheckCircle2,
  Clock3,
  Copy,
  Edit3,
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
  Trash2,
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
  traceDisplayAllocationSummary,
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
  canControlViewStream,
  canEmbedViewStream,
  canOpenControlViewStream,
  canOpenViewStream,
  controlInputLabel,
  viewStreamCapabilityLabel,
  viewStreamControlTitle,
  viewStreamLabel,
  viewStreamOpenTitle,
  type ServiceViewStream,
} from "@/lib/service-view-streams";
import {
  browserRowCloseTitle,
  browserRowRepairTitle,
} from "@/lib/service-browser-row-actions";

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
  browserBuild?: string | null;
  displayIsolation?: string | null;
  displayName?: string | null;
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
  browserBuild?: string | null;
  targetServiceIds?: string[];
  authenticatedServiceIds?: string[];
  accountIds?: string[];
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
  browserSummaries?: ServiceProfileAllocationBrowserSummary[];
  tabIds?: string[];
};

type ServiceProfileRecord = {
  id?: string;
  name?: string;
  userDataDir?: string | null;
  sitePolicyIds?: string[];
  targetServiceIds?: string[];
  authenticatedServiceIds?: string[];
  accountIds?: string[];
  defaultBrowserHost?: string | null;
  browserBuild?: string | null;
  allocation?: string;
  keyring?: string;
  sharedServiceIds?: string[];
  credentialProviderIds?: string[];
  manualLoginPreferred?: boolean;
  targetReadiness?: ServiceProfileTargetReadiness[];
  persistent?: boolean;
  tags?: string[];
};

type RuntimeProfileConfigFormState = {
  name: string;
  userDataDir: string;
  defaultBrowserHost: string;
  browserBuild: string;
  allocation: string;
  keyring: string;
  targetServiceIds: string;
  authenticatedServiceIds: string;
  accountIds: string;
  sitePolicyIds: string;
  sharedServiceIds: string;
  credentialProviderIds: string;
  tags: string;
  manualLoginPreferred: boolean;
  persistent: boolean;
};

type ServiceProfileAllocationBrowserSummary = {
  browserId?: string;
  host?: string;
  health?: string;
  pid?: number | null;
  hasCdpEndpoint?: boolean;
  activeSessionIds?: string[];
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
  displayIsolation?: string | null;
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
  onControlBrowser?: (browser: ServiceBrowser) => void;
  onAcknowledgeIncident?: (incident: IncidentRecord, note: string) => void;
  onResolveIncident?: (incident: IncidentRecord, note: string) => void;
  onShowIncidentTrace?: (incident: IncidentRecord) => void;
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

type ServiceBrowserCloseData = {
  closed?: boolean;
  browserId?: string;
  requestedBrowserId?: string;
  serviceOwned?: boolean;
};

type ServiceBrowserRepairData = {
  repaired?: boolean;
  browser?: ServiceBrowser;
  incident?: IncidentRecord | null;
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
  profiles?: Record<string, ServiceProfileRecord>;
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

type ServiceContractsData = {
  contracts?: {
    serviceRequest?: {
      actions?: string[];
    };
  };
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
type ProfileReadinessFilter = "all" | "needs_attention" | "normal";
type IncidentHandlingFilter = "all" | "unacknowledged" | "acknowledged" | "resolved";
type ServiceJobDisplayFilter = "all" | "private_virtual_display" | "shared_display" | "ambient_display" | "unrecorded";
type ServiceJobSortKey = "submittedAt" | "state" | "action" | "displayIsolation" | "serviceName" | "taskName";
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

const SERVICE_JOB_DISPLAY_FILTER_OPTIONS: Array<{ value: ServiceJobDisplayFilter; label: string }> = [
  { value: "all", label: "Any display" },
  { value: "private_virtual_display", label: "Private display" },
  { value: "shared_display", label: "Shared display" },
  { value: "ambient_display", label: "Ambient display" },
  { value: "unrecorded", label: "Unrecorded" },
];

const SERVICE_JOB_STATE_FILTER_OPTIONS = [
  { value: "all", label: "All states" },
  { value: "queued", label: "Queued" },
  { value: "waiting_profile_lease", label: "Waiting lease" },
  { value: "running", label: "Running" },
  { value: "succeeded", label: "Succeeded" },
  { value: "failed", label: "Failed" },
  { value: "cancelled", label: "Cancelled" },
  { value: "timed_out", label: "Timed out" },
];

const SERVICE_JOB_SORT_LABELS: Record<ServiceJobSortKey, string> = {
  submittedAt: "Time",
  state: "State",
  action: "Action",
  displayIsolation: "Display",
  serviceName: "Service",
  taskName: "Task",
};

const INCIDENT_HANDLING_OPTIONS: Array<{ value: IncidentHandlingFilter; label: string }> = [
  { value: "all", label: "All" },
  { value: "unacknowledged", label: "Unacknowledged" },
  { value: "acknowledged", label: "Acknowledged" },
  { value: "resolved", label: "Resolved" },
];

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

function serviceJobSearchText(job: ServiceJob): string {
  return [
    job.id,
    job.action,
    job.state,
    job.priority,
    job.serviceName,
    job.agentName,
    job.taskName,
    job.displayIsolation,
    job.error,
    job.submittedAt,
    job.startedAt,
    job.completedAt,
    ...(job.namingWarnings ?? []),
  ].filter(Boolean).join(" ").toLowerCase();
}

function profileAllocationSearchText(allocation: ServiceProfileAllocation): string {
  return [
    allocation.profileId,
    allocation.profileName,
    allocation.allocation,
    allocation.keyring,
    allocation.browserBuild,
    allocation.leaseState,
    allocation.recommendedAction,
    ...(allocation.targetServiceIds ?? []),
    ...(allocation.authenticatedServiceIds ?? []),
    ...(allocation.accountIds ?? []),
    ...(allocation.sharedServiceIds ?? []),
    ...(allocation.holderSessionIds ?? []),
    ...(allocation.exclusiveHolderSessionIds ?? []),
    ...(allocation.waitingJobIds ?? []),
    ...(allocation.conflictSessionIds ?? []),
    ...(allocation.serviceNames ?? []),
    ...(allocation.agentNames ?? []),
    ...(allocation.taskNames ?? []),
    ...(allocation.browserIds ?? []),
    ...(allocation.browserSummaries ?? []).flatMap((browser) => [
      browser.browserId,
      browser.host,
      browser.health,
      browser.pid ? String(browser.pid) : "",
      ...(browser.activeSessionIds ?? []),
    ]),
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

function serviceProfileId(profile: ServiceProfileRecord, fallback = ""): string {
  return profile.id?.trim() || fallback;
}

function serviceProfileSearchText(profile: ServiceProfileRecord, allocation?: ServiceProfileAllocation): string {
  return [
    profile.id,
    profile.name,
    profile.userDataDir,
    profile.defaultBrowserHost,
    profile.browserBuild,
    profile.allocation,
    profile.keyring,
    profile.manualLoginPreferred ? "manual login preferred" : "",
    profile.persistent ? "persistent" : "",
    ...(profile.sitePolicyIds ?? []),
    ...(profile.targetServiceIds ?? []),
    ...(profile.authenticatedServiceIds ?? []),
    ...(profile.accountIds ?? []),
    ...(profile.sharedServiceIds ?? []),
    ...(profile.credentialProviderIds ?? []),
    ...(profile.tags ?? []),
    ...(profile.targetReadiness ?? []).flatMap((readiness) => [
      readiness.targetServiceId,
      readiness.loginId,
      readiness.state,
      readiness.evidence,
      readiness.recommendedAction,
    ]),
    allocation ? profileAllocationSearchText(allocation) : "",
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

function uniqueStringValues(values: Array<string | null | undefined>): string[] {
  return Array.from(new Set(values.map((value) => value?.trim()).filter(Boolean) as string[])).sort((left, right) =>
    left.localeCompare(right),
  );
}

function firstStringValue(values?: (string | null | undefined)[], fallback = "unknown"): string {
  return values?.find((value) => value?.trim())?.trim() ?? fallback;
}

function profileAllocationTargetValues(allocation: ServiceProfileAllocation): string[] {
  return uniqueStringValues([
    ...(allocation.targetReadiness ?? []).map((row) => row.targetServiceId),
    ...(allocation.targetServiceIds ?? []),
    ...(allocation.authenticatedServiceIds ?? []),
  ]);
}

function profileAllocationLoginValues(allocation: ServiceProfileAllocation): string[] {
  return uniqueStringValues([
    ...(allocation.targetReadiness ?? []).map((row) => row.loginId),
    ...(allocation.accountIds ?? []),
  ]);
}

function profileAllocationPrimaryTarget(allocation: ServiceProfileAllocation): string {
  return firstStringValue([
    allocation.targetReadiness?.find((row) => row.targetServiceId?.trim())?.targetServiceId,
    allocation.targetServiceIds?.find((value) => value.trim()),
    allocation.authenticatedServiceIds?.find((value) => value.trim()),
  ]);
}

function profileAllocationPrimaryLogin(allocation: ServiceProfileAllocation): string {
  return firstStringValue([
    allocation.targetReadiness?.find((row) => row.loginId?.trim())?.loginId,
    allocation.accountIds?.find((value) => value.trim()),
  ], "default identity");
}

function profileAllocationPrimaryBrowser(allocation: ServiceProfileAllocation): string {
  const summary = allocation.browserSummaries?.find((browser) => browser.browserId?.trim());
  if (summary?.browserId) {
    const host = summary.host ? ` on ${summary.host}` : "";
    const health = summary.health ? `, ${formatHealthLabel(summary.health)}` : "";
    return `${summary.browserId}${host}${health}`;
  }
  return firstStringValue(allocation.browserIds, "no browser assigned");
}

function profileReadinessNeedsAttention(rows?: ServiceProfileTargetReadiness[]): boolean {
  return Boolean(rows?.some((row) =>
    row.manualSeedingRequired ||
    ["needs_manual_seeding", "stale", "failed", "unverified", "seeding_closed_unverified"].includes((row.state ?? "").toLowerCase())
  ));
}

function serviceProfileReadiness(profile: ServiceProfileRecord, allocation?: ServiceProfileAllocation): ServiceProfileTargetReadiness[] {
  return profile.targetReadiness?.length ? profile.targetReadiness : allocation?.targetReadiness ?? [];
}

function serviceProfileTargets(profile: ServiceProfileRecord, allocation?: ServiceProfileAllocation): string[] {
  return uniqueStringValues([
    ...(profile.targetServiceIds ?? []),
    ...(profile.authenticatedServiceIds ?? []),
    ...serviceProfileReadiness(profile, allocation).map((row) => row.targetServiceId),
    ...(allocation?.targetServiceIds ?? []),
    ...(allocation?.authenticatedServiceIds ?? []),
  ]);
}

function serviceProfileAccounts(profile: ServiceProfileRecord, allocation?: ServiceProfileAllocation): string[] {
  return uniqueStringValues([
    ...(profile.accountIds ?? []),
    ...serviceProfileReadiness(profile, allocation).map((row) => row.loginId),
    ...(allocation?.accountIds ?? []),
  ]);
}

function serviceProfileBrowserBuild(profile: ServiceProfileRecord, allocation?: ServiceProfileAllocation): string | null {
  return profile.browserBuild ?? allocation?.browserBuild ?? null;
}

function serviceProfileKeyring(profile: ServiceProfileRecord, allocation?: ServiceProfileAllocation): string {
  return profile.keyring ?? allocation?.keyring ?? "default policy";
}

function commaList(values?: string[]): string {
  return values?.filter((value) => value.trim().length > 0).join(", ") ?? "";
}

function parseCommaList(value: string): string[] {
  return uniqueStringValues(value.split(",").map((item) => item.trim()).filter(Boolean));
}

function nullableFormValue(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function runtimeProfileConfigFormState(
  profile: ServiceProfileRecord,
  allocation?: ServiceProfileAllocation,
): RuntimeProfileConfigFormState {
  return {
    name: profile.name ?? allocation?.profileName ?? serviceProfileId(profile, allocation?.profileId ?? ""),
    userDataDir: profile.userDataDir ?? "",
    defaultBrowserHost: profile.defaultBrowserHost ?? "",
    browserBuild: serviceProfileBrowserBuild(profile, allocation) ?? "",
    allocation: profile.allocation ?? allocation?.allocation ?? "shared_service",
    keyring: profile.keyring ?? allocation?.keyring ?? "basic_password_store",
    targetServiceIds: commaList(serviceProfileTargets(profile, allocation)),
    authenticatedServiceIds: commaList(profile.authenticatedServiceIds ?? allocation?.authenticatedServiceIds),
    accountIds: commaList(serviceProfileAccounts(profile, allocation)),
    sitePolicyIds: commaList(profile.sitePolicyIds),
    sharedServiceIds: commaList(profile.sharedServiceIds ?? allocation?.sharedServiceIds),
    credentialProviderIds: commaList(profile.credentialProviderIds),
    tags: commaList(profile.tags),
    manualLoginPreferred: profile.manualLoginPreferred ?? false,
    persistent: profile.persistent ?? true,
  };
}

function runtimeProfileConfigPayload(
  profile: ServiceProfileRecord,
  form: RuntimeProfileConfigFormState,
): ServiceProfileRecord {
  const profileId = serviceProfileId(profile);
  return {
    id: profileId,
    name: form.name.trim() || profileId,
    userDataDir: nullableFormValue(form.userDataDir),
    sitePolicyIds: parseCommaList(form.sitePolicyIds),
    targetServiceIds: parseCommaList(form.targetServiceIds),
    authenticatedServiceIds: parseCommaList(form.authenticatedServiceIds),
    accountIds: parseCommaList(form.accountIds),
    defaultBrowserHost: nullableFormValue(form.defaultBrowserHost),
    browserBuild: nullableFormValue(form.browserBuild),
    allocation: form.allocation,
    keyring: form.keyring,
    sharedServiceIds: parseCommaList(form.sharedServiceIds),
    credentialProviderIds: parseCommaList(form.credentialProviderIds),
    manualLoginPreferred: form.manualLoginPreferred,
    targetReadiness: profile.targetReadiness ?? [],
    persistent: form.persistent,
    tags: parseCommaList(form.tags),
  };
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

function incidentTraceFilters(incident: IncidentRecord): TraceFilters {
  const records = [...incident.serviceEvents, ...incident.jobEvents]
    .sort((left, right) => new Date(left.timestamp).getTime() - new Date(right.timestamp).getTime());
  const contextRecord = records.find((record) =>
    record.serviceName || record.agentName || record.taskName || record.profileId || record.sessionId
  );
  return {
    serviceName: contextRecord?.serviceName ?? "",
    agentName: contextRecord?.agentName ?? "",
    taskName: contextRecord?.taskName ?? "",
    browserId: incident.browserId ?? contextRecord?.browserId ?? "",
    profileId: contextRecord?.profileId ?? "",
    sessionId: contextRecord?.sessionId ?? "",
    since: records[0]?.timestamp ?? incident.latestTimestamp,
    limit: 50,
  };
}

function traceHandoff(filters: TraceFilters) {
  return createServiceTraceHandoff({
    serviceName: filters.serviceName,
    agentName: filters.agentName,
    taskName: filters.taskName,
    browserId: filters.browserId,
    profileId: filters.profileId,
    sessionId: filters.sessionId,
    since: filters.since,
    limit: filters.limit,
  });
}

function traceQueryParams(filters: TraceFilters): URLSearchParams {
  return new URLSearchParams(
    Object.entries(traceHandoff(filters).query).map(([key, value]) => [key, String(value)]),
  );
}

function traceCliCommand(filters: TraceFilters): string {
  return traceHandoff(filters).cliCommand;
}

function traceHttpPath(filters: TraceFilters): string {
  return traceHandoff(filters).httpPath;
}

function incidentHandoff(filters: TraceFilters, trace: ServiceTraceData | null) {
  const singleIncidentId = trace?.incidents?.length === 1 ? trace.incidents[0]?.id : null;
  if (singleIncidentId) {
    return createServiceIncidentHandoff({
      incidentId: singleIncidentId,
      limit: 20,
    });
  }
  return createServiceIncidentHandoff({
    serviceName: filters.serviceName,
    agentName: filters.agentName,
    taskName: filters.taskName,
    browserId: filters.browserId,
    profileId: filters.profileId,
    sessionId: filters.sessionId,
    since: filters.since,
    state: "active",
    handlingState: "unacknowledged",
    summary: true,
    limit: 20,
  });
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
  onShowJobsForDisplayAllocation,
  onShowTraceJob,
  onShowTraceIncident,
  onLoad,
  onClear,
}: {
  filters: TraceFilters;
  trace: ServiceTraceData | null;
  loading: boolean;
  error: string;
  timeline: ServiceTraceTimelineItem[];
  onFiltersChange: (filters: TraceFilters) => void;
  onShowJobsForDisplayAllocation: (displayIsolation: string | null, jobIds?: string[]) => void;
  onShowTraceJob: (jobId: string) => void;
  onShowTraceIncident: (incidentId: string) => void;
  onLoad: () => void;
  onClear: () => void;
}) {
  const [copyStatus, setCopyStatus] = useState("");
  const counts = trace?.counts;
  const matched = trace?.matched;
  const summaryCards = traceSummaryCards(trace);
  const browserCapabilityLaunches = traceBrowserCapabilityLaunches(trace);
  const browserCapabilityLaunchSummary = trace?.summary?.browserCapabilityLaunches;
  const displayAllocationSummary = traceDisplayAllocationSummary(trace);
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
  const traceHandoffSummary = useMemo(() => traceHandoff(filters), [filters]);
  const incidentHandoffSummary = useMemo(() => incidentHandoff(filters, trace), [filters, trace]);
  const cliCommand = traceHandoffSummary.cliCommand;
  const httpPath = traceHandoffSummary.httpPath;
  const incidentCliCommand = incidentHandoffSummary.cliCommand;
  const incidentHttpPath = incidentHandoffSummary.httpPath;
  const incidentActivityCommand = incidentHandoffSummary.activityCliCommand;

  useEffect(() => {
    setCopyStatus("");
  }, [cliCommand, httpPath, incidentCliCommand, incidentHttpPath, incidentActivityCommand]);

  const copyTraceHandoff = useCallback(async (label: string, value: string) => {
    try {
      await navigator.clipboard.writeText(value);
      setCopyStatus(`Copied ${label}.`);
    } catch {
      setCopyStatus(`Clipboard unavailable. Select the ${label} text and copy it manually.`);
    }
  }, []);

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
      {hasFilters && (
        <div className="service-trace-handoff" aria-label="Trace handoff commands">
          <div>
            <span>Trace CLI</span>
            <code>{cliCommand}</code>
            <button
              type="button"
              className="service-trace-copy"
              onClick={() => void copyTraceHandoff("CLI trace command", cliCommand)}
            >
              <Copy className="size-3" />
              Copy
            </button>
          </div>
          <div>
            <span>Trace HTTP</span>
            <code>{httpPath}</code>
            <button
              type="button"
              className="service-trace-copy"
              onClick={() => void copyTraceHandoff("HTTP trace path", httpPath)}
            >
              <Copy className="size-3" />
              Copy
            </button>
          </div>
          <div>
            <span>Incidents CLI</span>
            <code>{incidentCliCommand}</code>
            <button
              type="button"
              className="service-trace-copy"
              onClick={() => void copyTraceHandoff("CLI incident command", incidentCliCommand)}
            >
              <Copy className="size-3" />
              Copy
            </button>
          </div>
          <div>
            <span>Incidents HTTP</span>
            <code>{incidentHttpPath}</code>
            <button
              type="button"
              className="service-trace-copy"
              onClick={() => void copyTraceHandoff("HTTP incident path", incidentHttpPath)}
            >
              <Copy className="size-3" />
              Copy
            </button>
          </div>
          {incidentActivityCommand && (
            <div>
              <span>Activity CLI</span>
              <code>{incidentActivityCommand}</code>
              <button
                type="button"
                className="service-trace-copy"
                onClick={() => void copyTraceHandoff("CLI incident activity command", incidentActivityCommand)}
              >
                <Copy className="size-3" />
                Copy
              </button>
            </div>
          )}
          {copyStatus && <p>{copyStatus}</p>}
        </div>
      )}
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
        <div className="service-trace-display-summary" aria-label="Trace display allocation summary">
          <div className="service-trace-contexts-header">
            <span>Display allocation</span>
            <Badge variant="outline" className="rounded-full px-2 py-0 text-[9px] uppercase">
              {displayAllocationSummary.recorded} recorded
            </Badge>
            {displayAllocationSummary.unrecorded > 0 && (
              <Badge variant="secondary" className="rounded-full px-2 py-0 text-[9px] uppercase">
                {displayAllocationSummary.unrecorded} unrecorded
              </Badge>
            )}
          </div>
          {displayAllocationSummary.allocations.length === 0 ? (
            <p className="rounded-2xl bg-foreground/[0.04] px-3 py-4 text-center text-xs text-muted-foreground">
              No display allocation intent was recorded for jobs in this trace.
            </p>
          ) : (
            <div className="service-trace-display-grid">
              {displayAllocationSummary.allocations.map((allocation) => (
                <button
                  type="button"
                  key={allocation.displayIsolation ?? "unknown"}
                  className="service-trace-display-card"
                  title={displayIsolationValueTitle(allocation.displayIsolation)}
                  onClick={() =>
                    onShowJobsForDisplayAllocation(allocation.displayIsolation ?? null, allocation.jobIds)
                  }
                >
                  <span>{allocation.label ?? displayIsolationLabel(allocation.displayIsolation)}</span>
                  <strong>{allocation.count ?? 0}</strong>
                  <small>{allocation.jobIds?.length ?? 0} jobs · show in Jobs</small>
                </button>
              ))}
              {displayAllocationSummary.unrecorded > 0 && (
                <button
                  type="button"
                  className="service-trace-display-card service-trace-display-card-muted"
                  title="Show jobs whose retained records do not include display allocation metadata"
                  onClick={() => onShowJobsForDisplayAllocation(null)}
                >
                  <span>Unrecorded</span>
                  <strong>{displayAllocationSummary.unrecorded}</strong>
                  <small>jobs · show in Jobs</small>
                </button>
              )}
            </div>
          )}
        </div>
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
                  <button
                    type="button"
                    key={`${wait.jobId}:${wait.startedAt ?? ""}:${wait.endedAt ?? ""}`}
                    className={cn("service-trace-wait-card", active && "service-trace-wait-card-active")}
                    onClick={() => onShowTraceJob(wait.jobId)}
                    title={`Show retained job ${wait.jobId} in Jobs`}
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
                    <small className="service-trace-card-action">Show job in Jobs</small>
                  </button>
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
            timeline.map((item) => {
              const jobId = typeof item.jobId === "string" && item.jobId.length > 0 ? item.jobId : null;
              const incidentId =
                typeof item.incidentId === "string" && item.incidentId.length > 0 ? item.incidentId : null;
              return (
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
                    {jobId && (
                      <button
                        type="button"
                        className="service-trace-timeline-job-link"
                        onClick={() => onShowTraceJob(jobId)}
                      >
                        Show job {jobId} in Jobs
                      </button>
                    )}
                    {incidentId && (
                      <button
                        type="button"
                        className="service-trace-timeline-incident-link"
                        onClick={() => onShowTraceIncident(incidentId)}
                      >
                        Show incident {incidentId} in Incidents
                      </button>
                    )}
                  </div>
                </div>
              );
            })
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
          {job.displayIsolation && (
            <Badge
              variant="secondary"
              className="h-4 max-w-32 truncate px-1.5 text-[9px]"
              title={displayIsolationValueTitle(job.displayIsolation)}
            >
              {displayIsolationLabel(job.displayIsolation)}
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

function JobSortButton({
  sortKey,
  activeSortKey,
  direction,
  onSort,
}: {
  sortKey: ServiceJobSortKey;
  activeSortKey: ServiceJobSortKey;
  direction: SortDirection;
  onSort: (sortKey: ServiceJobSortKey) => void;
}) {
  const active = sortKey === activeSortKey;
  return (
    <button
      type="button"
      className={cn("service-filter-chip", active && "service-filter-chip-active")}
      onClick={() => onSort(sortKey)}
      aria-label={`Sort jobs by ${SERVICE_JOB_SORT_LABELS[sortKey]}`}
    >
      {SERVICE_JOB_SORT_LABELS[sortKey]}
      <span aria-hidden="true">{active ? (direction === "asc" ? "↑" : "↓") : "↕"}</span>
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
  const controllable = canControlViewStream(stream);
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
        <Badge variant={controllable ? "default" : "secondary"} className="h-5 px-1.5 text-[9px]">
          {controlInputLabel(stream)}
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

function browserViewStreamCapability(browser?: ServiceBrowser | null): string {
  const stream = browserPrimaryViewStream(browser);
  if (!stream) return "none";
  return viewStreamCapabilityLabel(stream);
}

function displayIsolationLabel(value?: string | null): string {
  switch (value) {
    case "private_virtual_display":
      return "private display";
    case "shared_display":
      return "shared display";
    case "ambient_display":
      return "ambient display";
    default:
      return "unknown";
  }
}

function displayIsolationValueTitle(value?: string | null): string {
  switch (value) {
    case "private_virtual_display":
      return "This request or browser uses its own service-managed virtual display.";
    case "shared_display":
      return "This request or browser uses an explicitly configured shared display.";
    case "ambient_display":
      return "This request or browser uses the host DISPLAY inherited by the daemon.";
    default:
      return "Display allocation was not recorded.";
  }
}

function displayIsolationTitle(browser: ServiceBrowser): string {
  if (browser.displayIsolation) {
    return displayIsolationValueTitle(browser.displayIsolation);
  }
  return browser.host === "remote_headed"
    ? "Display isolation was not recorded for this remote-headed browser."
    : "Display isolation applies to remote-headed browser hosts.";
}

function RemoteViewReadinessStrip({ browser, stream }: { browser: ServiceBrowser; stream?: ServiceViewStream | null }) {
  const viewReady = canOpenViewStream(stream);
  const controlReady = canOpenControlViewStream(stream);
  return (
    <div className="service-remote-view-readiness" aria-label="Remote view readiness">
      <div>
        <span>Remote view</span>
        <strong>{viewReady ? "ready" : "not ready"}</strong>
      </div>
      <div>
        <span>Remote control</span>
        <strong>{controlReady ? "ready" : "view only"}</strong>
      </div>
      <div>
        <span>Provider</span>
        <strong>{stream ? viewStreamLabel(stream) : "none"}</strong>
      </div>
      <div>
        <span>Input</span>
        <strong>{stream ? controlInputLabel(stream) : "none"}</strong>
      </div>
      <div>
        <span>Display</span>
        <strong title={displayIsolationTitle(browser)}>
          {displayIsolationLabel(browser.displayIsolation)}
        </strong>
      </div>
      <p>
        {stream?.url
          ? `Gateway URL: ${stream.url}${browser.displayName ? ` / Display: ${browser.displayName}` : ""}`
          : stream
            ? viewStreamOpenTitle(stream)
            : "No service-owned view stream has been recorded for this browser."}
      </p>
    </div>
  );
}

type BrowserSortKey = "health" | "id" | "profile" | "host" | "sessions" | "streams";
type SortDirection = "asc" | "desc";
type BrowserLifecycleFilter = "actionable" | "all" | "live" | "retained";
type BrowserTableColumnKey = "health" | "profile" | "host" | "ownership" | "sessions" | "streams" | "lastError";
type BrowserTableColumnId = BrowserTableColumnKey | "id" | "actions";
type BrowserTableDensity = "compact" | "standard" | "expanded";
type BrowserStreamFilter = "all" | "with_stream" | "without_stream";
type BrowserOwnershipSummary = {
  serviceNames: string[];
  agentNames: string[];
  taskNames: string[];
  sessionIds: string[];
};

const EMPTY_BROWSER_OWNERSHIP: BrowserOwnershipSummary = {
  serviceNames: [],
  agentNames: [],
  taskNames: [],
  sessionIds: [],
};

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
  { key: "ownership", label: "Ownership" },
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
const BROWSER_TABLE_INITIAL_ROW_LIMIT = 50;
const BROWSER_TABLE_ROW_LIMIT_STEP = 50;
const TERMINAL_BROWSER_HEALTH = new Set(["closed", "faulted", "not_started", "process_exited"]);
const DEFAULT_BROWSER_TABLE_COLUMN_WIDTHS: Record<BrowserTableColumnId, number> = {
  health: 132,
  id: 220,
  profile: 180,
  host: 190,
  ownership: 230,
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
    browser.browserBuild,
    browser.pid ? `pid ${browser.pid}` : "retained",
    browser.cdpEndpoint,
    browser.lastError,
    ...(browser.activeSessionIds ?? []),
  ].filter(Boolean).join(" ").toLowerCase();
}

function browserOwnershipSummary(browser: ServiceBrowser, sessions: ServiceSession[]): BrowserOwnershipSummary {
  const linkedSessions = sessions.filter((session) =>
    Boolean(
      (browser.id && session.browserIds?.includes(browser.id)) ||
        (session.id && browser.activeSessionIds?.includes(session.id)),
    ),
  );
  return {
    serviceNames: uniqueStringValues(linkedSessions.map((session) => session.serviceName)),
    agentNames: uniqueStringValues(linkedSessions.map((session) => session.agentName)),
    taskNames: uniqueStringValues(linkedSessions.map((session) => session.taskName)),
    sessionIds: uniqueStringValues(linkedSessions.map((session) => session.id)),
  };
}

function browserOwnershipSearchText(ownership: BrowserOwnershipSummary): string {
  return [
    ...ownership.serviceNames,
    ...ownership.agentNames,
    ...ownership.taskNames,
    ...ownership.sessionIds,
  ].join(" ").toLowerCase();
}

function browserFilterOptionValues(browsers: ServiceBrowser[], field: "health" | "host" | "browserBuild"): string[] {
  return Array.from(new Set(
    browsers
      .map((browser) => browser[field])
      .filter((value): value is string => Boolean(value)),
  )).sort((left, right) => left.localeCompare(right, undefined, { numeric: true, sensitivity: "base" }));
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

function serviceJobDisplayMatchesFilter(job: ServiceJob, filter: ServiceJobDisplayFilter): boolean {
  if (filter === "all") return true;
  if (filter === "unrecorded") return !job.displayIsolation;
  return job.displayIsolation === filter;
}

function serviceJobDisplayFilterForTraceAllocation(displayIsolation: string | null): ServiceJobDisplayFilter {
  if (
    displayIsolation === "private_virtual_display" ||
    displayIsolation === "shared_display" ||
    displayIsolation === "ambient_display"
  ) {
    return displayIsolation;
  }
  if (!displayIsolation) return "unrecorded";
  return "all";
}

function serviceJobSortValue(job: ServiceJob, sortKey: ServiceJobSortKey): string | number {
  if (sortKey === "submittedAt") {
    const timestamp = job.submittedAt ? new Date(job.submittedAt).getTime() : 0;
    return Number.isFinite(timestamp) ? timestamp : 0;
  }
  if (sortKey === "state") return job.state ?? "";
  if (sortKey === "action") return job.action ?? "";
  if (sortKey === "displayIsolation") return displayIsolationLabel(job.displayIsolation);
  if (sortKey === "serviceName") return job.serviceName ?? "";
  return job.taskName ?? "";
}

function isLiveBrowserRecord(browser: ServiceBrowser): boolean {
  if (TERMINAL_BROWSER_HEALTH.has((browser.health ?? "").toLowerCase())) return false;
  return Boolean(
    browser.pid ||
      browser.cdpEndpoint ||
      (browser.activeSessionIds?.length ?? 0) > 0 ||
      (browser.viewStreams?.length ?? 0) > 0 ||
      ["launching", "ready", "degraded", "cdp_disconnected", "reconnecting", "closing"].includes(browser.health ?? ""),
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
  sessions,
  onSelect,
  onViewStream,
  onFocusViewStream,
  onCloseBrowser,
  onRepairBrowser,
  closeSupported,
  repairSupported,
  activeSessionName,
  actingBrowserActionId,
  selectedBrowserId,
}: {
  browsers: ServiceBrowser[];
  sessions: ServiceSession[];
  onSelect: (browser: ServiceBrowser) => void;
  onViewStream?: (browser: ServiceBrowser) => void;
  onFocusViewStream?: (browser: ServiceBrowser) => void;
  onCloseBrowser?: (browser: ServiceBrowser) => void;
  onRepairBrowser?: (browser: ServiceBrowser) => void;
  closeSupported?: boolean;
  repairSupported?: boolean;
  activeSessionName?: string | null;
  actingBrowserActionId?: string | null;
  selectedBrowserId?: string | null;
}) {
  const [filter, setFilter] = useState("");
  const [lifecycleFilter, setLifecycleFilter] = useState<BrowserLifecycleFilter>(initialBrowserLifecycleFilter);
  const [visibleColumns, setVisibleColumns] = useState<BrowserTableColumnKey[]>(initialBrowserTableColumns);
  const [columnWidths, setColumnWidths] = useState<Record<BrowserTableColumnId, number>>(initialBrowserTableColumnWidths);
  const [density, setDensity] = useState<BrowserTableDensity>(initialBrowserTableDensity);
  const [sortKey, setSortKey] = useState<BrowserSortKey>("health");
  const [sortDirection, setSortDirection] = useState<SortDirection>("asc");
  const [rowLimit, setRowLimit] = useState(BROWSER_TABLE_INITIAL_ROW_LIMIT);
  const [healthFilter, setHealthFilter] = useState("all");
  const [hostFilter, setHostFilter] = useState("all");
  const [browserBuildFilter, setBrowserBuildFilter] = useState("all");
  const [streamFilter, setStreamFilter] = useState<BrowserStreamFilter>("all");
  const [ownershipServiceFilter, setOwnershipServiceFilter] = useState("all");
  const [ownershipAgentFilter, setOwnershipAgentFilter] = useState("all");
  const [ownershipTaskFilter, setOwnershipTaskFilter] = useState("all");
  const resizeStateRef = useRef<{ column: BrowserTableColumnId; startX: number; startWidth: number } | null>(null);
  const rowButtonRefs = useRef(new Map<string, HTMLButtonElement>());
  const visibleColumnSet = useMemo(() => new Set(visibleColumns), [visibleColumns]);
  const liveCount = useMemo(() => browsers.filter(isLiveBrowserRecord).length, [browsers]);
  const inertCount = useMemo(() => browsers.filter(isInertRetainedBrowserRecord).length, [browsers]);
  const healthOptions = useMemo(() => browserFilterOptionValues(browsers, "health"), [browsers]);
  const hostOptions = useMemo(() => browserFilterOptionValues(browsers, "host"), [browsers]);
  const browserBuildOptions = useMemo(() => browserFilterOptionValues(browsers, "browserBuild"), [browsers]);
  const browserOwnershipById = useMemo(
    () => new Map(browsers.map((browser) => [browser.id, browserOwnershipSummary(browser, sessions)])),
    [browsers, sessions],
  );
  const browserOwnershipValues = useMemo(() => Array.from(browserOwnershipById.values()), [browserOwnershipById]);
  const ownershipServiceOptions = useMemo(
    () => uniqueStringValues(browserOwnershipValues.flatMap((ownership) => ownership.serviceNames)),
    [browserOwnershipValues],
  );
  const ownershipAgentOptions = useMemo(
    () => uniqueStringValues(browserOwnershipValues.flatMap((ownership) => ownership.agentNames)),
    [browserOwnershipValues],
  );
  const ownershipTaskOptions = useMemo(
    () => uniqueStringValues(browserOwnershipValues.flatMap((ownership) => ownership.taskNames)),
    [browserOwnershipValues],
  );
  const activeTableColumns = useMemo(
    () => (["health", "id", "profile", "host", "ownership", "sessions", "streams", "lastError", "actions"] as BrowserTableColumnId[])
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
      if (healthFilter !== "all" && browser.health !== healthFilter) return false;
      if (hostFilter !== "all" && browser.host !== hostFilter) return false;
      if (browserBuildFilter !== "all" && browser.browserBuild !== browserBuildFilter) return false;
      const hasViewStream = (browser.viewStreams?.length ?? 0) > 0;
      if (streamFilter === "with_stream" && !hasViewStream) return false;
      if (streamFilter === "without_stream" && hasViewStream) return false;
      const ownership = browserOwnershipById.get(browser.id) ?? EMPTY_BROWSER_OWNERSHIP;
      if (ownershipServiceFilter !== "all" && !ownership.serviceNames.includes(ownershipServiceFilter)) return false;
      if (ownershipAgentFilter !== "all" && !ownership.agentNames.includes(ownershipAgentFilter)) return false;
      if (ownershipTaskFilter !== "all" && !ownership.taskNames.includes(ownershipTaskFilter)) return false;
      return query ? `${browserSearchText(browser)} ${browserOwnershipSearchText(ownership)}`.includes(query) : true;
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
  }, [browserBuildFilter, browserOwnershipById, browsers, filter, healthFilter, hostFilter, lifecycleFilter, ownershipAgentFilter, ownershipServiceFilter, ownershipTaskFilter, sortDirection, sortKey, streamFilter]);
  const visibleBrowsers = useMemo(
    () => filteredBrowsers.slice(0, rowLimit),
    [filteredBrowsers, rowLimit],
  );
  const hiddenBrowserCount = Math.max(0, filteredBrowsers.length - visibleBrowsers.length);

  useEffect(() => {
    setRowLimit(BROWSER_TABLE_INITIAL_ROW_LIMIT);
  }, [browserBuildFilter, filter, healthFilter, hostFilter, lifecycleFilter, ownershipAgentFilter, ownershipServiceFilter, ownershipTaskFilter, sortDirection, sortKey, streamFilter]);

  const setRowButtonRef = (browserId: string, node: HTMLButtonElement | null) => {
    if (node) {
      rowButtonRefs.current.set(browserId, node);
      return;
    }
    rowButtonRefs.current.delete(browserId);
  };

  const focusBrowserRow = (index: number) => {
    const nextBrowser = visibleBrowsers[index];
    if (!nextBrowser?.id) return;
    onSelect(nextBrowser);
    rowButtonRefs.current.get(nextBrowser.id)?.focus();
  };

  const navigateBrowserRows = (browser: ServiceBrowser, event: ReactKeyboardEvent<HTMLButtonElement>) => {
    if (!browser.id) return;
    const currentIndex = visibleBrowsers.findIndex((row) => row.id === browser.id);
    if (currentIndex < 0) return;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      focusBrowserRow(Math.min(visibleBrowsers.length - 1, currentIndex + 1));
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      focusBrowserRow(Math.max(0, currentIndex - 1));
      return;
    }
    if (event.key === "Home") {
      event.preventDefault();
      focusBrowserRow(0);
      return;
    }
    if (event.key === "End") {
      event.preventDefault();
      focusBrowserRow(visibleBrowsers.length - 1);
    }
  };

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
    setHealthFilter("all");
    setHostFilter("all");
    setBrowserBuildFilter("all");
    setStreamFilter("all");
    setOwnershipServiceFilter("all");
    setOwnershipAgentFilter("all");
    setOwnershipTaskFilter("all");
    setVisibleColumns(DEFAULT_BROWSER_TABLE_COLUMNS);
    setColumnWidths(DEFAULT_BROWSER_TABLE_COLUMN_WIDTHS);
    setDensity("standard");
    setRowLimit(BROWSER_TABLE_INITIAL_ROW_LIMIT);
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
      <p id="service-browser-table-keyboard-hint" className="sr-only">
        Browser row links support Arrow Up, Arrow Down, Home, and End within the visible row window.
      </p>
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
          {visibleBrowsers.length} of {filteredBrowsers.length} filtered; {browsers.length} total, {liveCount} live, {inertCount} inert retained
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
        <div className="service-browser-table-advanced-filters" aria-label="Browser table field filters">
          <label>
            <span>Health</span>
            <select value={healthFilter} onChange={(event) => setHealthFilter(event.target.value)}>
              <option value="all">All health states</option>
              {healthOptions.map((value) => <option key={value} value={value}>{value}</option>)}
            </select>
          </label>
          <label>
            <span>Host</span>
            <select value={hostFilter} onChange={(event) => setHostFilter(event.target.value)}>
              <option value="all">All hosts</option>
              {hostOptions.map((value) => <option key={value} value={value}>{value}</option>)}
            </select>
          </label>
          {browserBuildOptions.length > 0 && (
            <label>
              <span>Build</span>
              <select value={browserBuildFilter} onChange={(event) => setBrowserBuildFilter(event.target.value)}>
                <option value="all">All builds</option>
                {browserBuildOptions.map((value) => <option key={value} value={value}>{value}</option>)}
              </select>
            </label>
          )}
          <label>
            <span>Streams</span>
            <select value={streamFilter} onChange={(event) => setStreamFilter(event.target.value as BrowserStreamFilter)}>
              <option value="all">Any stream state</option>
              <option value="with_stream">View stream available</option>
              <option value="without_stream">No view stream</option>
            </select>
          </label>
          {ownershipServiceOptions.length > 0 && (
            <label>
              <span>Service</span>
              <select value={ownershipServiceFilter} onChange={(event) => setOwnershipServiceFilter(event.target.value)}>
                <option value="all">All services</option>
                {ownershipServiceOptions.map((value) => <option key={value} value={value}>{value}</option>)}
              </select>
            </label>
          )}
          {ownershipAgentOptions.length > 0 && (
            <label>
              <span>Agent</span>
              <select value={ownershipAgentFilter} onChange={(event) => setOwnershipAgentFilter(event.target.value)}>
                <option value="all">All agents</option>
                {ownershipAgentOptions.map((value) => <option key={value} value={value}>{value}</option>)}
              </select>
            </label>
          )}
          {ownershipTaskOptions.length > 0 && (
            <label>
              <span>Task</span>
              <select value={ownershipTaskFilter} onChange={(event) => setOwnershipTaskFilter(event.target.value)}>
                <option value="all">All tasks</option>
                {ownershipTaskOptions.map((value) => <option key={value} value={value}>{value}</option>)}
              </select>
            </label>
          )}
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
              {visibleColumnSet.has("ownership") && (
                <BrowserTableHeaderCell column="ownership" width={columnWidths.ownership} label="Ownership" onResizeStart={startColumnResize} onResetWidth={resetColumnWidth}>
                  Ownership
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
              visibleBrowsers.map((browser, index) => (
                <BrowserTableRow
                  key={browser.id || browser.cdpEndpoint || `browser-${index}`}
                  browser={browser}
                  ownership={browserOwnershipById.get(browser.id) ?? EMPTY_BROWSER_OWNERSHIP}
                  selected={Boolean(browser.id && browser.id === selectedBrowserId)}
                  visibleColumns={visibleColumnSet}
                  onSelect={onSelect}
                  onViewStream={onViewStream}
                  onFocusViewStream={onFocusViewStream}
                  onCloseBrowser={onCloseBrowser}
                  onRepairBrowser={onRepairBrowser}
                  closeSupported={closeSupported}
                  repairSupported={repairSupported}
                  activeSessionName={activeSessionName}
                  acting={Boolean(actingBrowserActionId && browser.id === actingBrowserActionId)}
                  onNavigate={navigateBrowserRows}
                  rowButtonRef={browser.id ? (node) => setRowButtonRef(browser.id, node) : undefined}
                  density={density}
                />
              ))
            )}
          </tbody>
        </table>
      </div>
      <div className="service-browser-card-list" aria-label="Managed browser cards">
        {filteredBrowsers.length === 0 ? (
          <p className="service-browser-card-empty">
            No browser records match the current filter.
          </p>
        ) : (
          visibleBrowsers.map((browser, index) => (
            <BrowserTableCard
              key={browser.id || browser.cdpEndpoint || `browser-card-${index}`}
              browser={browser}
              ownership={browserOwnershipById.get(browser.id) ?? EMPTY_BROWSER_OWNERSHIP}
              selected={Boolean(browser.id && browser.id === selectedBrowserId)}
              onSelect={onSelect}
              onViewStream={onViewStream}
              onFocusViewStream={onFocusViewStream}
              onCloseBrowser={onCloseBrowser}
              onRepairBrowser={onRepairBrowser}
              closeSupported={closeSupported}
              repairSupported={repairSupported}
              activeSessionName={activeSessionName}
              acting={Boolean(actingBrowserActionId && browser.id === actingBrowserActionId)}
            />
          ))
        )}
      </div>
      {hiddenBrowserCount > 0 && (
        <div className="service-browser-table-window" aria-live="polite">
          <span>{hiddenBrowserCount} more browser records match this view.</span>
          <button
            type="button"
            className="service-filter-chip"
            onClick={() => setRowLimit((current) => current + BROWSER_TABLE_ROW_LIMIT_STEP)}
          >
            Show {Math.min(BROWSER_TABLE_ROW_LIMIT_STEP, hiddenBrowserCount)} more
          </button>
          <button
            type="button"
            className="service-filter-chip"
            onClick={() => setRowLimit(filteredBrowsers.length)}
          >
            Show all
          </button>
        </div>
      )}
    </div>
  );
}

function BrowserRowActions({
  browser,
  onSelect,
  onViewStream,
  onFocusViewStream,
  onCloseBrowser,
  onRepairBrowser,
  closeSupported,
  repairSupported,
  activeSessionName,
  acting,
  density,
}: {
  browser: ServiceBrowser;
  onSelect: (browser: ServiceBrowser) => void;
  onViewStream?: (browser: ServiceBrowser) => void;
  onFocusViewStream?: (browser: ServiceBrowser) => void;
  onCloseBrowser?: (browser: ServiceBrowser) => void;
  onRepairBrowser?: (browser: ServiceBrowser) => void;
  closeSupported?: boolean;
  repairSupported?: boolean;
  activeSessionName?: string | null;
  acting?: boolean;
  density: BrowserTableDensity;
}) {
  const primaryViewStream = browserPrimaryViewStream(browser);
  const viewStreamAvailable = canOpenViewStream(primaryViewStream);
  const controlAvailable = canOpenControlViewStream(primaryViewStream);
  const closeAvailable = Boolean(closeSupported && onCloseBrowser && activeSessionName && browser.id === `session:${activeSessionName}`);
  const repairAvailable = Boolean(repairSupported && onRepairBrowser && ["degraded", "faulted"].includes((browser.health ?? "").toLowerCase()));
  const closeTitle = browserRowCloseTitle({
    available: closeAvailable,
    supported: Boolean(closeSupported),
  });
  const repairTitle = browserRowRepairTitle({
    available: repairAvailable,
    supported: Boolean(repairSupported),
  });
  const unavailableActionCount = [
    !viewStreamAvailable || !onViewStream,
    !controlAvailable || !onFocusViewStream,
    !closeAvailable,
    !repairAvailable,
  ].filter(Boolean).length;

  return (
    <div className="service-browser-row-actions" aria-label={`Browser actions for ${browser.id || "unnamed browser"}`}>
      <Button
        type="button"
        size="sm"
        variant="outline"
        className={cn("px-2 text-[10px]", density === "compact" ? "h-6" : "h-7")}
        onClick={() => onSelect(browser)}
      >
        Inspect
      </Button>
      {viewStreamAvailable && onViewStream && (
        <Button
          type="button"
          size="sm"
          variant="outline"
          className={cn("px-2 text-[10px]", density === "compact" ? "h-6" : "h-7")}
          title={viewStreamOpenTitle(primaryViewStream)}
          onClick={() => onViewStream(browser)}
        >
          View
        </Button>
      )}
      {controlAvailable && onFocusViewStream && (
        <Button
          type="button"
          size="sm"
          variant="outline"
          className={cn("px-2 text-[10px]", density === "compact" ? "h-6" : "h-7")}
          title={viewStreamControlTitle(primaryViewStream)}
          onClick={() => onFocusViewStream(browser)}
        >
          Control
        </Button>
      )}
      {closeAvailable && (
        <AlertDialog>
          <AlertDialogTrigger asChild>
            <Button
              type="button"
              size="sm"
              variant="outline"
              className={cn("px-2 text-[10px]", density === "compact" ? "h-6" : "h-7")}
              disabled={acting}
              title={closeTitle}
            >
              Close
            </Button>
          </AlertDialogTrigger>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Close service browser?</AlertDialogTitle>
              <AlertDialogDescription>
                This queues a polite close for {browser.id || "the selected browser"}. If polite close fails, agent-browser records degraded shutdown health.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction onClick={() => onCloseBrowser?.(browser)}>Close browser</AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      )}
      {repairAvailable && (
        <Button
          type="button"
          size="sm"
          variant="outline"
          className={cn("px-2 text-[10px]", density === "compact" ? "h-6" : "h-7")}
          disabled={acting}
          title={repairTitle}
          onClick={() => onRepairBrowser?.(browser)}
        >
          Repair
        </Button>
      )}
      {unavailableActionCount > 0 && (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              type="button"
              size="sm"
              variant="outline"
              className={cn("px-2 text-[10px]", density === "compact" ? "h-6" : "h-7")}
              aria-label={`Show unavailable actions for ${browser.id || "unnamed browser"}`}
              title={`${unavailableActionCount} unavailable row actions`}
            >
              <MoreHorizontal className="size-3" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-64">
            <DropdownMenuLabel>Unavailable actions</DropdownMenuLabel>
            {(!viewStreamAvailable || !onViewStream) && (
              <DropdownMenuItem disabled title={viewStreamOpenTitle(primaryViewStream)}>
                View: {viewStreamOpenTitle(primaryViewStream)}
              </DropdownMenuItem>
            )}
            {(!controlAvailable || !onFocusViewStream) && (
              <DropdownMenuItem disabled title={viewStreamControlTitle(primaryViewStream)}>
                Control: {viewStreamControlTitle(primaryViewStream)}
              </DropdownMenuItem>
            )}
            {!closeAvailable && (
              <DropdownMenuItem disabled title={closeTitle}>
                Close: {closeTitle}
              </DropdownMenuItem>
            )}
            {!repairAvailable && (
              <DropdownMenuItem disabled title={repairTitle}>
                Repair: {repairTitle}
              </DropdownMenuItem>
            )}
          </DropdownMenuContent>
        </DropdownMenu>
      )}
    </div>
  );
}

function BrowserTableRow({
  browser,
  ownership,
  selected,
  visibleColumns,
  onSelect,
  onViewStream,
  onFocusViewStream,
  onCloseBrowser,
  onRepairBrowser,
  closeSupported,
  repairSupported,
  activeSessionName,
  acting,
  onNavigate,
  rowButtonRef,
  density,
}: {
  browser: ServiceBrowser;
  ownership: BrowserOwnershipSummary;
  selected: boolean;
  visibleColumns: Set<BrowserTableColumnKey>;
  onSelect: (browser: ServiceBrowser) => void;
  onViewStream?: (browser: ServiceBrowser) => void;
  onFocusViewStream?: (browser: ServiceBrowser) => void;
  onCloseBrowser?: (browser: ServiceBrowser) => void;
  onRepairBrowser?: (browser: ServiceBrowser) => void;
  closeSupported?: boolean;
  repairSupported?: boolean;
  activeSessionName?: string | null;
  acting?: boolean;
  onNavigate: (browser: ServiceBrowser, event: ReactKeyboardEvent<HTMLButtonElement>) => void;
  rowButtonRef?: (node: HTMLButtonElement | null) => void;
  density: BrowserTableDensity;
}) {
  const tone = healthTone(browser.health);
  const sessionCount = browser.activeSessionIds?.length ?? 0;
  const viewStreamCount = browser.viewStreams?.length ?? 0;
  const viewStreamCapability = browserViewStreamCapability(browser);
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
          ref={rowButtonRef}
          className={cn("service-browser-table-id", selected && "service-browser-table-id-selected")}
          onClick={() => onSelect(browser)}
          onKeyDown={(event) => onNavigate(browser, event)}
          aria-label={`Inspect browser ${browser.id}`}
          aria-current={selected ? "true" : undefined}
          aria-describedby="service-browser-table-keyboard-hint"
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
      {visibleColumns.has("ownership") && (
        <td>
          <BrowserOwnershipCell ownership={ownership} />
        </td>
      )}
      {visibleColumns.has("sessions") && <td className="service-browser-table-number">{sessionCount}</td>}
      {visibleColumns.has("streams") && (
        <td>
          <div className="service-browser-table-streams">
            <span className="service-browser-table-number">{viewStreamCount}</span>
            <span>{viewStreamCapability}</span>
          </div>
        </td>
      )}
      {visibleColumns.has("lastError") && (
        <td className={cn("service-browser-table-error", !browser.lastError && "service-browser-table-cell-muted")}>
          {browser.lastError || "none"}
        </td>
      )}
      <td>
        <BrowserRowActions
          browser={browser}
          onSelect={onSelect}
          onViewStream={onViewStream}
          onFocusViewStream={onFocusViewStream}
          onCloseBrowser={onCloseBrowser}
          onRepairBrowser={onRepairBrowser}
          closeSupported={closeSupported}
          repairSupported={repairSupported}
          activeSessionName={activeSessionName}
          acting={acting}
          density={density}
        />
      </td>
    </tr>
  );
}

function BrowserTableCard({
  browser,
  ownership,
  selected,
  onSelect,
  onViewStream,
  onFocusViewStream,
  onCloseBrowser,
  onRepairBrowser,
  closeSupported,
  repairSupported,
  activeSessionName,
  acting,
}: {
  browser: ServiceBrowser;
  ownership: BrowserOwnershipSummary;
  selected: boolean;
  onSelect: (browser: ServiceBrowser) => void;
  onViewStream?: (browser: ServiceBrowser) => void;
  onFocusViewStream?: (browser: ServiceBrowser) => void;
  onCloseBrowser?: (browser: ServiceBrowser) => void;
  onRepairBrowser?: (browser: ServiceBrowser) => void;
  closeSupported?: boolean;
  repairSupported?: boolean;
  activeSessionName?: string | null;
  acting?: boolean;
}) {
  const tone = healthTone(browser.health);
  const processLabel = browser.pid ? `pid ${browser.pid}` : "retained";
  const viewStreamCapability = browserViewStreamCapability(browser);
  return (
    <article className={cn("service-browser-card", selected && "service-browser-card-selected")}>
      <button
        type="button"
        className="service-browser-card-primary"
        onClick={() => onSelect(browser)}
        aria-label={`Inspect browser ${browser.id || "unnamed browser"}`}
        aria-current={selected ? "true" : undefined}
      >
        <span className={cn("service-browser-health-dot", `service-browser-health-${tone}`)} />
        <span className="min-w-0 flex-1">
          <span className="service-browser-card-title">{browser.id || "unnamed browser"}</span>
          <span className="service-browser-card-subtitle">
            {browser.host ?? "unknown host"} / {processLabel}
          </span>
        </span>
        <Badge variant="outline" className="h-5 max-w-28 truncate px-1.5 text-[9px]">
          {browser.health ?? "unknown"}
        </Badge>
      </button>
      <div className="service-browser-card-grid">
        <span>
          <strong>Profile</strong>
          {browser.profileId || "unassigned"}
        </span>
        <span>
          <strong>Sessions</strong>
          {browser.activeSessionIds?.length ?? 0}
        </span>
        <span>
          <strong>Streams</strong>
          {(browser.viewStreams?.length ?? 0)} / {viewStreamCapability}
        </span>
        <span>
          <strong>Error</strong>
          {browser.lastError || "none"}
        </span>
      </div>
      <BrowserOwnershipCell ownership={ownership} />
      <BrowserRowActions
        browser={browser}
        onSelect={onSelect}
        onViewStream={onViewStream}
        onFocusViewStream={onFocusViewStream}
        onCloseBrowser={onCloseBrowser}
        onRepairBrowser={onRepairBrowser}
        closeSupported={closeSupported}
        repairSupported={repairSupported}
        activeSessionName={activeSessionName}
        acting={acting}
        density="standard"
      />
    </article>
  );
}

function BrowserOwnershipCell({ ownership }: { ownership: BrowserOwnershipSummary }) {
  const hasOwnership =
    ownership.serviceNames.length > 0 ||
    ownership.agentNames.length > 0 ||
    ownership.taskNames.length > 0;
  if (!hasOwnership) {
    return <span className="service-browser-table-cell-muted">unassigned</span>;
  }
  return (
    <div className="service-browser-ownership-cell">
      <span className="service-browser-ownership-chip service-browser-ownership-service">
        svc {formatStringList(ownership.serviceNames, "unknown")}
      </span>
      <span className="service-browser-ownership-chip">
        agent {formatStringList(ownership.agentNames, "unknown")}
      </span>
      <span className="service-browser-ownership-chip">
        task {formatStringList(ownership.taskNames, "unknown")}
      </span>
    </div>
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
                  <div className="mt-2 flex flex-wrap gap-1.5">
                    <Badge variant="outline" className="h-5 px-1.5 text-[9px]">
                      view {viewStreamLabel(stream)}
                    </Badge>
                    <Badge variant={canControlViewStream(stream) ? "default" : "secondary"} className="h-5 px-1.5 text-[9px]">
                      input {controlInputLabel(stream)}
                    </Badge>
                  </div>
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
              {!canControlViewStream(stream) && (
                <p className="service-browser-control-note">
                  This stream is available for viewing. The service has not marked it as operator-controllable.
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
  onControlBrowser,
}: {
  browser: ServiceBrowser;
  onInspectViewStream?: (stream: ServiceViewStream, browser: ServiceBrowser) => void;
  onControlBrowser?: (browser: ServiceBrowser) => void;
}) {
  const viewStreamCount = browser.viewStreams?.length ?? 0;
  const primaryViewStream = browserPrimaryViewStream(browser);
  const controlAvailable = canOpenControlViewStream(primaryViewStream);
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
        <EventDetailItem label="Display isolation" value={displayIsolationLabel(browser.displayIsolation)} />
        <EventDetailItem label="Display" value={browser.displayName} />
        <EventDetailItem label="PID" value={browser.pid ? String(browser.pid) : null} />
        <EventDetailItem label="CDP endpoint" value={browser.cdpEndpoint} />
        <EventDetailItem label="Active sessions" value={String(browser.activeSessionIds?.length ?? 0)} />
        <EventDetailItem label="View streams" value={String(viewStreamCount)} />
        <EventDetailItem label="Primary view" value={primaryViewStream ? viewStreamLabel(primaryViewStream) : null} />
        <EventDetailItem label="Primary input" value={primaryViewStream ? controlInputLabel(primaryViewStream) : null} />
      </div>
      <RemoteViewReadinessStrip browser={browser} stream={primaryViewStream} />
      {onControlBrowser && (
        <Button
          type="button"
          size="sm"
          className="w-fit gap-1.5 rounded-full"
          disabled={!controlAvailable}
          title={viewStreamControlTitle(primaryViewStream)}
          onClick={() => onControlBrowser(browser)}
        >
          <Eye className="size-3.5" />
          Open remote control
        </Button>
      )}
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
        {selection.kind === "browser" && (
          <BrowserDetailContent browser={selection.browser} onControlBrowser={actions.onControlBrowser} />
        )}
        {selection.kind === "profile" && <ProfileAllocationDetailContent allocation={selection.allocation} />}
        {selection.kind === "incident" && (
          <IncidentDetailContent
            incident={selection.incident}
            acting={actions.actingIncidentId === selection.incident.id}
            onAcknowledge={actions.onAcknowledgeIncident}
            onResolve={actions.onResolveIncident}
            onShowTrace={actions.onShowIncidentTrace}
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
        <EventDetailItem label="Display allocation" value={job.displayIsolation ? displayIsolationLabel(job.displayIsolation) : null} />
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
  onNavigate,
  rowRef,
  selected,
}: {
  allocation: ServiceProfileAllocation;
  onSelect: (allocation: ServiceProfileAllocation) => void;
  onNavigate: (allocation: ServiceProfileAllocation, event: ReactKeyboardEvent<HTMLButtonElement>) => void;
  rowRef?: (node: HTMLButtonElement | null) => void;
  selected: boolean;
}) {
  const tone = profileAllocationTone(allocation.leaseState);
  const holderCount = allocation.holderCount ?? allocation.holderSessionIds?.length ?? 0;
  const waitingCount = allocation.waitingJobCount ?? allocation.waitingJobIds?.length ?? 0;
  const primaryTarget = profileAllocationPrimaryTarget(allocation);
  const primaryLogin = profileAllocationPrimaryLogin(allocation);
  const primaryBrowser = profileAllocationPrimaryBrowser(allocation);
  const readinessAttention = profileReadinessNeedsAttention(allocation.targetReadiness);
  return (
    <button
      type="button"
      ref={rowRef}
      className={cn("service-browser-row service-profile-allocation-row", selected && "service-profile-allocation-row-selected")}
      onClick={() => onSelect(allocation)}
      onKeyDown={(event) => onNavigate(allocation, event)}
      aria-label={`Inspect profile allocation ${allocation.profileId}`}
      aria-current={selected ? "true" : undefined}
      aria-describedby="service-profile-allocation-keyboard-hint"
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
          {readinessAttention && (
            <Badge variant="outline" className="service-profile-attention-badge h-4 max-w-36 truncate px-1.5 text-[9px]">
              readiness attention
            </Badge>
          )}
        </div>
        <div className="service-profile-route-grid">
          <span className="service-profile-route-cell">
            <strong>Target</strong>
            <span>{primaryTarget}</span>
          </span>
          <span className="service-profile-route-cell">
            <strong>Login</strong>
            <span>{primaryLogin}</span>
          </span>
          <span className="service-profile-route-cell">
            <strong>Browser build</strong>
            <span>{allocation.browserBuild ?? "service default"}</span>
          </span>
          <span className="service-profile-route-cell">
            <strong>Keyring</strong>
            <span>{allocation.keyring ?? "default policy"}</span>
          </span>
        </div>
        <div className="service-profile-route-detail">
          <span>Browser: {primaryBrowser}</span>
          <span>{holderCount} holders</span>
          <span>{waitingCount} waiting</span>
          <span>{allocation.tabIds?.length ?? 0} tabs</span>
        </div>
        <div className="mt-2 grid gap-1 text-[10px] text-muted-foreground sm:grid-cols-2">
          <span className="truncate">service: {formatStringList(allocation.serviceNames)}</span>
          <span className="truncate">agent: {formatStringList(allocation.agentNames)}</span>
          <span className="truncate">task: {formatStringList(allocation.taskNames)}</span>
          <span className="truncate">conflicts: {formatStringList(allocation.conflictSessionIds)}</span>
        </div>
      </div>
    </button>
  );
}

function RuntimeProfileConfigCard({
  profile,
  allocation,
  onSelect,
  onEdit,
  selected,
}: {
  profile: ServiceProfileRecord;
  allocation?: ServiceProfileAllocation;
  onSelect: (allocation: ServiceProfileAllocation) => void;
  onEdit: (profile: ServiceProfileRecord) => void;
  selected: boolean;
}) {
  const profileId = serviceProfileId(profile, allocation?.profileId ?? "unknown");
  const readinessRows = serviceProfileReadiness(profile, allocation);
  const readinessAttention = profileReadinessNeedsAttention(readinessRows);
  const targets = serviceProfileTargets(profile, allocation);
  const accounts = serviceProfileAccounts(profile, allocation);
  const browserBuild = serviceProfileBrowserBuild(profile, allocation);
  const keyring = serviceProfileKeyring(profile, allocation);
  return (
    <article className={cn("service-runtime-profile-card", selected && "service-runtime-profile-card-selected")}>
      <div className="service-runtime-profile-card-header">
        <div className="min-w-0">
          <div className="flex min-w-0 items-center gap-2">
            <span className={cn("service-browser-health-dot", readinessAttention ? "service-browser-health-warn" : "service-browser-health-good")} />
            <h4>{profile.name || profileId}</h4>
          </div>
          <p>{profileId}</p>
        </div>
        <div className="service-runtime-profile-badges">
          <Badge variant="outline">{profile.allocation ?? allocation?.allocation ?? "managed"}</Badge>
          <Badge variant="outline">{profile.persistent === false ? "ephemeral" : "persistent"}</Badge>
          {readinessAttention && <Badge variant="outline" className="service-profile-attention-badge">readiness attention</Badge>}
        </div>
      </div>
      <div className="service-runtime-profile-grid">
        <span>
          <strong>User data</strong>
          <code>{profile.userDataDir ?? "runtime profile path not recorded"}</code>
        </span>
        <span>
          <strong>Browser build</strong>
          <code>{browserBuild ?? "service default"}</code>
        </span>
        <span>
          <strong>Host</strong>
          <code>{profile.defaultBrowserHost ?? "service default"}</code>
        </span>
        <span>
          <strong>Keyring</strong>
          <code>{keyring}</code>
        </span>
      </div>
      <div className="service-runtime-profile-token-row">
        <span>targets: {formatStringList(targets)}</span>
        <span>accounts: {formatStringList(accounts)}</span>
        <span>authenticated: {formatStringList(profile.authenticatedServiceIds ?? allocation?.authenticatedServiceIds)}</span>
        <span>shared: {formatStringList(profile.sharedServiceIds ?? allocation?.sharedServiceIds)}</span>
      </div>
      <div className="service-runtime-profile-token-row">
        <span>site policies: {formatStringList(profile.sitePolicyIds)}</span>
        <span>credential providers: {formatStringList(profile.credentialProviderIds)}</span>
        <span>tags: {formatStringList(profile.tags)}</span>
        <span>{profile.manualLoginPreferred ? "manual login preferred" : "automated login acceptable when policy allows"}</span>
      </div>
      {readinessRows.length > 0 && (
        <div className="service-runtime-readiness-list" aria-label={`Readiness for runtime profile ${profileId}`}>
          {readinessRows.slice(0, 3).map((row, index) => (
            <span key={`${profileId}-${row.targetServiceId ?? "target"}-${row.loginId ?? "login"}-${index}`}>
              <strong>{row.targetServiceId ?? "target"}</strong>
              <code>{row.state ?? "unknown"}</code>
              {row.loginId && <em>{row.loginId}</em>}
            </span>
          ))}
          {readinessRows.length > 3 && <span>{readinessRows.length - 3} more readiness rows</span>}
        </div>
      )}
      {allocation && (
        <div className="service-runtime-profile-actions">
          <Button size="sm" variant="outline" onClick={() => onEdit(profile)}>
            <Edit3 className="size-3.5" />
            Edit config
          </Button>
          <Button size="sm" variant="outline" onClick={() => onSelect(allocation)}>
            Inspect allocation
          </Button>
          <span>{allocation.leaseState ?? "unknown lease"} / {allocation.recommendedAction ?? "inspect"}</span>
        </div>
      )}
      {!allocation && (
        <div className="service-runtime-profile-actions">
          <Button size="sm" variant="outline" onClick={() => onEdit(profile)}>
            <Edit3 className="size-3.5" />
            Edit config
          </Button>
          <span>No allocation row is currently retained for this profile.</span>
        </div>
      )}
    </article>
  );
}

function RuntimeProfileConfigDialog({
  profile,
  allocation,
  saving,
  deleting,
  error,
  onSave,
  onDelete,
  onOpenChange,
}: {
  profile: ServiceProfileRecord | null;
  allocation?: ServiceProfileAllocation;
  saving: boolean;
  deleting: boolean;
  error: string;
  onSave: (profile: ServiceProfileRecord, form: RuntimeProfileConfigFormState) => Promise<void>;
  onDelete: (profile: ServiceProfileRecord) => Promise<void>;
  onOpenChange: (open: boolean) => void;
}) {
  const [form, setForm] = useState<RuntimeProfileConfigFormState | null>(null);

  useEffect(() => {
    if (!profile) {
      setForm(null);
      return;
    }
    setForm(runtimeProfileConfigFormState(profile, allocation));
  }, [allocation, profile]);

  const profileId = profile ? serviceProfileId(profile, allocation?.profileId ?? "") : "";
  const updateField = <K extends keyof RuntimeProfileConfigFormState>(
    key: K,
    value: RuntimeProfileConfigFormState[K],
  ) => {
    setForm((current) => current ? { ...current, [key]: value } : current);
  };

  return (
    <Dialog open={!!profile} onOpenChange={onOpenChange}>
      <DialogContent className="service-profile-config-dialog">
        {profile && form && (
          <>
            <DialogHeader>
              <DialogTitle className="pr-8 text-xl font-black tracking-[-0.04em]">
                Edit runtime profile config
              </DialogTitle>
              <DialogDescription>
                Persist service-managed profile routing and launch policy for {profileId || "this profile"}.
              </DialogDescription>
            </DialogHeader>
            <div className="service-profile-config-form">
              {error && (
                <div className="service-profile-config-error">
                  <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
                  <span>{error}</span>
                </div>
              )}
              <div className="service-profile-config-grid">
                <label>
                  <span>Profile ID</span>
                  <input value={profileId} readOnly />
                </label>
                <label>
                  <span>Name</span>
                  <input value={form.name} onChange={(event) => updateField("name", event.target.value)} />
                </label>
                <label className="service-profile-config-wide">
                  <span>User data dir</span>
                  <input value={form.userDataDir} onChange={(event) => updateField("userDataDir", event.target.value)} />
                </label>
                <label>
                  <span>Browser build</span>
                  <select value={form.browserBuild} onChange={(event) => updateField("browserBuild", event.target.value)}>
                    <option value="">Service default</option>
                    <option value="stock_chrome">stock_chrome</option>
                    <option value="stealthcdp_chromium">stealthcdp_chromium</option>
                    <option value="cdp_free_headed">cdp_free_headed</option>
                  </select>
                </label>
                <label>
                  <span>Default host</span>
                  <select value={form.defaultBrowserHost} onChange={(event) => updateField("defaultBrowserHost", event.target.value)}>
                    <option value="">Service default</option>
                    <option value="local_headless">local_headless</option>
                    <option value="local_headed">local_headed</option>
                    <option value="docker_headed">docker_headed</option>
                    <option value="remote_headed">remote_headed</option>
                    <option value="cloud_provider">cloud_provider</option>
                    <option value="attached_existing">attached_existing</option>
                  </select>
                </label>
                <label>
                  <span>Allocation</span>
                  <select value={form.allocation} onChange={(event) => updateField("allocation", event.target.value)}>
                    <option value="shared_service">shared_service</option>
                    <option value="per_service">per_service</option>
                    <option value="per_site">per_site</option>
                    <option value="per_identity">per_identity</option>
                    <option value="caller_supplied">caller_supplied</option>
                  </select>
                </label>
                <label>
                  <span>Keyring</span>
                  <select value={form.keyring} onChange={(event) => updateField("keyring", event.target.value)}>
                    <option value="basic_password_store">basic_password_store</option>
                    <option value="real_os_keychain">real_os_keychain</option>
                    <option value="managed_vault">managed_vault</option>
                    <option value="manual_login_profile">manual_login_profile</option>
                  </select>
                </label>
                <label>
                  <span>Target services</span>
                  <input value={form.targetServiceIds} onChange={(event) => updateField("targetServiceIds", event.target.value)} />
                </label>
                <label>
                  <span>Account IDs</span>
                  <input value={form.accountIds} onChange={(event) => updateField("accountIds", event.target.value)} />
                </label>
                <label>
                  <span>Authenticated services</span>
                  <input value={form.authenticatedServiceIds} onChange={(event) => updateField("authenticatedServiceIds", event.target.value)} />
                </label>
                <label>
                  <span>Shared services</span>
                  <input value={form.sharedServiceIds} onChange={(event) => updateField("sharedServiceIds", event.target.value)} />
                </label>
                <label>
                  <span>Site policies</span>
                  <input value={form.sitePolicyIds} onChange={(event) => updateField("sitePolicyIds", event.target.value)} />
                </label>
                <label>
                  <span>Credential providers</span>
                  <input value={form.credentialProviderIds} onChange={(event) => updateField("credentialProviderIds", event.target.value)} />
                </label>
                <label className="service-profile-config-wide">
                  <span>Tags</span>
                  <input value={form.tags} onChange={(event) => updateField("tags", event.target.value)} />
                </label>
              </div>
              <div className="service-profile-config-checks">
                <label>
                  <input
                    type="checkbox"
                    checked={form.manualLoginPreferred}
                    onChange={(event) => updateField("manualLoginPreferred", event.target.checked)}
                  />
                  <span>Prefer manual login for this profile</span>
                </label>
                <label>
                  <input
                    type="checkbox"
                    checked={form.persistent}
                    onChange={(event) => updateField("persistent", event.target.checked)}
                  />
                  <span>Persistent service profile</span>
                </label>
              </div>
              <p className="service-profile-config-help">
                List fields are comma-separated. Freshness and seeding readiness rows are preserved; use the seeding or freshness actions to change login evidence.
              </p>
              <div className="service-profile-config-actions">
                <AlertDialog>
                  <AlertDialogTrigger asChild>
                    <Button variant="outline" disabled={saving || deleting}>
                      <Trash2 className="size-3.5" />
                      Delete
                    </Button>
                  </AlertDialogTrigger>
                  <AlertDialogContent>
                    <AlertDialogHeader>
                      <AlertDialogTitle>Delete runtime profile config?</AlertDialogTitle>
                      <AlertDialogDescription>
                        This removes the persisted service profile record for {profileId}. It does not delete the browser user-data directory.
                      </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                      <AlertDialogCancel disabled={deleting}>Cancel</AlertDialogCancel>
                      <AlertDialogAction
                        className="bg-destructive/10 text-destructive hover:bg-destructive/20 focus-visible:border-destructive/40 focus-visible:ring-destructive/20"
                        disabled={deleting}
                        onClick={() => onDelete(profile)}
                      >
                        {deleting ? <Loader2 className="size-3.5 animate-spin" /> : <Trash2 className="size-3.5" />}
                        Delete config
                      </AlertDialogAction>
                    </AlertDialogFooter>
                  </AlertDialogContent>
                </AlertDialog>
                <Button disabled={saving || deleting} onClick={() => onSave(profile, form)}>
                  {saving ? <Loader2 className="size-3.5 animate-spin" /> : null}
                  Save config
                </Button>
              </div>
            </div>
          </>
        )}
      </DialogContent>
    </Dialog>
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
        <EventDetailItem label="Browser build" value={allocation.browserBuild} />
        <EventDetailItem label="Primary target" value={profileAllocationPrimaryTarget(allocation)} />
        <EventDetailItem label="Primary login" value={profileAllocationPrimaryLogin(allocation)} />
        <EventDetailItem label="Primary browser" value={profileAllocationPrimaryBrowser(allocation)} />
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
      <ProfileAllocationTokenSection title="Account identities" values={allocation.accountIds} />
      <ProfileAllocationTokenSection title="Target services" values={allocation.targetServiceIds} />
      <ProfileAllocationTokenSection title="Authenticated services" values={allocation.authenticatedServiceIds} />
      <ProfileReadinessSection rows={allocation.targetReadiness} />
      <ProfileAllocationTokenSection title="Shared services" values={allocation.sharedServiceIds} />
      <ProfileBrowserSummarySection rows={allocation.browserSummaries} />
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

function ProfileBrowserSummarySection({ rows }: { rows?: ServiceProfileAllocationBrowserSummary[] }) {
  const items = rows?.filter((row) => row.browserId?.trim()) ?? [];
  if (items.length === 0) return null;
  return (
    <div>
      <p className="mb-2 text-[10px] font-black uppercase tracking-[0.18em] text-muted-foreground">
        Browser summaries
      </p>
      <div className="flex flex-col gap-2">
        {items.map((row, index) => (
          <div
            key={`${row.browserId}-${index}`}
            className="rounded-2xl border border-border/70 bg-foreground/[0.03] px-3 py-2 text-xs"
          >
            <div className="font-black text-foreground">
              {row.browserId}
            </div>
            <div className="mt-1 text-muted-foreground">
              {row.host ?? "unknown host"} / {formatHealthLabel(row.health)} / {row.pid ? `pid ${row.pid}` : "retained"}
            </div>
            <div className="mt-1 text-muted-foreground">
              {row.hasCdpEndpoint ? "CDP endpoint known" : "no CDP endpoint"} / {row.activeSessionIds?.length ?? 0} active sessions
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
          Control
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
          Open remote control
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
  onShowTrace,
  acting,
}: {
  incident: IncidentRecord | null;
  activity: ServiceTraceTimelineItem[] | null;
  activityLoading: boolean;
  activityError: string;
  onOpenChange: (open: boolean) => void;
  onAcknowledge: (incident: IncidentRecord, note: string) => void;
  onResolve: (incident: IncidentRecord, note: string) => void;
  onShowTrace: (incident: IncidentRecord) => void;
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
                <Button
                  type="button"
                  variant="outline"
                  className="rounded-full"
                  onClick={() => onShowTrace(incident)}
                >
                  <History className="size-3.5" />
                  Show related trace
                </Button>
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
  onShowTrace,
}: {
  incident: IncidentRecord;
  acting?: boolean;
  onAcknowledge?: (incident: IncidentRecord, note: string) => void;
  onResolve?: (incident: IncidentRecord, note: string) => void;
  onShowTrace?: (incident: IncidentRecord) => void;
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
      {(actionsAvailable || onShowTrace) && (
        <div className="service-incident-actions">
          {onShowTrace && (
            <Button
              type="button"
              variant="outline"
              className="rounded-full"
              onClick={() => onShowTrace(incident)}
            >
              <History className="size-3.5" />
              Show related trace
            </Button>
          )}
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
  const [contracts, setContracts] = useState<ServiceContractsData | null>(null);
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
  const [profileTargetFilter, setProfileTargetFilter] = useState("all");
  const [profileLoginFilter, setProfileLoginFilter] = useState("all");
  const [profileBrowserBuildFilter, setProfileBrowserBuildFilter] = useState("all");
  const [profileReadinessFilter, setProfileReadinessFilter] = useState<ProfileReadinessFilter>("all");
  const [incidentQuery, setIncidentQuery] = useState("");
  const [incidentLimit, setIncidentLimit] = useState<ServiceRecordLimit>(24);
  const [workspaceTab, setWorkspaceTab] = useState<ServiceWorkspaceTab>("incidents");
  const [cleanupLoading, setCleanupLoading] = useState<RetainedCleanupKind | null>(null);
  const [cleanupApplying, setCleanupApplying] = useState<RetainedCleanupKind | null>(null);
  const [cleanupKind, setCleanupKind] = useState<RetainedCleanupKind | null>(null);
  const [cleanupResult, setCleanupResult] = useState<RetainedCleanupResult | null>(null);
  const [cleanupError, setCleanupError] = useState("");
  const [managedAttentionOpen, setManagedAttentionOpen] = useState(false);
  const [incidentOnly, setIncidentOnly] = useState(false);
  const [incidentHandlingFilter, setIncidentHandlingFilter] = useState<IncidentHandlingFilter>("all");
  const [jobQuery, setJobQuery] = useState("");
  const [jobDisplayFilter, setJobDisplayFilter] = useState<ServiceJobDisplayFilter>("all");
  const [jobStateFilter, setJobStateFilter] = useState("all");
  const [jobSortKey, setJobSortKey] = useState<ServiceJobSortKey>("submittedAt");
  const [jobSortDirection, setJobSortDirection] = useState<SortDirection>("desc");
  const [jobLimit, setJobLimit] = useState<ServiceRecordLimit>(24);
  const [jobFilterNotice, setJobFilterNotice] = useState("");
  const [incidentFilterNotice, setIncidentFilterNotice] = useState("");
  const [actingIncidentId, setActingIncidentId] = useState<string | null>(null);
  const operatorIdentity = "default";
  const [selectedIncident, setSelectedIncident] = useState<IncidentRecord | null>(null);
  const [selectedIncidentActivity, setSelectedIncidentActivity] = useState<ServiceTraceTimelineItem[] | null>(null);
  const [selectedIncidentActivityLoading, setSelectedIncidentActivityLoading] = useState(false);
  const [selectedIncidentActivityError, setSelectedIncidentActivityError] = useState("");
  const [selectedEvent, setSelectedEvent] = useState<ServiceEvent | null>(null);
  const [selectedBrowser, setSelectedBrowser] = useState<ServiceBrowser | null>(null);
  const [selectedBrowserId, setSelectedBrowserId] = useState<string | null>(null);
  const [selectedProfileAllocationId, setSelectedProfileAllocationId] = useState<string | null>(null);
  const [selectedSession, setSelectedSession] = useState<ServiceSession | null>(null);
  const [selectedTab, setSelectedTab] = useState<ServiceTab | null>(null);
  const [selectedViewStream, setSelectedViewStream] = useState<SelectedViewStream | null>(null);
  const [viewStreamFullscreen, setViewStreamFullscreen] = useState(false);
  const [selectedJob, setSelectedJob] = useState<ServiceJob | null>(null);
  const [selectedProfileAllocation, setSelectedProfileAllocation] = useState<ServiceProfileAllocation | null>(null);
  const [selectedProfileAllocationLoading, setSelectedProfileAllocationLoading] = useState(false);
  const [selectedProfileAllocationError, setSelectedProfileAllocationError] = useState("");
  const [selectedProfileConfig, setSelectedProfileConfig] = useState<ServiceProfileRecord | null>(null);
  const [profileConfigSaving, setProfileConfigSaving] = useState(false);
  const [profileConfigDeleting, setProfileConfigDeleting] = useState(false);
  const [profileConfigError, setProfileConfigError] = useState("");
  const [actingBrowserActionId, setActingBrowserActionId] = useState<string | null>(null);
  const profileAllocationLookupId = useRef(0);
  const profileAllocationRowRefs = useRef(new Map<string, HTMLButtonElement>());

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
      const contractsPromise = fetch(`${serviceBase(activePort)}/contracts`).catch(() => null);
      const [statusResp, jobsResp, eventsResp, incidentsResp, contractsResp] = await Promise.all([
        fetch(`${serviceBase(activePort)}/status`),
        fetch(`${serviceBase(activePort)}/jobs?limit=${jobLimit}`),
        fetch(`${serviceBase(activePort)}/events?${params.toString()}`),
        fetch(`${serviceBase(activePort)}/incidents?summary=true&limit=50`),
        contractsPromise,
      ]);
      const statusJson = (await statusResp.json()) as ApiResponse<ServiceStatusData>;
      const jobsJson = (await jobsResp.json()) as ApiResponse<ServiceJobsData>;
      const eventsJson = (await eventsResp.json()) as ApiResponse<ServiceEventsData>;
      const incidentsJson = (await incidentsResp.json()) as ApiResponse<ServiceIncidentsData>;
      const contractsJson = contractsResp?.ok
        ? ((await contractsResp.json()) as ApiResponse<ServiceContractsData>)
        : null;
      if (!statusJson.success) throw new Error(statusJson.error || "Service status failed");
      if (!jobsJson.success) throw new Error(jobsJson.error || "Service jobs failed");
      if (!eventsJson.success) throw new Error(eventsJson.error || "Service events failed");
      setStatus(statusJson.data ?? null);
      setJobs(jobsJson.data ?? null);
      setEvents(eventsJson.data ?? null);
      setIncidents(incidentsJson.success ? incidentsJson.data ?? null : null);
      setContracts(contractsJson?.success ? contractsJson.data ?? null : null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Service API unavailable");
    } finally {
      if (showSpinner) setLoading(false);
    }
  }, [activePort, canFetch, eventBrowserId, eventKind, eventLimit, eventWindow, jobLimit]);

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

  const loadTraceForFilters = useCallback(async (filters: TraceFilters) => {
    if (!canFetch || traceLoading) return;
    setTraceLoading(true);
    setTraceError("");
    try {
      const params = traceQueryParams(filters);
      const resp = await fetch(`${serviceBase(activePort)}/trace?${params.toString()}`);
      const json = (await resp.json()) as ApiResponse<ServiceTraceData | ServiceTraceToolPayload>;
      if (!json.success) throw new Error(json.error || "Service trace failed");
      setTrace(normalizeServiceTraceData(json.data));
    } catch (err) {
      setTraceError(err instanceof Error ? err.message : "Service trace unavailable");
    } finally {
      setTraceLoading(false);
    }
  }, [activePort, canFetch, traceLoading]);

  const loadTrace = useCallback(async () => {
    await loadTraceForFilters(traceFilters);
  }, [loadTraceForFilters, traceFilters]);

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
    setSelectedProfileAllocationId(allocation.profileId || null);
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

  const editRuntimeProfileConfig = useCallback((profile: ServiceProfileRecord) => {
    setProfileConfigError("");
    setSelectedProfileConfig(profile);
  }, []);

  const saveRuntimeProfileConfig = useCallback(async (
    profile: ServiceProfileRecord,
    form: RuntimeProfileConfigFormState,
  ) => {
    const profileId = serviceProfileId(profile);
    if (!canFetch || !profileId) return;
    setProfileConfigSaving(true);
    setProfileConfigError("");
    try {
      const resp = await fetch(`${serviceBase(activePort)}/profiles/${encodeURIComponent(profileId)}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(runtimeProfileConfigPayload(profile, form)),
      });
      const json = (await resp.json()) as ApiResponse<{ profile?: ServiceProfileRecord }>;
      if (!json.success) throw new Error(json.error || "Service profile save failed");
      setSelectedProfileConfig(null);
      await fetchService(false);
    } catch (err) {
      setProfileConfigError(err instanceof Error ? err.message : "Service profile save unavailable");
    } finally {
      setProfileConfigSaving(false);
    }
  }, [activePort, canFetch, fetchService]);

  const deleteRuntimeProfileConfig = useCallback(async (profile: ServiceProfileRecord) => {
    const profileId = serviceProfileId(profile);
    if (!canFetch || !profileId) return;
    setProfileConfigDeleting(true);
    setProfileConfigError("");
    try {
      const resp = await fetch(`${serviceBase(activePort)}/profiles/${encodeURIComponent(profileId)}`, {
        method: "DELETE",
      });
      const json = (await resp.json()) as ApiResponse<{ deleted?: boolean; profile?: ServiceProfileRecord }>;
      if (!json.success) throw new Error(json.error || "Service profile delete failed");
      setSelectedProfileConfig(null);
      if (selectedProfileAllocationId === profileId) setSelectedProfileAllocationId(null);
      await fetchService(false);
    } catch (err) {
      setProfileConfigError(err instanceof Error ? err.message : "Service profile delete unavailable");
    } finally {
      setProfileConfigDeleting(false);
    }
  }, [activePort, canFetch, fetchService, selectedProfileAllocationId]);

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

  const serviceState = status?.service_state;
  const serviceRequestActions = useMemo(
    () => new Set(contracts?.contracts?.serviceRequest?.actions ?? []),
    [contracts?.contracts?.serviceRequest?.actions],
  );
  const browserCloseSupported = serviceRequestActions.has("service_browser_close");
  const browserRepairSupported = serviceRequestActions.has("service_browser_repair");
  const control = status?.control_plane;
  const serviceJobTimeoutMs =
    control?.service_job_timeout_ms ?? serviceState?.controlPlane?.serviceJobTimeoutMs ?? null;
  const reconciliation = serviceState?.reconciliation;
  const retainedServiceJobs = useMemo(
    () => Object.values(serviceState?.jobs ?? {}),
    [serviceState?.jobs],
  );
  const recentJobs = jobs?.jobs ?? retainedServiceJobs.slice(-8);
  const showJobsForDisplayAllocation = useCallback((displayIsolation: string | null, jobIds: string[] = []) => {
    const displayFilter = serviceJobDisplayFilterForTraceAllocation(displayIsolation);
    const label = displayIsolationLabel(displayIsolation);
    setWorkspaceTab("jobs");
    setJobDisplayFilter(displayFilter);
    setJobStateFilter("all");
    setJobSortKey("displayIsolation");
    setJobSortDirection("asc");
    setJobLimit((current) => (current < 100 ? 100 : current));
    setJobQuery(displayFilter === "all" && displayIsolation ? displayIsolation : "");

    if (jobIds.length > 0) {
      const retainedJobIds = new Set(recentJobs.map((job) => job.id));
      const missingJobCount = jobIds.filter((jobId) => !retainedJobIds.has(jobId)).length;
      if (missingJobCount > 0) {
        setJobFilterNotice(
          `Showing ${label} jobs from the retained Jobs window. ${missingJobCount} trace job${missingJobCount === 1 ? "" : "s"} may appear after the 100-row refresh completes.`,
        );
        return;
      }
    }

    setJobFilterNotice(`Showing ${label} jobs from the trace display allocation summary.`);
  }, [recentJobs]);
  const showTraceJob = useCallback((jobId: string) => {
    setWorkspaceTab("jobs");
    setJobQuery(jobId);
    setJobStateFilter("all");
    setJobDisplayFilter("all");
    setJobSortKey("submittedAt");
    setJobSortDirection("desc");
    setJobLimit((current) => (current < 100 ? 100 : current));

    const retainedJob = recentJobs.find((job) => job.id === jobId);
    setJobFilterNotice(
      retainedJob
        ? `Showing trace job ${jobId} from the retained Jobs window.`
        : `Trace job ${jobId} is outside the retained Jobs window; it may appear after the 100-row refresh completes.`,
    );
  }, [recentJobs]);
  const filteredJobs = useMemo(() => {
    const query = jobQuery.trim().toLowerCase();
    const rows = recentJobs.filter((job) => {
      if (jobStateFilter !== "all" && job.state !== jobStateFilter) return false;
      if (!serviceJobDisplayMatchesFilter(job, jobDisplayFilter)) return false;
      return query ? serviceJobSearchText(job).includes(query) : true;
    });
    rows.sort((left, right) => {
      const order = compareBrowserValues(
        serviceJobSortValue(left, jobSortKey),
        serviceJobSortValue(right, jobSortKey),
      );
      return jobSortDirection === "asc" ? order : -order;
    });
    return rows;
  }, [jobDisplayFilter, jobQuery, jobSortDirection, jobSortKey, jobStateFilter, recentJobs]);
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
  const jobDisplaySummary = useMemo(() => {
    const counts = recentJobs.reduce(
      (summary, job) => {
        const key = job.displayIsolation ?? "unrecorded";
        summary[key] = (summary[key] ?? 0) + 1;
        return summary;
      },
      {} as Record<string, number>,
    );
    const parts = SERVICE_JOB_DISPLAY_FILTER_OPTIONS
      .filter((option) => option.value !== "all")
      .map((option) => {
        const count = counts[option.value] ?? 0;
        return count > 0 ? `${option.label}: ${count}` : null;
      })
      .filter((value): value is string => Boolean(value));
    return parts.length > 0 ? parts.join(" / ") : "No display allocation requests in this window";
  }, [recentJobs]);
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
  const showTraceIncident = useCallback((incidentId: string) => {
    setWorkspaceTab("incidents");
    setIncidentQuery(incidentId);
    setIncidentHandlingFilter("all");
    setIncidentLimit((current) => (current < 100 ? 100 : current));

    const retainedIncident = incidentRecords.find((incident) => incident.id === incidentId);
    if (retainedIncident) {
      inspectIncident(retainedIncident);
    }
    setIncidentFilterNotice(
      retainedIncident
        ? `Showing trace incident ${incidentId} from the retained Incidents window.`
        : `Trace incident ${incidentId} is outside the retained Incidents window; it may appear after the 100-row refresh completes.`,
    );
  }, [incidentRecords, inspectIncident]);
  const showIncidentTrace = useCallback((incident: IncidentRecord) => {
    const filters = incidentTraceFilters(incident);
    setWorkspaceTab("events");
    setTraceFilters(filters);
    void loadTraceForFilters(filters);
  }, [loadTraceForFilters]);
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
  const profileAllocationById = useMemo(() => {
    const byId = new Map<string, ServiceProfileAllocation>();
    for (const allocation of profileAllocations) {
      if (allocation.profileId) byId.set(allocation.profileId, allocation);
    }
    return byId;
  }, [profileAllocations]);
  const selectedProfileConfigAllocation = selectedProfileConfig
    ? profileAllocationById.get(serviceProfileId(selectedProfileConfig))
    : undefined;
  const profileRecords = useMemo(
    () => Object.entries(serviceState?.profiles ?? {}).map(([id, profile]) => ({ ...profile, id: profile.id ?? id })),
    [serviceState?.profiles],
  );
  const profileTargetOptions = useMemo(
    () => uniqueStringValues([
      ...profileAllocations.flatMap(profileAllocationTargetValues),
      ...profileRecords.flatMap((profile) => serviceProfileTargets(profile, profileAllocationById.get(serviceProfileId(profile)))),
    ]),
    [profileAllocationById, profileAllocations, profileRecords],
  );
  const profileLoginOptions = useMemo(
    () => uniqueStringValues([
      ...profileAllocations.flatMap(profileAllocationLoginValues),
      ...profileRecords.flatMap((profile) => serviceProfileAccounts(profile, profileAllocationById.get(serviceProfileId(profile)))),
    ]),
    [profileAllocationById, profileAllocations, profileRecords],
  );
  const profileBrowserBuildOptions = useMemo(
    () => uniqueStringValues([
      ...profileAllocations.map((allocation) => allocation.browserBuild),
      ...profileRecords.map((profile) => profile.browserBuild),
    ]),
    [profileAllocations, profileRecords],
  );
  const profileAllocationQueryText = profileAllocationQuery.trim().toLowerCase();
  const filteredProfileRecords = useMemo(() => {
    const fieldFiltered = profileRecords.filter((profile) => {
      const allocation = profileAllocationById.get(serviceProfileId(profile));
      if (profileTargetFilter !== "all" && !serviceProfileTargets(profile, allocation).includes(profileTargetFilter)) return false;
      if (profileLoginFilter !== "all" && !serviceProfileAccounts(profile, allocation).includes(profileLoginFilter)) return false;
      if (profileBrowserBuildFilter !== "all" && serviceProfileBrowserBuild(profile, allocation) !== profileBrowserBuildFilter) return false;
      const needsAttention = profileReadinessNeedsAttention(serviceProfileReadiness(profile, allocation));
      if (profileReadinessFilter === "needs_attention" && !needsAttention) return false;
      if (profileReadinessFilter === "normal" && needsAttention) return false;
      return true;
    });
    return profileAllocationQueryText
      ? fieldFiltered.filter((profile) => {
        const allocation = profileAllocationById.get(serviceProfileId(profile));
        return includesQuery(serviceProfileSearchText(profile, allocation), profileAllocationQueryText);
      })
      : fieldFiltered;
  }, [
    profileAllocationById,
    profileAllocationQueryText,
    profileBrowserBuildFilter,
    profileLoginFilter,
    profileReadinessFilter,
    profileRecords,
    profileTargetFilter,
  ]);
  const filteredProfileAllocations = useMemo(() => {
    const fieldFiltered = profileAllocations.filter((allocation) => {
      if (profileTargetFilter !== "all" && !profileAllocationTargetValues(allocation).includes(profileTargetFilter)) return false;
      if (profileLoginFilter !== "all" && !profileAllocationLoginValues(allocation).includes(profileLoginFilter)) return false;
      if (profileBrowserBuildFilter !== "all" && allocation.browserBuild !== profileBrowserBuildFilter) return false;
      const needsAttention = profileReadinessNeedsAttention(allocation.targetReadiness);
      if (profileReadinessFilter === "needs_attention" && !needsAttention) return false;
      if (profileReadinessFilter === "normal" && needsAttention) return false;
      return true;
    });
    return profileAllocationQueryText
      ? fieldFiltered.filter((allocation) => includesQuery(profileAllocationSearchText(allocation), profileAllocationQueryText))
      : fieldFiltered;
  }, [
    profileAllocationQueryText,
    profileAllocations,
    profileBrowserBuildFilter,
    profileLoginFilter,
    profileReadinessFilter,
    profileTargetFilter,
  ]);
  const visibleProfileAllocations = useMemo(
    () => filteredProfileAllocations.slice(0, profileAllocationLimit),
    [filteredProfileAllocations, profileAllocationLimit],
  );
  const visibleProfileRecords = useMemo(
    () => filteredProfileRecords.slice(0, profileAllocationLimit),
    [filteredProfileRecords, profileAllocationLimit],
  );
  const hiddenProfileRecordCount = Math.max(0, filteredProfileRecords.length - visibleProfileRecords.length);
  const hiddenProfileAllocationCount = Math.max(0, filteredProfileAllocations.length - visibleProfileAllocations.length);
  const setProfileAllocationRowRef = (profileId: string, node: HTMLButtonElement | null) => {
    if (node) {
      profileAllocationRowRefs.current.set(profileId, node);
      return;
    }
    profileAllocationRowRefs.current.delete(profileId);
  };
  const focusProfileAllocationRow = useCallback((index: number) => {
    const nextAllocation = visibleProfileAllocations[index];
    if (!nextAllocation?.profileId) return;
    setSelectedProfileAllocationId(nextAllocation.profileId);
    void inspectProfileAllocation(nextAllocation);
    profileAllocationRowRefs.current.get(nextAllocation.profileId)?.focus();
  }, [inspectProfileAllocation, visibleProfileAllocations]);
  const navigateProfileAllocationRows = useCallback((allocation: ServiceProfileAllocation, event: ReactKeyboardEvent<HTMLButtonElement>) => {
    if (!allocation.profileId) return;
    const currentIndex = visibleProfileAllocations.findIndex((row) => row.profileId === allocation.profileId);
    if (currentIndex < 0) return;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      focusProfileAllocationRow(Math.min(visibleProfileAllocations.length - 1, currentIndex + 1));
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      focusProfileAllocationRow(Math.max(0, currentIndex - 1));
      return;
    }
    if (event.key === "Home") {
      event.preventDefault();
      focusProfileAllocationRow(0);
      return;
    }
    if (event.key === "End") {
      event.preventDefault();
      focusProfileAllocationRow(visibleProfileAllocations.length - 1);
    }
  }, [focusProfileAllocationRow, visibleProfileAllocations]);
  const profileRoutingSummary = useMemo(() => {
    const targets = new Set<string>();
    const accounts = new Set<string>();
    let authenticatedTargets = 0;
    let readinessAttention = 0;
    let explicitBrowserBuilds = 0;
    let profilesWithBrowsers = 0;
    for (const profile of profileRecords) {
      const allocation = profileAllocationById.get(serviceProfileId(profile));
      for (const target of serviceProfileTargets(profile, allocation)) {
        if (target.trim()) targets.add(target);
      }
      for (const account of serviceProfileAccounts(profile, allocation)) {
        if (account.trim()) accounts.add(account);
      }
      authenticatedTargets += profile.authenticatedServiceIds?.length ?? allocation?.authenticatedServiceIds?.length ?? 0;
      if (profileReadinessNeedsAttention(serviceProfileReadiness(profile, allocation))) readinessAttention += 1;
      if (serviceProfileBrowserBuild(profile, allocation)?.trim()) explicitBrowserBuilds += 1;
    }
    for (const allocation of profileAllocations) {
      for (const target of allocation.targetServiceIds ?? []) {
        if (target.trim()) targets.add(target);
      }
      for (const row of allocation.targetReadiness ?? []) {
        if (row.targetServiceId?.trim()) targets.add(row.targetServiceId);
        if (row.loginId?.trim()) accounts.add(row.loginId);
      }
      for (const account of allocation.accountIds ?? []) {
        if (account.trim()) accounts.add(account);
      }
      authenticatedTargets += allocation.authenticatedServiceIds?.length ?? 0;
      if (profileReadinessNeedsAttention(allocation.targetReadiness)) readinessAttention += 1;
      if (allocation.browserBuild?.trim()) explicitBrowserBuilds += 1;
      if ((allocation.browserSummaries?.length ?? allocation.browserIds?.length ?? 0) > 0) profilesWithBrowsers += 1;
    }
    return {
      profiles: profileRecords.length,
      targets: targets.size,
      accounts: accounts.size,
      authenticatedTargets,
      readinessAttention,
      explicitBrowserBuilds,
      profilesWithBrowsers,
    };
  }, [profileAllocationById, profileAllocations, profileRecords]);
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
  const browserTabsById = useMemo(() => {
    const grouped = new Map<string, ServiceTab[]>();
    for (const tab of tabRecords) {
      if (!tab.browserId) continue;
      grouped.set(tab.browserId, [...(grouped.get(tab.browserId) ?? []), tab]);
    }
    return grouped;
  }, [tabRecords]);
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
    const viewStreamAvailable = tab.browserId ? canOpenViewStream(browserPrimaryViewStream(browserById.get(tab.browserId))) : false;
    if (onInspectSelection) {
      onInspectSelection({ kind: "tab", tab, viewStreamAvailable });
      return;
    }
    setSelectedTab(tab);
  }, [browserById, onInspectSelection]);
  const openBrowserViewStream = useCallback((browser: ServiceBrowser) => {
    const stream = browserPrimaryViewStream(browser);
    if (!stream) {
      setError("No view stream is registered for this browser.");
      return;
    }
    if (!canEmbedViewStream(stream)) {
      setError(viewStreamOpenTitle(stream));
      return;
    }
    openViewStream(stream, browser);
  }, [openViewStream]);
  const focusBrowserViewStream = useCallback(async (browser: ServiceBrowser) => {
    const stream = browserPrimaryViewStream(browser);
    if (!stream) {
      setError("No view stream is registered for this browser.");
      return;
    }
    if (!canOpenControlViewStream(stream)) {
      setError(viewStreamControlTitle(stream));
      return;
    }

    let focusMessage: string | null = null;
    const browserTabs = browser.id ? browserTabsById.get(browser.id) : null;
    const primaryTab = browserTabs?.find((tab) => isActiveServiceTab(tab)) ?? browserTabs?.[0] ?? null;
    const tabIndex = primaryTab?.id ? tabIndexById.get(primaryTab.id) : undefined;
    if (canFetch && tabIndex !== undefined) {
      try {
        const resp = await fetch(`${serviceBase(activePort)}/request`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            action: "view_focus",
            serviceName: "agent-browser-dashboard",
            agentName: operatorIdentity.trim() || activeSession || "operator",
            taskName: "focus-browser-row-view",
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

    openViewStream(stream, browser, primaryTab, focusMessage);
  }, [activePort, activeSession, browserTabsById, canFetch, openViewStream, operatorIdentity, tabIndexById]);

  useEffect(() => {
    if (!onInspectorActionsChange) return;
    onInspectorActionsChange({
      actingIncidentId,
      onControlBrowser: focusBrowserViewStream,
      onAcknowledgeIncident: acknowledgeInspectorIncident,
      onResolveIncident: resolveInspectorIncident,
      onShowIncidentTrace: showIncidentTrace,
      onCancelJob: cancelInspectorJob,
    });
  }, [
    acknowledgeInspectorIncident,
    actingIncidentId,
    cancelInspectorJob,
    focusBrowserViewStream,
    onInspectorActionsChange,
    resolveInspectorIncident,
    showIncidentTrace,
  ]);

  const closeServiceBrowser = useCallback(async (browser: ServiceBrowser) => {
    if (!canFetch || !browser.id) return;
    setActingBrowserActionId(browser.id);
    setError("");
    try {
      const resp = await fetch(`${serviceBase(activePort)}/request`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "service_browser_close",
          serviceName: "agent-browser-dashboard",
          agentName: operatorIdentity.trim() || activeSession || "operator",
          taskName: "close-browser-row",
          params: { browserId: browser.id },
          jobTimeoutMs: 10000,
        }),
      });
      const json = (await resp.json()) as ApiResponse<ServiceBrowserCloseData>;
      if (!json.success) throw new Error(json.error || "Service browser close request failed");
      await fetchService(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Service browser close request failed");
    } finally {
      setActingBrowserActionId(null);
    }
  }, [activePort, activeSession, canFetch, fetchService, operatorIdentity]);
  const repairServiceBrowser = useCallback(async (browser: ServiceBrowser) => {
    if (!canFetch || !browser.id) return;
    setActingBrowserActionId(browser.id);
    setError("");
    try {
      const resp = await fetch(`${serviceBase(activePort)}/request`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "service_browser_repair",
          serviceName: "agent-browser-dashboard",
          agentName: operatorIdentity.trim() || activeSession || "operator",
          taskName: "repair-browser-row",
          params: {
            browserId: browser.id,
            by: operatorIdentity.trim() || activeSession || "operator",
            note: "Dashboard row repair requested",
          },
          jobTimeoutMs: 10000,
        }),
      });
      const json = (await resp.json()) as ApiResponse<ServiceBrowserRepairData>;
      if (!json.success) throw new Error(json.error || "Service browser repair request failed");
      await fetchService(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Service browser repair request failed");
    } finally {
      setActingBrowserActionId(null);
    }
  }, [activePort, activeSession, canFetch, fetchService, operatorIdentity]);
  const inspectTabViewStream = useCallback(async (tab: ServiceTab) => {
    const browser = tab.browserId ? browserById.get(tab.browserId) : null;
    const stream = browserPrimaryViewStream(browser);
    if (!browser || !stream) {
      setError("No view stream is registered for this tab's browser.");
      return;
    }
    if (!canOpenControlViewStream(stream)) {
      setError(viewStreamControlTitle(stream));
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
  const toggleJobSort = useCallback((nextSortKey: ServiceJobSortKey) => {
    if (nextSortKey === jobSortKey) {
      setJobSortDirection((current) => current === "asc" ? "desc" : "asc");
      return;
    }
    setJobSortKey(nextSortKey);
    setJobSortDirection(nextSortKey === "submittedAt" ? "desc" : "asc");
  }, [jobSortKey]);
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
  const managedAttentionCount = [
    reconciliation?.lastError,
    retainedStateCleanupNeeded,
  ].filter(Boolean).length;
  const managedAttentionExpanded =
    managedAttentionOpen ||
    Boolean(cleanupLoading || cleanupApplying || cleanupError || cleanupResult);
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
        onShowTrace={(incident) => {
          showIncidentTrace(incident);
          setSelectedIncident(null);
        }}
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
        viewStreamAvailable={selectedTab?.browserId ? canOpenControlViewStream(browserPrimaryViewStream(browserById.get(selectedTab.browserId))) : false}
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
      <RuntimeProfileConfigDialog
        profile={selectedProfileConfig}
        allocation={selectedProfileConfigAllocation}
        saving={profileConfigSaving}
        deleting={profileConfigDeleting}
        error={profileConfigError}
        onSave={saveRuntimeProfileConfig}
        onDelete={deleteRuntimeProfileConfig}
        onOpenChange={(open) => {
          if (!open) {
            setSelectedProfileConfig(null);
            setProfileConfigError("");
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

          {managedAttentionCount > 0 && (
            <div className="service-attention-rail" aria-label="Managed state attention items">
              <button
                type="button"
                className="service-attention-toggle"
                aria-expanded={managedAttentionExpanded}
                aria-label={`${managedAttentionCount} managed-state item${managedAttentionCount === 1 ? "" : "s"} need review`}
                onClick={() => setManagedAttentionOpen((open) => !open)}
              >
                <span className="service-attention-icon">
                  <AlertTriangle className="size-3.5" />
                </span>
                <span className="min-w-0 flex-1">
                  <span className="service-attention-kicker">Attention</span>
                  <span className="service-attention-summary">
                    {managedAttentionCount} managed-state item{managedAttentionCount === 1 ? "" : "s"} need review
                  </span>
                </span>
                <ChevronDown
                  className={cn("size-3.5 shrink-0 transition-transform", managedAttentionExpanded && "rotate-180")}
                  aria-hidden="true"
                />
              </button>
              {managedAttentionExpanded && (
                <div className="service-state-alerts">
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
                <BrowserTable
                  browsers={browserRecords}
                  sessions={sessionRecords}
                  onSelect={inspectBrowser}
                  onViewStream={openBrowserViewStream}
                  onFocusViewStream={focusBrowserViewStream}
                  onCloseBrowser={closeServiceBrowser}
                  onRepairBrowser={repairServiceBrowser}
                  closeSupported={browserCloseSupported}
                  repairSupported={browserRepairSupported}
                  activeSessionName={activeSession}
                  actingBrowserActionId={actingBrowserActionId}
                  selectedBrowserId={selectedBrowserId}
                />
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
                <span>{filteredProfileRecords.length} of {profileRecords.length} runtime profile configs; {filteredProfileAllocations.length} allocation rows</span>
              </div>
              <div className="service-profile-routing-strip" aria-label="Profile identity and routing summary">
                <span>
                  <strong>{profileRoutingSummary.profiles}</strong>
                  runtime profiles
                </span>
                <span>
                  <strong>{profileRoutingSummary.targets}</strong>
                  target identities
                </span>
                <span>
                  <strong>{profileRoutingSummary.accounts}</strong>
                  login identities
                </span>
                <span>
                  <strong>{profileRoutingSummary.authenticatedTargets}</strong>
                  authenticated targets
                </span>
                <span>
                  <strong>{profileRoutingSummary.profilesWithBrowsers}</strong>
                  profiles with browsers
                </span>
                <span>
                  <strong>{profileRoutingSummary.explicitBrowserBuilds}</strong>
                  pinned builds
                </span>
                <span className={profileRoutingSummary.readinessAttention > 0 ? "service-profile-routing-strip-attention" : undefined}>
                  <strong>{profileRoutingSummary.readinessAttention}</strong>
                  readiness attention
                </span>
              </div>
              <div className="service-record-controls">
                <label className="service-browser-filter service-record-filter">
                  <Filter className="size-3.5" />
                  <span className="sr-only">Filter profile allocation rows</span>
                  <input
                    value={profileAllocationQuery}
                    onChange={(event) => setProfileAllocationQuery(event.target.value)}
                    placeholder="Filter runtime profiles, target identities, login identities, browsers, tasks"
                  />
                </label>
                <div className="service-profile-field-filters" aria-label="Profile routing field filters">
                  <label>
                    <span>Target</span>
                    <select value={profileTargetFilter} onChange={(event) => setProfileTargetFilter(event.target.value)}>
                      <option value="all">All target identities</option>
                      {profileTargetOptions.map((value) => <option key={value} value={value}>{value}</option>)}
                    </select>
                  </label>
                  <label>
                    <span>Login</span>
                    <select value={profileLoginFilter} onChange={(event) => setProfileLoginFilter(event.target.value)}>
                      <option value="all">All login identities</option>
                      {profileLoginOptions.map((value) => <option key={value} value={value}>{value}</option>)}
                    </select>
                  </label>
                  {profileBrowserBuildOptions.length > 0 && (
                    <label>
                      <span>Build</span>
                      <select value={profileBrowserBuildFilter} onChange={(event) => setProfileBrowserBuildFilter(event.target.value)}>
                        <option value="all">All browser builds</option>
                        {profileBrowserBuildOptions.map((value) => <option key={value} value={value}>{value}</option>)}
                      </select>
                    </label>
                  )}
                  <label>
                    <span>Readiness</span>
                    <select value={profileReadinessFilter} onChange={(event) => setProfileReadinessFilter(event.target.value as ProfileReadinessFilter)}>
                      <option value="all">Any readiness</option>
                      <option value="needs_attention">Needs attention</option>
                      <option value="normal">No readiness attention</option>
                    </select>
                  </label>
                </div>
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
                  Runtime profile config: {visibleProfileRecords.length} shown
                  {hiddenProfileRecordCount > 0 ? ` / ${hiddenProfileRecordCount} hidden` : ""}
                </p>
                {filteredProfileRecords.length === 0 ? (
                  <p className="rounded-2xl bg-foreground/[0.04] px-3 py-5 text-center text-xs text-muted-foreground">
                    {profileRecords.length === 0
                      ? "No runtime profile config records yet. Register a managed profile or launch a runtime profile to make configuration visible here."
                      : "No runtime profile config records match the current filter."}
                  </p>
                ) : (
                  <div className="service-runtime-profile-grid-list">
                    {visibleProfileRecords.map((profile, index) => {
                      const profileId = serviceProfileId(profile, `runtime-profile-${index}`);
                      const allocation = profileAllocationById.get(profileId);
                      return (
                        <RuntimeProfileConfigCard
                          key={profileId}
                          profile={profile}
                          allocation={allocation}
                          onSelect={inspectProfileAllocation}
                          onEdit={editRuntimeProfileConfig}
                          selected={Boolean(profileId && profileId === selectedProfileAllocationId)}
                        />
                      );
                    })}
                  </div>
                )}
              </div>
              <div className="service-section-list">
                <p id="service-profile-allocation-keyboard-hint" className="sr-only">
                  Profile routing rows support Arrow Up, Arrow Down, Home, and End within the visible row window.
                </p>
                <p className="service-record-list-heading">
                  Allocation and lease state: {visibleProfileAllocations.length} shown
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
                      onNavigate={navigateProfileAllocationRows}
                      rowRef={allocation.profileId ? (node) => setProfileAllocationRowRef(allocation.profileId, node) : undefined}
                      selected={Boolean(allocation.profileId && allocation.profileId === selectedProfileAllocationId)}
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
                      onClick={() => {
                        setIncidentHandlingFilter(option.value);
                        setIncidentFilterNotice("");
                      }}
                    >
                      {option.label}
                    </button>
                  ))}
                </div>
                <label className="service-browser-filter service-record-filter">
                  <span className="sr-only">Filter incidents</span>
                  <input
                    value={incidentQuery}
                    onChange={(event) => {
                      setIncidentQuery(event.target.value);
                      setIncidentFilterNotice("");
                    }}
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
                      onClick={() => {
                        setIncidentLimit(limit);
                        setIncidentFilterNotice("");
                      }}
                    >
                      {limit}
                    </button>
                  ))}
                </div>
              </div>
              {incidentFilterNotice && (
                <p className="service-workspace-inline-note">{incidentFilterNotice}</p>
              )}
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
                        viewStreamAvailable={tab.browserId ? canOpenControlViewStream(browserPrimaryViewStream(browserById.get(tab.browserId))) : false}
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
                <span>{jobActivitySummary.active} active / {jobActivitySummary.retained} retained; {filteredJobs.length} shown; {jobSummary}</span>
                {loading && <Loader2 className="size-3.5 animate-spin text-muted-foreground" />}
              </div>
              <div className="service-workspace-summary-chips" aria-label="Service job activity summary">
                <span>{jobActivitySummary.active} queued or running</span>
                <span>{jobActivitySummary.terminal} terminal</span>
                <span>{jobDisplaySummary}</span>
              </div>
              <div className="service-filter-bar" aria-label="Service job filters">
                <label className="service-browser-filter service-record-filter">
                  <Filter className="size-3.5" />
                  <span className="sr-only">Filter service jobs</span>
                  <input
                    value={jobQuery}
                    onChange={(event) => {
                      setJobQuery(event.target.value);
                      setJobFilterNotice("");
                    }}
                    placeholder="filter jobs, services, agents, tasks"
                  />
                </label>
                <label className="service-record-select">
                  <span>State</span>
                  <select
                    value={jobStateFilter}
                    onChange={(event) => {
                      setJobStateFilter(event.target.value);
                      setJobFilterNotice("");
                    }}
                  >
                    {SERVICE_JOB_STATE_FILTER_OPTIONS.map((option) => (
                      <option key={option.value} value={option.value}>{option.label}</option>
                    ))}
                  </select>
                </label>
                <label className="service-record-select">
                  <span>Display</span>
                  <select
                    value={jobDisplayFilter}
                    onChange={(event) => {
                      setJobDisplayFilter(event.target.value as ServiceJobDisplayFilter);
                      setJobFilterNotice("");
                    }}
                  >
                    {SERVICE_JOB_DISPLAY_FILTER_OPTIONS.map((option) => (
                      <option key={option.value} value={option.value}>{option.label}</option>
                    ))}
                  </select>
                </label>
              </div>
              <div className="service-filter-bar" aria-label="Service job sort and limit">
                <div className="service-filter-group">
                  <span>Sort</span>
                  {(["submittedAt", "state", "action", "displayIsolation", "serviceName", "taskName"] as ServiceJobSortKey[]).map((sortKey) => (
                    <JobSortButton
                      key={sortKey}
                      sortKey={sortKey}
                      activeSortKey={jobSortKey}
                      direction={jobSortDirection}
                      onSort={toggleJobSort}
                    />
                  ))}
                </div>
                <div className="service-filter-group" aria-label="Service job display limit">
                  <span>Rows</span>
                  {SERVICE_RECORD_LIMIT_OPTIONS.map((limit) => (
                    <button
                      key={limit}
                      type="button"
                      className={cn("service-filter-chip", jobLimit === limit && "service-filter-chip-active")}
                      onClick={() => setJobLimit(limit)}
                    >
                      {limit}
                    </button>
                  ))}
                </div>
              </div>
              {jobFilterNotice && (
                <p className="service-workspace-inline-note">{jobFilterNotice}</p>
              )}
              <div className="service-section-list">
                {filteredJobs.length === 0 ? (
                  <p className="rounded-2xl bg-foreground/[0.04] px-3 py-6 text-center text-xs text-muted-foreground">
                    {recentJobs.length === 0 ? "No service jobs yet." : "No service jobs match the current filters."}
                  </p>
                ) : (
                  filteredJobs.map((job) => <JobRow key={job.id} job={job} onSelect={inspectJob} />)
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
                  onShowJobsForDisplayAllocation={showJobsForDisplayAllocation}
                  onShowTraceJob={showTraceJob}
                  onShowTraceIncident={showTraceIncident}
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
