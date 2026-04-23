"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { useAtomValue } from "jotai/react";
import {
  AlertTriangle,
  CheckCircle2,
  Clock3,
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
  kind: "reconciliation" | "browser_health_changed" | "reconciliation_error" | string;
  message: string;
  browserId?: string | null;
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
  browsers?: Record<string, unknown>;
  profiles?: Record<string, unknown>;
  sessions?: Record<string, unknown>;
  tabs?: Record<string, unknown>;
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

type ApiResponse<T> = {
  success: boolean;
  data?: T;
  error?: string | null;
};

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
  return (
    <span
      className={cn(
        "service-event-dot",
        isError && "service-event-dot-error",
        isHealth && "service-event-dot-health",
      )}
    />
  );
}

function EventRow({ event }: { event: ServiceEvent }) {
  return (
    <div className="service-event-row">
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
    </div>
  );
}

export function ServicePanel() {
  const activePort = useAtomValue(activePortAtom);
  const activeSession = useAtomValue(activeSessionNameAtom);
  const [status, setStatus] = useState<ServiceStatusData | null>(null);
  const [events, setEvents] = useState<ServiceEventsData | null>(null);
  const [loading, setLoading] = useState(false);
  const [reconciling, setReconciling] = useState(false);
  const [error, setError] = useState("");

  const canFetch = activePort > 0 && !!activeSession;

  const fetchService = useCallback(async (showSpinner: boolean) => {
    if (!canFetch) return;
    if (showSpinner) setLoading(true);
    setError("");
    try {
      const [statusResp, eventsResp] = await Promise.all([
        fetch(`${serviceBase(activePort)}/status`),
        fetch(`${serviceBase(activePort)}/events?limit=8`),
      ]);
      const statusJson = (await statusResp.json()) as ApiResponse<ServiceStatusData>;
      const eventsJson = (await eventsResp.json()) as ApiResponse<ServiceEventsData>;
      if (!statusJson.success) throw new Error(statusJson.error || "Service status failed");
      if (!eventsJson.success) throw new Error(eventsJson.error || "Service events failed");
      setStatus(statusJson.data ?? null);
      setEvents(eventsJson.data ?? null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Service API unavailable");
    } finally {
      if (showSpinner) setLoading(false);
    }
  }, [activePort, canFetch]);

  useEffect(() => {
    setStatus(null);
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

  const serviceState = status?.service_state;
  const control = status?.control_plane;
  const reconciliation = serviceState?.reconciliation;
  const recentEvents = events?.events ?? serviceState?.events?.slice(-8) ?? [];
  const entityCounts = useMemo(() => ({
    browsers: countEntries(serviceState?.browsers),
    profiles: countEntries(serviceState?.profiles),
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

          <div className="service-timeline-card">
            <div className="flex items-center gap-2 px-1">
              <CheckCircle2 className="size-4 text-success" />
              <div>
                <p className="text-xs font-black uppercase tracking-[0.18em] text-muted-foreground">
                  Recent events
                </p>
                <p className="text-[11px] text-muted-foreground">
                  Last {recentEvents.length} retained service events
                </p>
              </div>
              {loading && <Loader2 className="ml-auto size-3.5 animate-spin text-muted-foreground" />}
            </div>
            <Separator className="my-3" />
            <div className="space-y-1">
              {recentEvents.length === 0 ? (
                <p className="rounded-2xl bg-foreground/[0.04] px-3 py-6 text-center text-xs text-muted-foreground">
                  No service events yet.
                </p>
              ) : (
                recentEvents.map((event) => <EventRow key={event.id} event={event} />)
              )}
            </div>
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}
