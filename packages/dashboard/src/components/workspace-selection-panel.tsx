"use client";

import { useMemo, useState } from "react";
import {
  Activity,
  AlertTriangle,
  BadgeCheck,
  Copy,
  Cpu,
  ExternalLink,
  Eye,
  Focus,
  HardDrive,
  MousePointer2,
  RefreshCw,
  RadioTower,
  XCircle,
  Wrench,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import {
  formatSelectedWorkspaceRuntimeValue,
  selectedWorkspaceDiagnosticBundle,
  type SelectedWorkspaceContext,
} from "@/lib/selected-workspace-context";
import { updateDashboardWorkspaceUrlSelection } from "@/lib/workspace-url-selection";
import type { WorkspaceNodeAction, WorkspaceNodeActionId } from "@/lib/service-workspaces";

type WorkspaceSelectionPanelProps = {
  context: SelectedWorkspaceContext;
  loading?: boolean;
  error?: string | null;
  onRefresh?: () => void;
};

const ACTION_ICONS = {
  focus: Focus,
  view: Eye,
  control: MousePointer2,
  "external-open": ExternalLink,
  "copy-link": Copy,
  repair: Wrench,
  close: XCircle,
  kill: XCircle,
  resume: Activity,
  launch: RadioTower,
  seed: BadgeCheck,
  "add-tab": BadgeCheck,
};

const ACTION_ORDER: WorkspaceNodeActionId[] = [
  "focus",
  "view",
  "control",
  "external-open",
  "repair",
  "resume",
  "launch",
  "seed",
  "add-tab",
  "close",
  "kill",
  "copy-link",
];

const FRONTEND_RUNNABLE_ACTIONS = new Set<WorkspaceNodeActionId>([
  "copy-link",
  "external-open",
  "view",
  "control",
]);

export function WorkspaceSelectionPanel({
  context,
  loading = false,
  error = null,
  onRefresh,
}: WorkspaceSelectionPanelProps) {
  const [copyState, setCopyState] = useState<"idle" | "copied" | "failed">("idle");
  const diagnosticText = useMemo(
    () => JSON.stringify(selectedWorkspaceDiagnosticBundle(context), null, 2),
    [context],
  );
  const sortedActions = useMemo(() => sortActions(context.actions), [context.actions]);
  const statusFacts = useMemo(() => buildStatusFacts(context), [context]);
  const priorityNotice = priorityWorkspaceNotice(context);
  const pageLabel = compactPageLabel(context);
  const refreshAge = formatAge(Date.now() - context.refreshedAt);

  const copyDiagnostics = async () => {
    try {
      await navigator.clipboard.writeText(diagnosticText);
      setCopyState("copied");
      window.setTimeout(() => setCopyState("idle"), 1400);
    } catch {
      setCopyState("failed");
      window.setTimeout(() => setCopyState("idle"), 1800);
    }
  };

  const runAction = async (actionId: string) => {
    if (actionId === "copy-link") {
      await navigator.clipboard.writeText(window.location.href);
      return;
    }
    if (actionId === "external-open" && context.stream?.url) {
      window.open(context.stream.url, "_blank", "noopener,noreferrer");
      return;
    }
    if (actionId === "view" || actionId === "control") {
      updateDashboardWorkspaceUrlSelection(context.selection, "replace");
      const params = new URLSearchParams(window.location.search);
      params.set("view", actionId === "control" ? "workspace:control" : "workspace:view");
      window.history.pushState(
        { ...(window.history.state ?? {}), dashboardWorkspaceView: params.get("view") },
        "",
        `${window.location.pathname}?${params.toString()}${window.location.hash}`,
      );
    }
  };

  return (
    <section
      className="workspace-selection-panel"
      aria-label="Selected workspace details"
      data-selected-workspace-id={context.node?.id ?? ""}
      data-selected-workspace-state={context.state}
      data-selected-workspace-source={context.source}
      data-selected-workspace-context="ready"
    >
      <header className="workspace-selection-header-strip">
        <div className="workspace-selection-title-cell">
          <p className="workspace-selection-kicker">Workspace</p>
          <h2 title={context.label}>{context.label}</h2>
          <p className="workspace-selection-page" title={pageLabel}>{pageLabel}</p>
        </div>
        <div className="workspace-selection-header-status" aria-label="Workspace status">
          <WorkspaceStatusBadge context={context} />
          <span title={`Source: ${context.source}`}>{context.source}</span>
          <span title={`Refreshed ${refreshAge}`}>{refreshAge}</span>
        </div>
        <div className="workspace-selection-indicators" aria-label="Runtime indicators">
          {statusFacts.map((fact) => (
            <MiniIndicator key={fact.label} {...fact} />
          ))}
        </div>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          aria-label="Refresh workspace context"
          title="Refresh workspace context"
          onClick={onRefresh}
          disabled={loading || !onRefresh}
        >
          <RefreshCw className={cn("size-4", loading && "animate-spin")} />
        </Button>
      </header>

      {(error || context.missingReason || priorityNotice) && (
        <div className="workspace-selection-alert" role="status">
          <AlertTriangle className="size-3.5" />
          <span>{error ? `Service context refresh failed: ${error}` : context.missingReason ?? priorityNotice}</span>
        </div>
      )}

      <div className="workspace-selection-actions" aria-label="Selected workspace actions">
        {sortedActions.map((action) => {
          const Icon = ACTION_ICONS[action.id as keyof typeof ACTION_ICONS];
          const unsupportedReason = action.enabled && !FRONTEND_RUNNABLE_ACTIONS.has(action.id)
            ? "Backend support is advertised, but this compact Workspace action is not wired here yet."
            : null;
          const reason = action.reason ?? unsupportedReason ?? action.label;
          const canRun = action.enabled && FRONTEND_RUNNABLE_ACTIONS.has(action.id);
          return (
            <Button
              key={action.id}
              type="button"
              variant={canRun ? "secondary" : "outline"}
              size="xs"
              className="workspace-selection-action"
              disabled={!canRun}
              title={reason}
              data-action-id={action.id}
              data-action-enabled={canRun ? "true" : "false"}
              data-action-reason={reason}
              onClick={() => void runAction(action.id)}
            >
              {Icon ? <Icon className="size-3.5" /> : null}
              {action.label}
            </Button>
          );
        })}
        <Button type="button" variant="outline" size="xs" className="workspace-selection-action" onClick={copyDiagnostics}>
          <Copy className="size-3.5" />
          {copyState === "copied" ? "Copied" : copyState === "failed" ? "Copy failed" : "Copy diagnostics"}
        </Button>
      </div>

      <FactGrid rows={workspaceFactRows(context)} />

      <details className="workspace-selection-evidence">
        <summary>Evidence</summary>
        <dl className="workspace-selection-details">
          {context.evidence.rows.map((row) => (
            <div key={`${row.label}:${row.value}`}>
              <dt>{row.label}</dt>
              <dd>{row.value}</dd>
            </div>
          ))}
        </dl>
      </details>
    </section>
  );
}

function FactGrid({ rows }: { rows: Array<[string, unknown]> }) {
  return (
    <dl className="workspace-selection-details">
      {rows.map(([label, value]) => (
        <div key={label}>
          <dt>{label}</dt>
          <dd>{displayValue(value)}</dd>
        </div>
      ))}
    </dl>
  );
}

function MiniIndicator({
  label,
  value,
  title,
  icon: Icon,
}: {
  label: string;
  value: string;
  title: string;
  icon: typeof Cpu;
}) {
  return (
    <span className="workspace-selection-indicator" title={title}>
      <Icon className="size-3" />
      <span>{label}</span>
      <strong>{value}</strong>
    </span>
  );
}

function WorkspaceStatusBadge({ context }: { context: SelectedWorkspaceContext }) {
  if (context.state === "missing" || context.state === "blocked") {
    return <Badge variant="destructive">{context.state}</Badge>;
  }
  if (context.live && context.controllable) return <Badge variant="default">live control</Badge>;
  if (context.live && context.viewable) return <Badge variant="secondary">live view</Badge>;
  if (context.retained) return <Badge variant="outline">retained</Badge>;
  return <Badge variant="outline">{context.state}</Badge>;
}

function workspaceFactRows(context: SelectedWorkspaceContext): Array<[string, unknown]> {
  return [
    ["Workspace", context.node?.id],
    ["Browser", context.node?.browserId ?? context.browser?.id],
    ["Session", context.daemonSession?.session ?? context.node?.serviceSessionId],
    ["Profile", context.profileAllocation?.profileId ?? context.node?.profileId],
    ["Owner", ownerLabel(context)],
    ["Attention", priorityWorkspaceNotice(context)],
    ["Health", context.node?.health],
    ["Host", context.node?.host ?? context.browser?.host],
    ["Build", context.node?.browserBuild ?? context.browser?.browserBuild ?? context.profileAllocation?.browserBuild],
    ["Running", formatSelectedWorkspaceRuntimeValue(context.runtime.running)],
    ["PID", context.runtime.pid],
    ["Memory", formatSelectedWorkspaceRuntimeValue(context.runtime.rssBytes, "bytes")],
    ["CPU", formatSelectedWorkspaceRuntimeValue(context.runtime.cpuSeconds, "seconds")],
    ["Uptime", "not reported"],
    ["CDP", context.runtime.cdpPort],
    ["Stream", context.runtime.streamPort],
    ["Last frame", context.runtime.lastFrameAt ? formatTimestampAge(context.runtime.lastFrameAt) : "not reported"],
    ["Provider", context.stream?.provider],
    ["Route", context.stream?.routeSummary],
    ["Input", context.stream?.controlInput],
    ["View", context.viewable ? "ready" : context.live ? "not embeddable" : "not live"],
    ["Control", context.controllable ? "ready" : context.live ? "not controllable" : "not live"],
    ["Title", context.primaryTab?.title],
    ["URL", context.primaryTab?.url],
    ["Target", context.primaryTab?.targetId],
    ["Lifecycle", context.primaryTab?.lifecycle],
    ["Jobs", context.jobs.length],
    ["Incidents", context.incidents.length],
    ["Diagnostics", context.diagnostics.length],
  ];
}

function buildStatusFacts(context: SelectedWorkspaceContext) {
  return [
    {
      label: "PID",
      value: displayValue(context.runtime.pid),
      title: `Browser process ID: ${displayValue(context.runtime.pid)}`,
      icon: Cpu,
    },
    {
      label: "RSS",
      value: formatSelectedWorkspaceRuntimeValue(context.runtime.rssBytes, "bytes"),
      title: `Resident memory: ${formatSelectedWorkspaceRuntimeValue(context.runtime.rssBytes, "bytes")}`,
      icon: HardDrive,
    },
    {
      label: "CPU",
      value: formatSelectedWorkspaceRuntimeValue(context.runtime.cpuSeconds, "seconds"),
      title: `CPU time: ${formatSelectedWorkspaceRuntimeValue(context.runtime.cpuSeconds, "seconds")}`,
      icon: Activity,
    },
    {
      label: "CDP",
      value: displayValue(context.runtime.cdpPort),
      title: `CDP port: ${displayValue(context.runtime.cdpPort)}`,
      icon: RadioTower,
    },
    {
      label: "Stream",
      value: displayValue(context.runtime.streamPort),
      title: `Stream port: ${displayValue(context.runtime.streamPort)}`,
      icon: Eye,
    },
  ];
}

function sortActions(actions: WorkspaceNodeAction[]): WorkspaceNodeAction[] {
  return [...actions].sort((left, right) => {
    const leftIndex = ACTION_ORDER.indexOf(left.id);
    const rightIndex = ACTION_ORDER.indexOf(right.id);
    return (leftIndex === -1 ? 999 : leftIndex) - (rightIndex === -1 ? 999 : rightIndex);
  });
}

function priorityWorkspaceNotice(context: SelectedWorkspaceContext): string | null {
  return context.missingReason ||
    context.node?.attentionReason ||
    context.diagnostics[0]?.message ||
    context.incidents.find((incident) => !incident.resolvedAt)?.latestMessage ||
    null;
}

function compactPageLabel(context: SelectedWorkspaceContext): string {
  return context.primaryTab?.title ||
    hostFromUrl(context.primaryTab?.url) ||
    context.primaryTab?.url ||
    context.stream?.routeSummary ||
    "No active page reported";
}

function hostFromUrl(value?: string | null): string | null {
  if (!value) return null;
  try {
    return new URL(value).host;
  } catch {
    return null;
  }
}

function formatTimestampAge(timestamp: number): string {
  return `${formatAge(Date.now() - timestamp)} ago`;
}

function formatAge(milliseconds: number): string {
  if (!Number.isFinite(milliseconds) || milliseconds < 0) return "unknown";
  const seconds = Math.floor(milliseconds / 1000);
  if (seconds < 2) return "now";
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h`;
  return `${Math.floor(hours / 24)}d`;
}

function displayValue(value: unknown): string {
  if (value === null || value === undefined || value === "") return "not reported";
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return String(value);
}

function ownerLabel(context: SelectedWorkspaceContext): string {
  return [
    context.ownership.serviceName,
    context.ownership.agentName,
    context.ownership.taskName,
  ].filter(Boolean).join(" / ") || "unknown";
}
