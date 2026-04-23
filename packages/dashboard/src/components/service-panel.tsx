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

type ControlPlaneSnapshot = {
  worker_state?: string;
  browser_health?: string;
  queue_depth?: number;
  queue_capacity?: number;
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
    | string;
  message: string;
  browserId?: string | null;
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
  browserIds?: string[];
  tabIds?: string[];
  createdAt?: string | null;
  expiresAt?: string | null;
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
  };
  reconciliation?: ReconciliationSnapshot | null;
  events?: ServiceEvent[];
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

type ApiResponse<T> = {
  success: boolean;
  data?: T;
  error?: string | null;
};

type EventKindFilter =
  | "all"
  | "reconciliation"
  | "browser_health_changed"
  | "tab_lifecycle_changed"
  | "reconciliation_error";
type EventWindowFilter = "all" | "15m" | "1h" | "24h";
type EventLimit = 8 | 20 | 50;

const EVENT_KIND_OPTIONS: Array<{ value: EventKindFilter; label: string }> = [
  { value: "all", label: "All" },
  { value: "reconciliation", label: "Reconcile" },
  { value: "browser_health_changed", label: "Health" },
  { value: "tab_lifecycle_changed", label: "Tabs" },
  { value: "reconciliation_error", label: "Errors" },
];

const EVENT_WINDOW_OPTIONS: Array<{ value: EventWindowFilter; label: string; milliseconds?: number }> = [
  { value: "all", label: "All time" },
  { value: "15m", label: "15m", milliseconds: 15 * 60 * 1000 },
  { value: "1h", label: "1h", milliseconds: 60 * 60 * 1000 },
  { value: "24h", label: "24h", milliseconds: 24 * 60 * 60 * 1000 },
];

const EVENT_LIMIT_OPTIONS: EventLimit[] = [8, 20, 50];

function serviceBase(port: number): string {
  return `http://localhost:${port}/api/service`;
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

function healthTone(value?: string): "good" | "warn" | "bad" | "neutral" {
  const normalized = (value ?? "").toLowerCase();
  if (["ready", "cdp_ready", "running"].includes(normalized)) return "good";
  if (["notstarted", "not_started", "idle", ""].includes(normalized)) return "neutral";
  if (["cdp_disconnected", "unreachable", "process_exited"].includes(normalized)) return "bad";
  return "warn";
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
  return (
    <span
      className={cn(
        "service-event-dot",
        isError && "service-event-dot-error",
        isHealth && "service-event-dot-health",
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

function JobRow({ job, onSelect }: { job: ServiceJob; onSelect: (job: ServiceJob) => void }) {
  const failed = job.state === "failed" || job.state === "timed_out" || job.state === "cancelled";
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
}: {
  job: ServiceJob | null;
  onOpenChange: (open: boolean) => void;
}) {
  const request = formatDetails(job?.request);
  const response = formatDetails(job?.response ?? job?.result);
  const target = formatDetails(job?.target);
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
            </div>
          </>
        )}
      </DialogContent>
    </Dialog>
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

export function ServicePanel() {
  const activePort = useAtomValue(activePortAtom);
  const activeSession = useAtomValue(activeSessionNameAtom);
  const [status, setStatus] = useState<ServiceStatusData | null>(null);
  const [events, setEvents] = useState<ServiceEventsData | null>(null);
  const [jobs, setJobs] = useState<ServiceJobsData | null>(null);
  const [loading, setLoading] = useState(false);
  const [reconciling, setReconciling] = useState(false);
  const [error, setError] = useState("");
  const [eventKind, setEventKind] = useState<EventKindFilter>("all");
  const [eventWindow, setEventWindow] = useState<EventWindowFilter>("all");
  const [eventLimit, setEventLimit] = useState<EventLimit>(8);
  const [eventBrowserId, setEventBrowserId] = useState("");
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
    (eventLimit === 8 ? 0 : 1);

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
      const [statusResp, jobsResp, eventsResp] = await Promise.all([
        fetch(`${serviceBase(activePort)}/status`),
        fetch(`${serviceBase(activePort)}/jobs?limit=8`),
        fetch(`${serviceBase(activePort)}/events?${params.toString()}`),
      ]);
      const statusJson = (await statusResp.json()) as ApiResponse<ServiceStatusData>;
      const jobsJson = (await jobsResp.json()) as ApiResponse<ServiceJobsData>;
      const eventsJson = (await eventsResp.json()) as ApiResponse<ServiceEventsData>;
      if (!statusJson.success) throw new Error(statusJson.error || "Service status failed");
      if (!jobsJson.success) throw new Error(jobsJson.error || "Service jobs failed");
      if (!eventsJson.success) throw new Error(eventsJson.error || "Service events failed");
      setStatus(statusJson.data ?? null);
      setJobs(jobsJson.data ?? null);
      setEvents(eventsJson.data ?? null);
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
    setError("");
    if (!canFetch) return;
    fetchService(true);
    const timer = setInterval(() => {
      if (document.visibilityState === "visible") fetchService(false);
    }, 7000);
    return () => clearInterval(timer);
  }, [canFetch, fetchService]);

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

  const serviceState = status?.service_state;
  const control = status?.control_plane;
  const reconciliation = serviceState?.reconciliation;
  const recentJobs = jobs?.jobs ?? Object.values(serviceState?.jobs ?? {}).slice(-8);
  const recentEvents = events?.events ?? serviceState?.events?.slice(-8) ?? [];
  const jobSummary =
    jobs?.matched !== undefined && jobs?.total !== undefined
      ? `${jobs.matched} of ${jobs.total} matched`
      : `Last ${recentJobs.length} retained service jobs`;
  const eventSummary =
    events?.matched !== undefined && events?.total !== undefined
      ? `${events.matched} of ${events.total} matched`
      : `Last ${recentEvents.length} retained service events`;
  const browserRecords = useMemo(
    () => Object.values(serviceState?.browsers ?? {}),
    [serviceState?.browsers],
  );
  const sessionRecords = useMemo(
    () => Object.values(serviceState?.sessions ?? {}),
    [serviceState?.sessions],
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
              detail={`Queue ${control?.queue_depth ?? serviceState?.controlPlane?.queueDepth ?? 0} of ${control?.queue_capacity ?? serviceState?.controlPlane?.queueCapacity ?? 0}`}
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
                  }}
                >
                  Reset {activeFilterCount}
                </button>
              )}
            </div>
            <Separator className="my-3" />
            <div className="space-y-1">
              {recentEvents.length === 0 ? (
                <p className="rounded-2xl bg-foreground/[0.04] px-3 py-6 text-center text-xs text-muted-foreground">
                  No service events yet.
                </p>
              ) : (
                recentEvents.map((event) => (
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
