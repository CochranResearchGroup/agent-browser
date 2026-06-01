"use client";

import { startTransition, useCallback, useDeferredValue, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { useAtom, useAtomValue, useSetAtom } from "jotai/react";
import {
  AlertTriangle,
  Archive,
  ChevronDown,
  CheckCircle2,
  CircleDot,
  Cpu,
  ExternalLink,
  Eye,
  Info,
  Loader2,
  MemoryStick,
  MonitorDot,
  MousePointer2,
  Plus,
  RotateCcw,
  Search,
  Send,
  ShieldAlert,
  Trash2,
  X,
} from "lucide-react";
import {
  deriveWorkspaceNodes,
  type WorkspaceNode,
  type WorkspaceNodeActionId,
  type WorkspaceNodeGroup,
  type WorkspaceNodeInput,
  type WorkspaceServiceBrowser,
  type WorkspaceServiceIncident,
  type WorkspaceServiceJob,
  type WorkspaceServiceProfileAllocation,
  type WorkspaceServiceSession,
  type WorkspaceServiceTab,
  type WorkspaceServiceViewStream,
} from "@/lib/service-workspaces";
import {
  createLauncherServiceRequestFromAccessPlan,
  createLauncherSessionArgsFromAccessPlan,
  deriveLauncherEligibilityPreview,
  launcherAccessPlanPosture,
  type LauncherAccessPlanPreview,
  type LauncherBrowserCapabilityRegistry,
  type LauncherControlInputPreference,
  type LauncherDisplayIsolation,
  type LauncherEligibilityPreview,
  type LauncherEligibilityRow,
  type LauncherProfileRecord,
  type LauncherServiceRequest,
  type LauncherViewStreamPreference,
} from "@/lib/launcher-eligibility";
import {
  DASHBOARD_WORKSPACE_QUERY_KEYS,
  DASHBOARD_WORKSPACE_SELECTION_EVENT,
  dashboardWorkspaceSelectionHasValue,
  readDashboardWorkspaceUrlSelection,
  updateDashboardWorkspaceUrlSelection,
  writeDashboardWorkspaceUrlSelection,
  type DashboardWorkspaceUrlSelection,
} from "@/lib/workspace-url-selection";
import { SERVICE_API_BASE } from "@/lib/dashboard-api";
import { execCommand } from "@/lib/exec";
import {
  activePortAtom,
  addTabAtom,
  closeAllSessionsAtom,
  closeSessionAtom,
  createSessionAtom,
  killSessionAtom,
  newSessionDialogAtom,
  sessionsAtom,
  switchTabAtom,
} from "@/store/sessions";
import { engineForPortAtom, tabsForPortAtom } from "@/store/tabs";
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
} from "@/components/ui/alert-dialog";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Separator } from "@/components/ui/separator";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

type WorkspaceScope = "all" | WorkspaceNodeGroup;
type LauncherRowFilter = "all" | "eligible" | "needs-action" | "blocked";

type ServiceStatusData = {
  service_state?: {
    browsers?: Record<string, WorkspaceServiceBrowser>;
    sessions?: Record<string, WorkspaceServiceSession>;
    tabs?: Record<string, WorkspaceServiceTab>;
    jobs?: Record<string, WorkspaceServiceJob>;
    incidents?: WorkspaceServiceIncident[];
    profiles?: Record<string, LauncherProfileRecord>;
    browserCapabilityRegistry?: LauncherBrowserCapabilityRegistry;
  };
  profileAllocations?: WorkspaceServiceProfileAllocation[];
};

type ServiceContractsData = {
  contracts?: {
    serviceRequest?: {
      actions?: string[];
    };
  };
};

type ApiResponse<T> = {
  success: boolean;
  data?: T;
  error?: string | null;
};

const BROWSER_OPTIONS: { id: string; label: string; engine?: string; provider?: string }[] = [
  { id: "chrome", label: "Chrome", engine: "chrome" },
  { id: "lightpanda", label: "Lightpanda", engine: "lightpanda" },
  { id: "agentcore", label: "AgentCore", provider: "agentcore" },
  { id: "browserbase", label: "Browserbase", provider: "browserbase" },
  { id: "browserless", label: "Browserless", provider: "browserless" },
  { id: "browser-use", label: "Browser Use", provider: "browser-use" },
  { id: "kernel", label: "Kernel", provider: "kernel" },
];

const SCOPE_LABELS: Record<WorkspaceScope, string> = {
  all: "All",
  active: "Active",
  "needs-attention": "Attention",
  retained: "Retained",
};

const LAUNCHER_DISPLAY_OPTIONS: Array<{ value: LauncherDisplayIsolation; label: string }> = [
  { value: "service_default", label: "Service plan" },
  { value: "private_virtual_display", label: "Private display" },
  { value: "shared_display", label: "Shared display" },
  { value: "ambient_display", label: "Ambient display" },
];

const LAUNCHER_VIEW_STREAM_OPTIONS: Array<{ value: LauncherViewStreamPreference; label: string }> = [
  { value: "service_default", label: "Service plan" },
  { value: "rdp_gateway", label: "RDP gateway" },
  { value: "novnc", label: "noVNC" },
  { value: "virtual_display_webrtc", label: "Virtual WebRTC" },
  { value: "chrome_tab_webrtc", label: "Tab WebRTC" },
  { value: "cdp_screencast", label: "CDP screencast" },
  { value: "external_url", label: "External URL" },
];

const LAUNCHER_CONTROL_INPUT_OPTIONS: Array<{ value: LauncherControlInputPreference; label: string }> = [
  { value: "service_default", label: "Service plan" },
  { value: "manual_attached_desktop", label: "Manual desktop" },
  { value: "vnc_input", label: "VNC input" },
  { value: "webrtc_input", label: "WebRTC input" },
  { value: "cdp_input", label: "CDP input" },
];

const DEFAULT_LAUNCH_TARGET_URL = "about:blank";
const WORKSPACE_ACTIVE_ROW_WINDOW = 64;
const WORKSPACE_ATTENTION_ROW_WINDOW = 48;
const WORKSPACE_RETAINED_ROW_WINDOW = 80;
const LAUNCHER_ROW_WINDOW = 48;
const DISMISSED_ATTENTION_STORAGE_KEY = "agent-browser-dashboard-dismissed-attention-workspaces";

const LAUNCHER_FILTER_LABELS: Record<LauncherRowFilter, string> = {
  all: "All",
  eligible: "Eligible",
  "needs-action": "Needs action",
  blocked: "Blocked",
};

const EMPTY_LAUNCHER_PREVIEW: LauncherEligibilityPreview = {
  rows: [],
  summary: {
    total: 0,
    eligible: 0,
    needsOperatorAction: 0,
    blocked: 0,
    accessPlanFetched: 0,
    registryExecutables: 0,
    runtimeProfiles: 0,
  },
};

function serviceBase(_activePort: number): string {
  return SERVICE_API_BASE;
}

function readDismissedAttentionIds(): Set<string> {
  if (typeof window === "undefined") return new Set();
  try {
    const parsed = JSON.parse(window.localStorage.getItem(DISMISSED_ATTENTION_STORAGE_KEY) ?? "[]");
    return new Set(Array.isArray(parsed) ? parsed.filter((item): item is string => typeof item === "string") : []);
  } catch {
    return new Set();
  }
}

function writeDismissedAttentionIds(ids: Set<string>): void {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(DISMISSED_ATTENTION_STORAGE_KEY, JSON.stringify([...ids]));
}

async function fetchWithTimeout(input: RequestInfo | URL, init: RequestInit = {}, timeoutMs = 10000): Promise<Response> {
  const controller = new AbortController();
  const timeout = window.setTimeout(() => controller.abort(), timeoutMs);
  try {
    return await fetch(input, { ...init, signal: controller.signal });
  } finally {
    window.clearTimeout(timeout);
  }
}

function pushServiceJobsView(jobId?: string | null): void {
  if (typeof window === "undefined") return;
  const url = new URL(window.location.href);
  url.pathname = "/service";
  url.searchParams.set("view", "service:jobs");
  if (jobId) url.searchParams.set("job", jobId);
  const nextUrl = `${url.pathname}${url.search}${url.hash}`;
  const currentUrl = `${window.location.pathname}${window.location.search}${window.location.hash}`;
  if (nextUrl !== currentUrl) {
    window.history.pushState({ dashboardSection: "service", serviceView: "jobs" }, "", nextUrl);
  }
  window.dispatchEvent(new PopStateEvent("popstate", { state: window.history.state }));
  window.dispatchEvent(
    new CustomEvent(DASHBOARD_WORKSPACE_SELECTION_EVENT, { detail: readDashboardWorkspaceUrlSelection() }),
  );
}

function pushWorkspaceViewportUrl(node: WorkspaceNode, mode: "view" | "control"): DashboardWorkspaceUrlSelection | null {
  return pushWorkspaceViewportSelectionUrl(workspaceUrlSelectionForNode(node), mode);
}

function pushWorkspaceViewportSelectionUrl(
  selection: DashboardWorkspaceUrlSelection,
  mode: "view" | "control",
): DashboardWorkspaceUrlSelection | null {
  if (typeof window === "undefined") return null;
  const url = new URL(window.location.href);
  url.pathname = "/";
  url.searchParams.set("view", `workspace:${mode}`);
  const values: Record<(typeof DASHBOARD_WORKSPACE_QUERY_KEYS)[number], string | null> = {
    workspace: selection.workspaceId,
    browser: selection.browserId,
    session: selection.sessionId,
    tab: selection.tabId,
    profile: selection.profileId,
    job: selection.jobId,
  };
  for (const key of DASHBOARD_WORKSPACE_QUERY_KEYS) {
    const value = values[key];
    if (value) url.searchParams.set(key, value);
    else url.searchParams.delete(key);
  }
  const nextUrl = `${url.pathname}${url.search}${url.hash}`;
  const currentUrl = `${window.location.pathname}${window.location.search}${window.location.hash}`;
  if (nextUrl !== currentUrl) {
    window.history.pushState({ dashboardSection: "overview", workspaceViewport: mode }, "", nextUrl);
  }
  window.dispatchEvent(new PopStateEvent("popstate", { state: window.history.state }));
  window.dispatchEvent(new CustomEvent(DASHBOARD_WORKSPACE_SELECTION_EVENT, { detail: selection }));
  return selection;
}

function pushWorkspaceTileUrl(): void {
  if (typeof window === "undefined") return;
  const url = new URL(window.location.href);
  url.pathname = "/";
  url.searchParams.set("view", "workspace:tile");
  for (const key of DASHBOARD_WORKSPACE_QUERY_KEYS) {
    url.searchParams.delete(key);
  }
  const nextUrl = `${url.pathname}${url.search}${url.hash}`;
  const currentUrl = `${window.location.pathname}${window.location.search}${window.location.hash}`;
  if (nextUrl !== currentUrl) {
    window.history.pushState({ dashboardSection: "overview", workspaceViewport: "tile" }, "", nextUrl);
  }
  window.dispatchEvent(new PopStateEvent("popstate", { state: window.history.state }));
  window.dispatchEvent(new CustomEvent(DASHBOARD_WORKSPACE_SELECTION_EVENT, { detail: readDashboardWorkspaceUrlSelection() }));
}

type ServiceRequestWorkspaceIdentity = {
  jobId: string | null;
  browserId: string | null;
  sessionId: string | null;
  tabId: string | null;
  profileId: string | null;
};

function recordValue(source: unknown, key: string): unknown {
  return source && typeof source === "object" ? (source as Record<string, unknown>)[key] : undefined;
}

function stringCandidate(value: unknown): string | null {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function firstStringCandidate(...values: unknown[]): string | null {
  for (const value of values) {
    const candidate = stringCandidate(value);
    if (candidate) return candidate;
  }
  return null;
}

function extractServiceRequestWorkspaceIdentity(data: unknown): ServiceRequestWorkspaceIdentity {
  if (!data || typeof data !== "object") {
    return {
      jobId: null,
      browserId: null,
      sessionId: null,
      tabId: null,
      profileId: null,
    };
  }
  const record = data as Record<string, unknown>;
  const browser = recordValue(record, "browser");
  const tab = recordValue(record, "tab");
  const session = recordValue(record, "session");
  const profile = recordValue(record, "profile");
  return {
    jobId: firstStringCandidate(record.jobId, record.job_id, record.id, record.job),
    browserId: firstStringCandidate(record.browserId, record.browser_id, record.browser, recordValue(browser, "id")),
    sessionId: firstStringCandidate(record.sessionId, record.session_id, record.session, recordValue(session, "id")),
    tabId: firstStringCandidate(record.tabId, record.tab_id, record.tab, recordValue(tab, "id")),
    profileId: firstStringCandidate(
      record.profileId,
      record.profile_id,
      record.runtimeProfile,
      record.runtime_profile,
      record.profile,
      recordValue(profile, "id"),
    ),
  };
}

function launchViewStream(browser?: WorkspaceServiceBrowser | null): WorkspaceServiceViewStream | null {
  const streams = browser?.viewStreams ?? [];
  return streams.find((stream) => Boolean(stream.url)) ?? null;
}

function selectionForLaunchedBrowser(
  browser: WorkspaceServiceBrowser,
  serviceStatus: ServiceStatusData | null,
  identity: ServiceRequestWorkspaceIdentity,
): DashboardWorkspaceUrlSelection {
  const serviceTabs = Object.values(serviceStatus?.service_state?.tabs ?? {});
  const firstBrowserTab = serviceTabs.find((tab) => tab.browserId === browser.id);
  return {
    workspaceId: `browser:${browser.id}`,
    browserId: browser.id,
    sessionId: identity.sessionId ?? browser.activeSessionIds?.[0] ?? null,
    tabId: identity.tabId ?? firstBrowserTab?.id ?? null,
    profileId: identity.profileId ?? browser.profileId ?? null,
    jobId: identity.jobId,
  };
}

function workspaceUrlSelectionForNode(node: WorkspaceNode): DashboardWorkspaceUrlSelection {
  return {
    workspaceId: node.id,
    browserId: node.browserId ?? node.relatedIds.browserIds[0] ?? null,
    sessionId: node.serviceSessionId ??
      node.daemonSession ??
      node.relatedIds.serviceSessionIds[0] ??
      node.relatedIds.daemonSessionNames[0] ??
      null,
    tabId: node.primaryTab?.id ?? node.relatedIds.tabIds[0] ?? null,
    profileId: node.profileId ?? node.relatedIds.profileIds[0] ?? null,
    jobId: node.relatedIds.jobIds[0] ?? null,
  };
}

function workspaceUrlSelectionScore(node: WorkspaceNode, selection: DashboardWorkspaceUrlSelection): number {
  let score = 0;
  if (selection.workspaceId) {
    if (selection.workspaceId === node.id) {
      score += 100;
    } else if (/^(browser|service-session|daemon-session|profile):/.test(selection.workspaceId)) {
      return 0;
    }
  }
  if (
    selection.browserId &&
    (selection.browserId === node.browserId || node.relatedIds.browserIds.includes(selection.browserId))
  ) {
    score += 30;
  }
  if (
    selection.sessionId &&
    (
      selection.sessionId === node.serviceSessionId ||
      selection.sessionId === node.daemonSession ||
      node.relatedIds.serviceSessionIds.includes(selection.sessionId) ||
      node.relatedIds.daemonSessionNames.includes(selection.sessionId)
    )
  ) {
    score += 25;
  }
  if (
    selection.tabId &&
    (selection.tabId === node.primaryTab?.id || node.relatedIds.tabIds.includes(selection.tabId))
  ) {
    score += 20;
  }
  if (
    selection.profileId &&
    (selection.profileId === node.profileId || node.relatedIds.profileIds.includes(selection.profileId))
  ) {
    score += 10;
  }
  if (
    selection.jobId &&
    node.relatedIds.jobIds.includes(selection.jobId)
  ) {
    score += 8;
  }
  return score;
}

function nodeIcon(node: WorkspaceNode) {
  if (node.state === "blocked") return ShieldAlert;
  if (node.group === "needs-attention") return AlertTriangle;
  if (node.state === "controllable") return MousePointer2;
  if (node.state === "view-only") return Eye;
  if (node.group === "retained") return Archive;
  if (node.state === "busy") return Loader2;
  return MonitorDot;
}

function groupNodes(nodes: WorkspaceNode[]): Record<WorkspaceNodeGroup, WorkspaceNode[]> {
  const grouped: Record<WorkspaceNodeGroup, WorkspaceNode[]> = {
    "needs-attention": [],
    active: [],
    retained: [],
  };
  for (const node of nodes) {
    grouped[node.group].push(node);
  }
  return grouped;
}

function nodeSearchText(node: WorkspaceNode): string {
  return [
    node.label,
    node.secondaryLabel,
    node.health,
    node.attentionReason,
    node.profileId,
    node.browserId,
    node.serviceSessionId,
    node.daemonSession,
    node.ownership.serviceName,
    node.ownership.agentName,
    node.ownership.taskName,
    node.takeover?.ownerLabel,
    node.takeover?.queueImpact,
    ...(node.diagnostics ?? []).map((diagnostic) => diagnostic.message),
    node.primaryTab?.title,
    node.primaryTab?.url,
  ].filter(Boolean).join(" ").toLowerCase();
}

function nodeStatusLabel(node: WorkspaceNode): string {
  if (node.takeover?.active) return "takeover";
  if (node.state === "needs-attention") return "attention";
  if (node.state === "controllable") return "control";
  if (node.state === "view-only") return "view";
  if (node.state === "blocked") return "needs review";
  return node.state;
}

function primaryAction(node: WorkspaceNode) {
  const operatorControlIds: WorkspaceNodeActionId[] = ["control", "view"];
  for (const id of operatorControlIds) {
    const action = node.actions.find((candidate) => candidate.id === id && candidate.enabled);
    if (action) return action;
  }
  if (node.takeover?.active) {
    return node.actions.find((action) => action.id === "resume") ?? node.actions[0];
  }
  const preferredIds: WorkspaceNodeActionId[] = ["focus", "launch", "seed"];
  for (const id of preferredIds) {
    const action = node.actions.find((candidate) => candidate.id === id && candidate.enabled);
    if (action) return action;
  }
  return node.actions.find((action) => action.id === "focus") ?? node.actions[0];
}

function compactCountLabel(node: WorkspaceNode): string {
  const parts = [];
  if (node.counts.tabs) parts.push(`${node.counts.tabs} tabs`);
  if (node.counts.jobs) parts.push(`${node.counts.jobs} jobs`);
  if (node.counts.incidents) parts.push(`${node.counts.incidents} incidents`);
  return parts.join(" / ");
}

function formatBytes(bytes?: number | null): string | null {
  if (!bytes || bytes <= 0) return null;
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  return `${Math.round(bytes / (1024 * 1024))} MB`;
}

function formatCpuSeconds(seconds?: number | null): string | null {
  if (seconds === null || seconds === undefined || !Number.isFinite(seconds)) return null;
  if (seconds >= 60) return `${Math.round(seconds / 60)}m CPU`;
  return `${Math.max(0, Math.round(seconds))}s CPU`;
}

function processIndicatorItems(node: WorkspaceNode): Array<{ label: string; value: string; icon: ReactNode }> {
  const process = node.process;
  if (!process) return [];
  const items: Array<{ label: string; value: string; icon: ReactNode }> = [];
  if (process.pid) items.push({ label: "PID", value: String(process.pid), icon: <Cpu className="size-3" /> });
  const memory = formatBytes(process.rssBytes);
  if (memory) items.push({ label: "Memory", value: memory, icon: <MemoryStick className="size-3" /> });
  const cpu = formatCpuSeconds(process.cpuSeconds);
  if (cpu) items.push({ label: "CPU", value: cpu, icon: <Cpu className="size-3" /> });
  if (process.cdpPort) items.push({ label: "CDP", value: String(process.cdpPort), icon: <MonitorDot className="size-3" /> });
  if (process.streamPort) items.push({ label: "Stream", value: String(process.streamPort), icon: <Eye className="size-3" /> });
  return items;
}

function actionIcon(actionId: WorkspaceNodeActionId) {
  if (actionId === "control") return <MousePointer2 className="size-3.5" />;
  if (actionId === "view") return <Eye className="size-3.5" />;
  if (actionId === "launch" || actionId === "seed") return <Plus className="size-3.5" />;
  if (actionId === "resume" || actionId === "repair") return <RotateCcw className="size-3.5" />;
  if (actionId === "external-open") return <ExternalLink className="size-3.5" />;
  return <CircleDot className="size-3.5" />;
}

function launcherStatusLabel(status: LauncherEligibilityRow["status"]): string {
  if (status === "eligible") return "Eligible";
  if (status === "needs-operator-action") return "Needs action";
  return "Blocked";
}

function launcherRemoteViewLabel(row: LauncherEligibilityRow): string {
  if (row.remoteView === "controllable") return "control";
  if (row.remoteView === "view-only") return "view";
  return "no viewport";
}

function launcherIdentityLabel(row: LauncherEligibilityRow): string {
  const target = row.targetServiceIds[0] ?? "any target";
  const login = row.loginIds[0] ?? row.accountIds[0] ?? "default identity";
  return `${target} / ${login}`;
}

function launcherFilterCount(rows: LauncherEligibilityRow[], filter: LauncherRowFilter): number {
  if (filter === "all") return rows.length;
  if (filter === "eligible") return rows.filter((row) => row.status === "eligible").length;
  if (filter === "needs-action") return rows.filter((row) => row.status === "needs-operator-action").length;
  return rows.filter((row) => row.status === "blocked").length;
}

function launcherRowMatchesFilter(row: LauncherEligibilityRow, filter: LauncherRowFilter): boolean {
  if (filter === "all") return true;
  if (filter === "eligible") return row.status === "eligible";
  if (filter === "needs-action") return row.status === "needs-operator-action";
  return row.status === "blocked";
}

function launcherRowSearchText(row: LauncherEligibilityRow): string {
  return [
    row.profileName,
    row.profileId,
    row.browserBuild,
    row.browserHost,
    row.browserHostId,
    row.browserId,
    row.executableId,
    row.capabilityId,
    row.launchAction,
    row.reason,
    row.reasonSource,
    row.serviceReason,
    row.evidenceSummary,
    ...row.targetServiceIds,
    ...row.loginIds,
    ...row.accountIds,
    ...row.serviceNames,
    ...row.agentNames,
    ...row.taskNames,
  ].filter(Boolean).join(" ").toLowerCase();
}

function LauncherEligibilityPanel({
  preview,
  planningRowId,
  planError,
  selectedRowId,
  selectedAccessPlan,
  targetUrl,
  displayIsolation,
  viewStreamProvider,
  controlInputProvider,
  launchingRowId,
  launchError,
  launchResult,
  onSelect,
  onPlan,
  onTargetUrlChange,
  onDisplayIsolationChange,
  onViewStreamProviderChange,
  onControlInputProviderChange,
  onLaunch,
}: {
  preview: LauncherEligibilityPreview;
  planningRowId: string | null;
  planError: string;
  selectedRowId: string | null;
  selectedAccessPlan?: LauncherAccessPlanPreview;
  targetUrl: string;
  displayIsolation: LauncherDisplayIsolation;
  viewStreamProvider: LauncherViewStreamPreference;
  controlInputProvider: LauncherControlInputPreference;
  launchingRowId: string | null;
  launchError: string;
  launchResult: { jobId: string | null; action: string; request: LauncherServiceRequest } | null;
  onSelect: (row: LauncherEligibilityRow) => void;
  onPlan: (row: LauncherEligibilityRow) => void;
  onTargetUrlChange: (value: string) => void;
  onDisplayIsolationChange: (value: LauncherDisplayIsolation) => void;
  onViewStreamProviderChange: (value: LauncherViewStreamPreference) => void;
  onControlInputProviderChange: (value: LauncherControlInputPreference) => void;
  onLaunch: (row: LauncherEligibilityRow) => void;
}) {
  const rows = preview.rows;
  const [rowFilter, setRowFilter] = useState<LauncherRowFilter>("all");
  const [rowQuery, setRowQuery] = useState("");
  const [visibleRowCount, setVisibleRowCount] = useState(LAUNCHER_ROW_WINDOW);
  const filteredRows = useMemo(() => {
    const query = rowQuery.trim().toLowerCase();
    return rows.filter((row) => launcherRowMatchesFilter(row, rowFilter) &&
      (query ? launcherRowSearchText(row).includes(query) : true));
  }, [rowFilter, rowQuery, rows]);
  const visibleRows = filteredRows.slice(0, visibleRowCount);
  const hiddenRowCount = Math.max(0, filteredRows.length - visibleRows.length);
  const selectedRow = rows.find((row) => row.id === selectedRowId) ?? filteredRows[0] ?? rows[0] ?? null;
  const selectedPosture = launcherAccessPlanPosture(selectedAccessPlan);
  const canLaunchSelected = Boolean(
    selectedRow &&
    selectedRow.status === "eligible" &&
    selectedAccessPlan &&
    selectedPosture &&
    launchingRowId !== selectedRow.id,
  );
  const launchBlockedReason = !selectedRow
    ? "No browser/profile combination selected."
    : selectedRow.status !== "eligible"
      ? selectedRow.reason
      : !selectedAccessPlan
        ? "Fetch a no-launch access plan before queueing a launch."
        : !selectedPosture
          ? "Access plan did not include a service request payload."
          : "";
  const visibleSummary = {
    eligible: preview.summary.eligible,
    needsOperatorAction: preview.summary.needsOperatorAction,
    blocked: preview.summary.blocked,
    accessPlanFetched: preview.summary.accessPlanFetched,
  };
  useEffect(() => {
    setVisibleRowCount(LAUNCHER_ROW_WINDOW);
  }, [rowFilter, rowQuery, rows.length]);
  useEffect(() => {
    if (selectedRowId && filteredRows.some((row) => row.id === selectedRowId)) return;
    if (filteredRows[0]) onSelect(filteredRows[0]);
  }, [filteredRows, onSelect, selectedRowId]);

  return (
    <section className="workspace-launcher-preview" aria-label="Browser and profile launch eligibility">
      <div className="workspace-launcher-summary">
        <span>
          <strong>{visibleSummary.eligible}</strong>
          eligible
        </span>
        <span>
          <strong>{visibleSummary.needsOperatorAction}</strong>
          needs action
        </span>
        <span>
          <strong>{visibleSummary.blocked}</strong>
          blocked
        </span>
        <span>
          <strong>{visibleSummary.accessPlanFetched}</strong>
          planned
        </span>
      </div>
      <div className="workspace-launcher-controls">
        <label className="workspace-launcher-search">
          <Search className="size-3.5" />
          <span className="sr-only">Filter browser and profile combinations</span>
          <input
            value={rowQuery}
            onChange={(event) => setRowQuery(event.target.value)}
            placeholder="Filter profile, browser, service, account"
          />
          {rowQuery && (
            <button type="button" onClick={() => setRowQuery("")} aria-label="Clear launcher filter">
              <X className="size-3" />
            </button>
          )}
        </label>
        <div className="workspace-launcher-filters" role="tablist" aria-label="Launch combination status">
          {(["all", "eligible", "needs-action", "blocked"] as LauncherRowFilter[]).map((filter) => (
            <button
              key={filter}
              type="button"
              role="tab"
              aria-selected={rowFilter === filter}
              className={cn(rowFilter === filter && "workspace-launcher-filter-active")}
              onClick={() => setRowFilter(filter)}
            >
              <span>{LAUNCHER_FILTER_LABELS[filter]}</span>
              <small>{launcherFilterCount(rows, filter)}</small>
            </button>
          ))}
        </div>
      </div>
      {planError && <p className="workspace-launcher-error">{planError}</p>}
      <div className="workspace-launcher-rows">
        {rows.length === 0 ? (
          <p className="workspace-launcher-empty">No service profiles are available.</p>
        ) : visibleRows.length === 0 ? (
          <p className="workspace-launcher-empty">No combinations match the current filter.</p>
        ) : visibleRows.map((row) => (
          <div
            key={row.id}
            className={cn("workspace-launcher-row", selectedRowId === row.id && "workspace-launcher-row-selected")}
          >
            <span className={cn("workspace-launcher-dot", `workspace-launcher-dot-${row.status}`)} />
            <button
              type="button"
              className="workspace-launcher-row-main"
              onClick={() => onSelect(row)}
              aria-pressed={selectedRowId === row.id}
            >
              <div className="workspace-launcher-row-title">
                <span>{row.profileName}</span>
                <Badge variant={row.status === "blocked" ? "destructive" : "secondary"} className="workspace-launcher-status">
                  {launcherStatusLabel(row.status)}
                </Badge>
              </div>
              <div className="workspace-launcher-route">
                <code>{row.browserBuild}</code>
                <span>{row.browserHost}</span>
                <span>{launcherRemoteViewLabel(row)}</span>
                <span>{row.launchAction}</span>
              </div>
              <p className="workspace-launcher-reason">{row.reason}</p>
              <div className="workspace-launcher-facts">
                <span>{launcherIdentityLabel(row)}</span>
                <span>{row.evidenceSummary}</span>
              </div>
            </button>
            <Tooltip>
              <TooltipTrigger asChild>
                <button
                  type="button"
                  className="workspace-launcher-plan-button"
                  onClick={() => {
                    onSelect(row);
                    onPlan(row);
                  }}
                  disabled={planningRowId === row.id}
                  aria-label={`Fetch access plan for ${row.profileName} with ${row.browserBuild}`}
                >
                  {planningRowId === row.id ? (
                    <Loader2 className="size-3.5 animate-spin" />
                  ) : row.accessPlanFetched ? (
                    <CheckCircle2 className="size-3.5" />
                  ) : (
                    <Eye className="size-3.5" />
                  )}
                  <span>{row.accessPlanFetched ? "Ready" : "Plan"}</span>
                </button>
              </TooltipTrigger>
              <TooltipContent side="right">Fetch no-launch access plan</TooltipContent>
            </Tooltip>
          </div>
        ))}
        {hiddenRowCount > 0 && (
          <button
            type="button"
            className="workspace-launcher-show-more"
            onClick={() => setVisibleRowCount((current) => current + LAUNCHER_ROW_WINDOW)}
          >
            Show {Math.min(LAUNCHER_ROW_WINDOW, hiddenRowCount)} more combinations
            <span>{visibleRows.length} of {filteredRows.length} shown</span>
          </button>
        )}
      </div>
      {selectedRow && (
        <div className="workspace-launcher-submit">
          <div className="workspace-launcher-submit-head">
            <div className="min-w-0">
              <strong>{selectedRow.profileName}</strong>
              <span>{selectedRow.browserBuild} / {selectedRow.browserHost}</span>
            </div>
            <Badge variant={selectedRow.status === "eligible" ? "secondary" : selectedRow.status === "blocked" ? "destructive" : "outline"}>
              {selectedPosture?.action ?? selectedRow.launchAction}
            </Badge>
          </div>
          <label className="workspace-launcher-field workspace-launcher-field-wide">
            Target URL
            <input
              value={targetUrl}
              onChange={(event) => onTargetUrlChange(event.target.value)}
              className="workspace-nav-dialog-input"
              placeholder={selectedPosture?.url || DEFAULT_LAUNCH_TARGET_URL}
            />
          </label>
          <div className="workspace-launcher-submit-actions">
            <button
              type="button"
              className="workspace-launcher-secondary-button"
              disabled={planningRowId === selectedRow.id}
              onClick={() => onPlan(selectedRow)}
            >
              {planningRowId === selectedRow.id ? <Loader2 className="size-3.5 animate-spin" /> : <Eye className="size-3.5" />}
              Plan selected
            </button>
            <button
              type="button"
              className="workspace-launcher-launch-button"
              disabled={!canLaunchSelected}
              onClick={() => onLaunch(selectedRow)}
              title={launchBlockedReason || "Queue service launch request"}
            >
              {launchingRowId === selectedRow.id ? <Loader2 className="size-3.5 animate-spin" /> : <Send className="size-3.5" />}
              Launch
            </button>
          </div>
          <div className="workspace-launcher-select-grid">
            <label className="workspace-launcher-field">
              Display
              <select
                value={displayIsolation}
                onChange={(event) => onDisplayIsolationChange(event.target.value as LauncherDisplayIsolation)}
                className="workspace-nav-dialog-input"
              >
                {LAUNCHER_DISPLAY_OPTIONS.map((option) => (
                  <option key={option.value} value={option.value}>{option.label}</option>
                ))}
              </select>
            </label>
            <label className="workspace-launcher-field">
              View
              <select
                value={viewStreamProvider}
                onChange={(event) => onViewStreamProviderChange(event.target.value as LauncherViewStreamPreference)}
                className="workspace-nav-dialog-input"
              >
                {LAUNCHER_VIEW_STREAM_OPTIONS.map((option) => (
                  <option key={option.value} value={option.value}>{option.label}</option>
                ))}
              </select>
            </label>
            <label className="workspace-launcher-field">
              Control
              <select
                value={controlInputProvider}
                onChange={(event) => onControlInputProviderChange(event.target.value as LauncherControlInputPreference)}
                className="workspace-nav-dialog-input"
              >
                {LAUNCHER_CONTROL_INPUT_OPTIONS.map((option) => (
                  <option key={option.value} value={option.value}>{option.label}</option>
                ))}
              </select>
            </label>
          </div>
          <div className="workspace-launcher-request-summary">
            <span>lease {selectedPosture?.profileLeasePolicy || "service"}</span>
            <span>display {selectedPosture?.displayIsolation || "service plan"}</span>
            <span>view {selectedPosture?.viewStreamProvider || "service plan"}</span>
            <span>control {selectedPosture?.controlInputProvider || "service plan"}</span>
          </div>
          {launchBlockedReason && <p className="workspace-launcher-hint">{launchBlockedReason}</p>}
          {launchError && <p className="workspace-launcher-error">{launchError}</p>}
          {launchResult && (
            <p className="workspace-launcher-success">
              <CheckCircle2 className="size-3.5" />
              Queued {launchResult.action}{launchResult.jobId ? ` / ${launchResult.jobId}` : ""}.
            </p>
          )}
        </div>
      )}
    </section>
  );
}

function WorkspaceGroup({
  title,
  nodes,
  selectedNodeId,
  defaultOpen = true,
  rowWindow = 0,
  onSelect,
  onPrimaryAction,
  onClose,
  onKill,
  onDismiss,
}: {
  title: string;
  nodes: WorkspaceNode[];
  selectedNodeId: string | null;
  defaultOpen?: boolean;
  rowWindow?: number;
  onSelect: (node: WorkspaceNode) => void;
  onPrimaryAction: (node: WorkspaceNode) => void;
  onClose: (node: WorkspaceNode) => void;
  onKill: (node: WorkspaceNode) => void;
  onDismiss?: (node: WorkspaceNode) => void;
}) {
  const [open, setOpen] = useState(defaultOpen);
  const [visibleNodeCount, setVisibleNodeCount] = useState(rowWindow || nodes.length);
  const selectedNodeIndex = selectedNodeId ? nodes.findIndex((node) => node.id === selectedNodeId) : -1;
  const rowWindowEnabled = rowWindow > 0;
  const targetVisibleNodeCount = rowWindowEnabled
    ? Math.max(rowWindow, selectedNodeIndex >= 0 ? selectedNodeIndex + 1 : rowWindow)
    : nodes.length;
  const visibleNodes = rowWindowEnabled ? nodes.slice(0, visibleNodeCount) : nodes;
  const hiddenNodeCount = Math.max(0, nodes.length - visibleNodes.length);

  useEffect(() => {
    setOpen(defaultOpen);
  }, [defaultOpen]);

  useEffect(() => {
    setVisibleNodeCount((current) => {
      if (!rowWindowEnabled) return nodes.length;
      if (selectedNodeIndex >= current) return targetVisibleNodeCount;
      return Math.min(Math.max(current, rowWindow), nodes.length);
    });
  }, [nodes.length, rowWindow, rowWindowEnabled, selectedNodeIndex, targetVisibleNodeCount]);

  if (nodes.length === 0) return null;

  return (
    <section className="workspace-nav-group">
      <button
        type="button"
        className="workspace-nav-group-header"
        onClick={() => setOpen((current) => !current)}
        aria-expanded={open}
      >
        <ChevronDown className={cn("size-3 transition-transform", !open && "-rotate-90")} />
        <span>{title}</span>
        <Badge variant="secondary" className="ml-auto h-4 px-1.5 text-[10px] tabular-nums">
          {nodes.length}
        </Badge>
      </button>
      {open && (
        <div className="workspace-nav-rows">
          {visibleNodes.map((node) => (
            <WorkspaceNodeRow
              key={node.id}
              node={node}
              selected={selectedNodeId === node.id}
              onSelect={() => onSelect(node)}
              onPrimaryAction={() => onPrimaryAction(node)}
              onClose={() => onClose(node)}
              onKill={() => onKill(node)}
              onDismiss={onDismiss ? () => onDismiss(node) : undefined}
            />
          ))}
          {hiddenNodeCount > 0 && (
            <button
              type="button"
              className="workspace-nav-show-more"
              onClick={() => setVisibleNodeCount((current) => Math.min(nodes.length, current + Math.max(rowWindow, WORKSPACE_RETAINED_ROW_WINDOW)))}
            >
              Show {Math.min(Math.max(rowWindow, WORKSPACE_RETAINED_ROW_WINDOW), hiddenNodeCount)} more rows
              <span>{visibleNodes.length} of {nodes.length} shown</span>
            </button>
          )}
        </div>
      )}
    </section>
  );
}

function WorkspaceNodeRow({
  node,
  selected,
  onSelect,
  onPrimaryAction,
  onClose,
  onKill,
  onDismiss,
}: {
  node: WorkspaceNode;
  selected: boolean;
  onSelect: () => void;
  onPrimaryAction: () => void;
  onClose: () => void;
  onKill: () => void;
  onDismiss?: () => void;
}) {
  const Icon = nodeIcon(node);
  const action = primaryAction(node);
  const countLabel = compactCountLabel(node);
  const processItems = processIndicatorItems(node).slice(0, 3);

  return (
    <div className={cn("workspace-nav-row", selected && "workspace-nav-row-selected")}>
      <button
        type="button"
        className="workspace-nav-row-main"
        onClick={onSelect}
        aria-current={selected ? "true" : undefined}
      >
        <span className={cn("workspace-nav-row-icon", node.state === "busy" && "animate-spin")}>
          <Icon className="size-3.5" />
        </span>
        <span className="min-w-0 flex-1">
          <span className="workspace-nav-row-title">{node.label}</span>
          <span className="workspace-nav-row-meta">{node.secondaryLabel || node.id}</span>
          {processItems.length > 0 && (
            <span className="workspace-nav-row-indicators">
              {processItems.map((item) => (
                <span key={`${item.label}:${item.value}`} title={`${item.label}: ${item.value}`}>
                  {item.icon}
                  {item.value}
                </span>
              ))}
            </span>
          )}
          {countLabel && <span className="workspace-nav-row-counts">{countLabel}</span>}
        </span>
      </button>
      <div className="workspace-nav-row-side">
        <Badge
          variant={node.group === "needs-attention" ? "destructive" : "secondary"}
          className="workspace-nav-state-badge"
        >
          {nodeStatusLabel(node)}
        </Badge>
        {action && (
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                type="button"
                className="workspace-nav-icon-action"
                disabled={!action.enabled}
                onClick={onPrimaryAction}
                aria-label={action.label}
                title={action.reason ?? action.label}
              >
                {actionIcon(action.id)}
              </button>
            </TooltipTrigger>
            <TooltipContent side="right">
              {action.reason ?? action.label}
            </TooltipContent>
          </Tooltip>
        )}
        {node.port ? (
          <>
            <Tooltip>
              <TooltipTrigger asChild>
                <button
                  type="button"
                  className="workspace-nav-icon-action"
                  onClick={onClose}
                  aria-label={`Close ${node.label}`}
                >
                  <Trash2 className="size-3.5" />
                </button>
              </TooltipTrigger>
              <TooltipContent side="right">Close workspace</TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger asChild>
                <button
                  type="button"
                  className="workspace-nav-icon-action workspace-nav-danger-action"
                  onClick={onKill}
                  aria-label={`Kill ${node.label}`}
                >
                  <X className="size-3.5" />
                </button>
              </TooltipTrigger>
              <TooltipContent side="right">Kill workspace</TooltipContent>
            </Tooltip>
          </>
        ) : null}
        {node.group === "needs-attention" && onDismiss ? (
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                type="button"
                className="workspace-nav-icon-action"
                onClick={onDismiss}
                aria-label={`Dismiss ${node.label}`}
              >
                <Archive className="size-3.5" />
              </button>
            </TooltipTrigger>
            <TooltipContent side="right">Dismiss from Workspaces</TooltipContent>
          </Tooltip>
        ) : null}
      </div>
      {node.attentionReason && (
        <div className="workspace-nav-attention">
          {node.attentionReason}
        </div>
      )}
    </div>
  );
}

function WorkspaceNodeDetail({
  node,
  onAction,
}: {
  node: WorkspaceNode;
  onAction: (node: WorkspaceNode, action: WorkspaceNodeActionId) => void;
}) {
  const processItems = processIndicatorItems(node);
  const detailActionIds = new Set<WorkspaceNodeActionId>([
    "control",
    "view",
    "focus",
    "launch",
    "seed",
    "resume",
    "repair",
    "add-tab",
    "external-open",
  ]);
  const enabledActions = node.actions.filter((action) => action.enabled && detailActionIds.has(action.id));
  const disabledActions = node.actions.filter((action) => !action.enabled).slice(0, 3);
  return (
    <section className="workspace-nav-detail" aria-label="Selected workspace detail">
      <div className="workspace-nav-detail-header">
        <Info className="size-3.5" />
        <div className="min-w-0">
          <p>{node.label}</p>
          <span>{node.id}</span>
        </div>
      </div>
      {node.attentionReason && (
        <p className="workspace-nav-detail-attention">{node.attentionReason}</p>
      )}
      <div className="workspace-nav-detail-grid">
        <span>State</span>
        <strong>{nodeStatusLabel(node)}</strong>
        <span>Health</span>
        <strong>{node.health ?? (node.live ? "live" : "retained")}</strong>
        <span>Browser</span>
        <strong>{node.browserId ?? node.daemonSession ?? "none"}</strong>
        <span>Profile</span>
        <strong>{node.profileId ?? "none"}</strong>
      </div>
      {processItems.length > 0 && (
        <div className="workspace-nav-detail-indicators">
          {processItems.map((item) => (
            <span key={`${item.label}:${item.value}`} title={`${item.label}: ${item.value}`}>
              {item.icon}
              <strong>{item.value}</strong>
              <small>{item.label}</small>
            </span>
          ))}
        </div>
      )}
      <div className="workspace-nav-detail-actions">
        {enabledActions.map((action) => (
          <Button
            key={action.id}
            type="button"
            size="sm"
            variant={action.id === "control" ? "default" : "outline"}
            className="workspace-nav-detail-action"
            onClick={() => onAction(node, action.id)}
          >
            {actionIcon(action.id)}
            {action.label}
          </Button>
        ))}
      </div>
      {disabledActions.length > 0 && (
        <div className="workspace-nav-detail-disabled">
          {disabledActions.map((action) => (
            <span key={action.id}>{action.label}: {action.reason ?? "Unavailable"}</span>
          ))}
        </div>
      )}
    </section>
  );
}

export function WorkspaceNavigator() {
  const sessions = useAtomValue(sessionsAtom);
  const activePort = useAtomValue(activePortAtom);
  const setActivePort = useSetAtom(activePortAtom);
  const getTabsForSession = useAtomValue(tabsForPortAtom);
  const getEngineForSession = useAtomValue(engineForPortAtom);
  const dispatchCreateSession = useSetAtom(createSessionAtom);
  const dispatchCloseSession = useSetAtom(closeSessionAtom);
  const dispatchKillSession = useSetAtom(killSessionAtom);
  const dispatchCloseAllSessions = useSetAtom(closeAllSessionsAtom);
  const dispatchAddTab = useSetAtom(addTabAtom);
  const dispatchSwitchTab = useSetAtom(switchTabAtom);
  const [newSessionOpen, setNewSessionOpen] = useAtom(newSessionDialogAtom);
  const [serviceStatus, setServiceStatus] = useState<ServiceStatusData | null>(null);
  const [serviceContracts, setServiceContracts] = useState<ServiceContractsData | null>(null);
  const [browserCapabilityRegistry, setBrowserCapabilityRegistry] = useState<LauncherBrowserCapabilityRegistry | null>(null);
  const [launcherAccessPlans, setLauncherAccessPlans] = useState<Record<string, LauncherAccessPlanPreview>>({});
  const [selectedLauncherRowId, setSelectedLauncherRowId] = useState<string | null>(null);
  const [launcherTargetUrl, setLauncherTargetUrl] = useState(DEFAULT_LAUNCH_TARGET_URL);
  const [launcherDisplayIsolation, setLauncherDisplayIsolation] = useState<LauncherDisplayIsolation>("shared_display");
  const [launcherViewStreamProvider, setLauncherViewStreamProvider] = useState<LauncherViewStreamPreference>("rdp_gateway");
  const [launcherControlInputProvider, setLauncherControlInputProvider] =
    useState<LauncherControlInputPreference>("manual_attached_desktop");
  const [launcherPlanLoadingId, setLauncherPlanLoadingId] = useState<string | null>(null);
  const [launcherPlanError, setLauncherPlanError] = useState("");
  const [launcherLaunchLoadingId, setLauncherLaunchLoadingId] = useState<string | null>(null);
  const [launcherLaunchError, setLauncherLaunchError] = useState("");
  const [launcherLaunchResult, setLauncherLaunchResult] = useState<{
    jobId: string | null;
    action: string;
    request: LauncherServiceRequest;
  } | null>(null);
  const [dismissedAttentionIds, setDismissedAttentionIds] = useState<Set<string>>(() => readDismissedAttentionIds());
  const [serviceError, setServiceError] = useState("");
  const [query, setQuery] = useState("");
  const deferredQuery = useDeferredValue(query);
  const [scope, setScope] = useState<WorkspaceScope>("all");
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [urlSelection, setUrlSelection] = useState<DashboardWorkspaceUrlSelection>(() => readDashboardWorkspaceUrlSelection());
  const [newSessionName, setNewSessionName] = useState("");
  const [newSessionBrowser, setNewSessionBrowser] = useState("chrome");
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState("");
  const [pendingDangerAction, setPendingDangerAction] = useState<{
    type: "close" | "kill" | "close-all";
    node?: WorkspaceNode;
  } | null>(null);
  const loadedOnceRef = useRef(false);
  const lastScrolledSelectionRef = useRef<string | null>(null);

  const fetchServiceStatus = useCallback(async (): Promise<ServiceStatusData | null> => {
    if (typeof window === "undefined") return null;
    try {
      const base = serviceBase(activePort);
      const statusPromise = fetch(`${serviceBase(activePort)}/status`);
      const [statusResp, contractsResp, registryResp] = await Promise.all([
        statusPromise,
        fetch(`${base}/contracts`).catch(() => null),
        fetch(`${base}/browser-capability-registry`).catch(() => null),
      ]);
      const json = (await statusResp.json()) as ApiResponse<ServiceStatusData>;
      if (!json.success) throw new Error(json.error || "Service status failed");
      const contractsJson = contractsResp?.ok
        ? ((await contractsResp.json()) as ApiResponse<ServiceContractsData>)
        : null;
      const registryJson = registryResp?.ok
        ? ((await registryResp.json()) as ApiResponse<LauncherBrowserCapabilityRegistry>)
        : null;
      loadedOnceRef.current = true;
      startTransition(() => {
        setServiceStatus(json.data ?? null);
        setServiceContracts(contractsJson?.success ? contractsJson.data ?? null : null);
        setBrowserCapabilityRegistry(registryJson?.success ? registryJson.data ?? null : null);
        setServiceError("");
      });
      return json.data ?? null;
    } catch (err) {
      setServiceError(err instanceof Error ? err.message : "Service status unavailable");
      return null;
    }
  }, [activePort]);

  useEffect(() => {
    void fetchServiceStatus();
    const timer = setInterval(fetchServiceStatus, 7000);
    return () => clearInterval(timer);
  }, [fetchServiceStatus]);

  useEffect(() => {
    const onUrlSelectionChange = () => setUrlSelection(readDashboardWorkspaceUrlSelection());
    const onPopState = () => onUrlSelectionChange();
    window.addEventListener("popstate", onPopState);
    window.addEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, onUrlSelectionChange);
    return () => {
      window.removeEventListener("popstate", onPopState);
      window.removeEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, onUrlSelectionChange);
    };
  }, []);

  useEffect(() => {
    if (newSessionOpen && !newSessionName) {
      const existing = new Set(sessions.map((session) => session.session));
      let n = sessions.length + 1;
      while (existing.has(`session-${n}`)) n++;
      setNewSessionName(`session-${n}`);
    }
  }, [newSessionName, newSessionOpen, sessions]);

  const workspaceInput = useMemo<WorkspaceNodeInput>(() => {
    const daemonTabsByPort: Record<number, ReturnType<typeof getTabsForSession>> = {};
    const daemonEngineByPort: Record<number, string> = {};
    for (const session of sessions) {
      daemonTabsByPort[session.port] = getTabsForSession(session.port);
      daemonEngineByPort[session.port] = getEngineForSession(session.port);
    }
    const serviceState = serviceStatus?.service_state;
    return {
      daemonSessions: sessions,
      daemonTabsByPort,
      daemonEngineByPort,
      serviceBrowsers: Object.values(serviceState?.browsers ?? {}),
      serviceSessions: Object.values(serviceState?.sessions ?? {}),
      serviceTabs: Object.values(serviceState?.tabs ?? {}),
      profileAllocations: serviceStatus?.profileAllocations ?? [],
      jobs: Object.values(serviceState?.jobs ?? {}),
      incidents: serviceState?.incidents ?? [],
    };
  }, [getEngineForSession, getTabsForSession, serviceStatus, sessions]);

  const nodes = useMemo(() => deriveWorkspaceNodes(workspaceInput), [workspaceInput]);
  const visibleNodes = useMemo(
    () => nodes.filter((node) => !(node.group === "needs-attention" && dismissedAttentionIds.has(node.id))),
    [dismissedAttentionIds, nodes],
  );
  const dismissedAttentionCount = useMemo(
    () => nodes.filter((node) => node.group === "needs-attention" && dismissedAttentionIds.has(node.id)).length,
    [dismissedAttentionIds, nodes],
  );
  const filteredNodes = useMemo(() => {
    const text = deferredQuery.trim().toLowerCase();
    return visibleNodes.filter((node) => {
      if (scope !== "all" && node.group !== scope) return false;
      return text ? nodeSearchText(node).includes(text) : true;
    });
  }, [deferredQuery, scope, visibleNodes]);
  const grouped = useMemo(() => groupNodes(filteredNodes), [filteredNodes]);
  const counts = useMemo(() => groupNodes(visibleNodes), [visibleNodes]);
  const selectedNode = useMemo(
    () => selectedNodeId ? nodes.find((node) => node.id === selectedNodeId) ?? null : null,
    [nodes, selectedNodeId],
  );
  const launcherPreview = useMemo(() => {
    if (!newSessionOpen) return EMPTY_LAUNCHER_PREVIEW;
    const serviceState = serviceStatus?.service_state;
    const profiles = Object.entries(serviceState?.profiles ?? {}).map(([id, profile]) => ({
      ...profile,
      id: profile.id ?? id,
    }));
    return deriveLauncherEligibilityPreview({
      profiles,
      allocations: serviceStatus?.profileAllocations ?? [],
      browsers: Object.values(serviceState?.browsers ?? {}),
      browserCapabilityRegistry: browserCapabilityRegistry ?? serviceState?.browserCapabilityRegistry ?? null,
      accessPlans: Object.values(launcherAccessPlans),
      serviceRequestActions: serviceContracts?.contracts?.serviceRequest?.actions ?? [],
    });
  }, [browserCapabilityRegistry, launcherAccessPlans, newSessionOpen, serviceContracts, serviceStatus]);
  const selectedLauncherAccessPlan = selectedLauncherRowId ? launcherAccessPlans[selectedLauncherRowId] : undefined;
  const selectedLauncherPosture = launcherAccessPlanPosture(selectedLauncherAccessPlan);

  useEffect(() => {
    if (!newSessionOpen) return;
    if (selectedLauncherRowId && launcherPreview.rows.some((row) => row.id === selectedLauncherRowId)) return;
    const firstRow = launcherPreview.rows.find((row) => row.status === "eligible") ?? launcherPreview.rows[0] ?? null;
    setSelectedLauncherRowId(firstRow?.id ?? null);
  }, [launcherPreview.rows, newSessionOpen, selectedLauncherRowId]);

  useEffect(() => {
    if (!selectedLauncherPosture) return;
    if (launcherTargetUrl && launcherTargetUrl !== DEFAULT_LAUNCH_TARGET_URL) return;
    setLauncherTargetUrl(selectedLauncherPosture.url || DEFAULT_LAUNCH_TARGET_URL);
  }, [launcherTargetUrl, selectedLauncherPosture]);

  const fetchLauncherAccessPlan = useCallback(async (row: LauncherEligibilityRow) => {
    setSelectedLauncherRowId(row.id);
    setLauncherPlanLoadingId(row.id);
    setLauncherPlanError("");
    setLauncherLaunchError("");
    setLauncherLaunchResult(null);
    try {
      const params = new URLSearchParams();
      params.set("readinessProfileId", row.profileId);
      if (row.browserBuild && row.browserBuild !== "service default") params.set("browserBuild", row.browserBuild);
      if (row.targetServiceIds[0]) params.set("targetServiceId", row.targetServiceIds[0]);
      if (row.loginIds[0]) params.set("loginId", row.loginIds[0]);
      if (row.accountIds[0]) params.set("accountId", row.accountIds[0]);
      if (row.serviceNames[0]) params.set("serviceName", row.serviceNames[0]);
      if (row.agentNames[0]) params.set("agentName", row.agentNames[0]);
      if (row.taskNames[0]) params.set("taskName", row.taskNames[0]);
      const targetUrl = launcherTargetUrl.trim();
      if (targetUrl && targetUrl !== DEFAULT_LAUNCH_TARGET_URL) params.set("url", targetUrl);
      if (launcherDisplayIsolation !== "service_default") params.set("displayIsolation", launcherDisplayIsolation);
      if (launcherViewStreamProvider !== "service_default") {
        params.set("viewStreamProvider", launcherViewStreamProvider);
        if (launcherViewStreamProvider === "rdp_gateway") {
          params.set("browserHost", "remote_headed");
        }
      }
      if (launcherControlInputProvider !== "service_default") {
        params.set("controlInputProvider", launcherControlInputProvider);
      }
      const resp = await fetchWithTimeout(`${serviceBase(activePort)}/access-plan?${params.toString()}`);
      const json = (await resp.json()) as ApiResponse<LauncherAccessPlanPreview>;
      if (!json.success) throw new Error(json.error || "Access plan failed");
      const plannedAccessPlan = {
        ...(json.data ?? {}),
        comboId: row.id,
        profileId: row.profileId,
        browserBuild: row.browserBuild,
      };
      const posture = launcherAccessPlanPosture(plannedAccessPlan);
      setLauncherAccessPlans((current) => ({
        ...current,
        [row.id]: plannedAccessPlan,
      }));
      if (posture?.url && (!launcherTargetUrl || launcherTargetUrl === DEFAULT_LAUNCH_TARGET_URL)) {
        setLauncherTargetUrl(posture.url);
      }
    } catch (err) {
      setLauncherPlanError(err instanceof Error ? err.message : "Access plan unavailable");
    } finally {
      setLauncherPlanLoadingId(null);
    }
  }, [
    activePort,
    launcherControlInputProvider,
    launcherDisplayIsolation,
    launcherTargetUrl,
    launcherViewStreamProvider,
  ]);

  const selectLauncherRow = useCallback((row: LauncherEligibilityRow) => {
    setSelectedLauncherRowId(row.id);
    setLauncherLaunchError("");
    setLauncherLaunchResult(null);
  }, []);

  const submitLauncherRequest = useCallback(async (row: LauncherEligibilityRow) => {
    const accessPlan = launcherAccessPlans[row.id];
    if (!accessPlan || launcherLaunchLoadingId) return;
    const sessionName = newSessionName.trim();
    if (!sessionName) {
      setLauncherLaunchError("A workspace name is required before launch.");
      return;
    }
    setLauncherLaunchLoadingId(row.id);
    setLauncherLaunchError("");
    setLauncherLaunchResult(null);
    try {
      const request = createLauncherServiceRequestFromAccessPlan(accessPlan, {
        url: launcherTargetUrl.trim() || undefined,
        displayIsolation: launcherDisplayIsolation,
        viewStreamProvider: launcherViewStreamProvider,
        controlInputProvider: launcherControlInputProvider,
        jobTimeoutMs: 60000,
      });
      const args = createLauncherSessionArgsFromAccessPlan(accessPlan, {
        sessionName,
        url: launcherTargetUrl.trim() || undefined,
        displayIsolation: launcherDisplayIsolation,
        viewStreamProvider: launcherViewStreamProvider,
        controlInputProvider: launcherControlInputProvider,
        executableId: row.executableId,
        browserHostId: row.browserHostId,
        jobTimeoutMs: 60000,
      });
      const result = await execCommand(args);
      if (!result.success) {
        throw new Error(result.stderr || result.stdout || "Workspace launch process failed.");
      }
      const launchedBrowserId = `session:${sessionName}`;
      const identity = extractServiceRequestWorkspaceIdentity({
        browserId: launchedBrowserId,
        sessionId: sessionName,
        profileId: stringCandidate(request.runtimeProfile) ?? row.profileId,
      });
      setLauncherLaunchResult({ jobId: identity.jobId, action: request.action, request });
      const freshStatus = await fetchServiceStatus();
      const browser = freshStatus?.service_state?.browsers?.[launchedBrowserId] ??
        Object.values(freshStatus?.service_state?.browsers ?? {}).find((candidate) =>
          candidate.activeSessionIds?.includes(sessionName),
        ) ??
        null;
      const stream = launchViewStream(browser);
      setNewSessionOpen(false);
      setNewSessionName("");
      if (browser && stream?.url) {
        const mode = stream.readOnly === true || !stream.controlInput ? "view" : "control";
        const selection = pushWorkspaceViewportSelectionUrl(
          selectionForLaunchedBrowser(browser, freshStatus, identity),
          mode,
        );
        if (selection) setUrlSelection(selection);
      } else {
        pushServiceJobsView(identity.jobId);
      }
    } catch (err) {
      setLauncherLaunchError(err instanceof Error ? err.message : "Service launch request failed");
    } finally {
      setLauncherLaunchLoadingId(null);
    }
  }, [
    activePort,
    fetchServiceStatus,
    launcherAccessPlans,
    launcherControlInputProvider,
    launcherDisplayIsolation,
    launcherLaunchLoadingId,
    launcherTargetUrl,
    launcherViewStreamProvider,
    newSessionName,
    setNewSessionOpen,
  ]);

  const selectNode = useCallback((node: WorkspaceNode, options: {
    persistUrl?: boolean;
    historyMode?: "push" | "replace";
    focusDaemon?: boolean;
    openViewport?: boolean;
  } = {}) => {
    setSelectedNodeId(node.id);
    const daemonName = node.daemonSession ?? node.relatedIds.daemonSessionNames[0] ?? node.relatedIds.serviceSessionIds[0];
    const daemonSession = daemonName ? sessions.find((session) => session.session === daemonName) : undefined;
    if (options.focusDaemon !== false && daemonSession?.port) {
      setActivePort(daemonSession.port);
      if (node.primaryTab?.id && node.source === "daemon-session") {
        const tabIndex = Number(node.primaryTab.id);
        if (Number.isFinite(tabIndex)) {
          dispatchSwitchTab({ port: daemonSession.port, tabIndex });
        }
      }
    }
    if (options.persistUrl !== false) {
      setUrlSelection(writeDashboardWorkspaceUrlSelection(
        workspaceUrlSelectionForNode(node),
        options.historyMode ?? "push",
      ));
    }
    if (options.openViewport !== false && node.viewStream?.embeddable) {
      const selection = pushWorkspaceViewportUrl(node, node.viewStream.controllable ? "control" : "view");
      if (selection) setUrlSelection(selection);
    }
  }, [dispatchSwitchTab, sessions, setActivePort]);

  useEffect(() => {
    if (!dashboardWorkspaceSelectionHasValue(urlSelection)) return;
    let bestNode: WorkspaceNode | null = null;
    let bestScore = 0;
    for (const node of nodes) {
      const score = workspaceUrlSelectionScore(node, urlSelection);
      if (score > bestScore) {
        bestNode = node;
        bestScore = score;
      }
    }
    if (bestNode && bestNode.id !== selectedNodeId) {
      selectNode(bestNode, {
        focusDaemon: false,
        persistUrl: false,
        openViewport: false,
      });
      if (bestNode.id !== urlSelection.workspaceId) {
        setUrlSelection(updateDashboardWorkspaceUrlSelection({ workspaceId: bestNode.id }, "replace"));
      }
    } else if (bestNode && bestNode.id === selectedNodeId && bestNode.id !== urlSelection.workspaceId) {
      setUrlSelection(updateDashboardWorkspaceUrlSelection({ workspaceId: bestNode.id }, "replace"));
    }
  }, [nodes, selectNode, selectedNodeId, urlSelection]);

  useEffect(() => {
    if (!selectedNodeId) {
      lastScrolledSelectionRef.current = null;
      return;
    }
    if (lastScrolledSelectionRef.current === selectedNodeId) return;
    const selectedRow = document.querySelector(".workspace-nav-row-selected");
    selectedRow?.scrollIntoView({ block: "center", inline: "nearest" });
    lastScrolledSelectionRef.current = selectedNodeId;
  }, [selectedNodeId]);

  const performNodeAction = useCallback((node: WorkspaceNode, actionId: WorkspaceNodeActionId) => {
    const action = node.actions.find((candidate) => candidate.id === actionId);
    if (!action?.enabled) return;
    if (action.id === "add-tab" && node.port) {
      dispatchAddTab(node.port);
      return;
    }
    if (action.id === "control" && node.viewStream?.controllable) {
      const selection = pushWorkspaceViewportUrl(node, "control");
      if (selection) setUrlSelection(selection);
      return;
    }
    if (action.id === "view" && node.viewStream?.embeddable) {
      const selection = pushWorkspaceViewportUrl(node, "view");
      if (selection) setUrlSelection(selection);
      return;
    }
    if (action.id === "external-open" && node.viewStream?.url) {
      window.open(node.viewStream.url, "_blank", "noopener,noreferrer");
      return;
    }
    selectNode(node);
  }, [dispatchAddTab, selectNode]);

  const performPrimaryAction = useCallback((node: WorkspaceNode) => {
    const action = primaryAction(node);
    if (!action) return;
    performNodeAction(node, action.id);
  }, [performNodeAction]);

  const dismissAttentionNode = useCallback((node: WorkspaceNode) => {
    if (node.group !== "needs-attention") return;
    setDismissedAttentionIds((current) => {
      const next = new Set(current);
      next.add(node.id);
      writeDismissedAttentionIds(next);
      return next;
    });
  }, []);

  const restoreDismissedAttention = useCallback(() => {
    const next = new Set<string>();
    writeDismissedAttentionIds(next);
    setDismissedAttentionIds(next);
  }, []);

  const handleCreateSubmit = useCallback(async () => {
    const name = newSessionName.trim();
    if (!name || creating) return;
    setCreating(true);
    setCreateError("");
    const option = BROWSER_OPTIONS.find((browser) => browser.id === newSessionBrowser);
    const error = await dispatchCreateSession({
      name,
      engine: option?.engine ?? "chrome",
      provider: option?.provider,
    });
    setCreating(false);
    if (error) {
      setCreateError(error);
      return;
    }
    setNewSessionName("");
    setNewSessionOpen(false);
  }, [creating, dispatchCreateSession, newSessionBrowser, newSessionName, setNewSessionOpen]);

  const confirmDangerAction = useCallback(() => {
    if (!pendingDangerAction) return;
    if (pendingDangerAction.type === "close-all") {
      dispatchCloseAllSessions();
      setPendingDangerAction(null);
      return;
    }
    const port = pendingDangerAction.node?.port;
    if (port && pendingDangerAction.type === "close") dispatchCloseSession(port);
    if (port && pendingDangerAction.type === "kill") dispatchKillSession(port);
    setPendingDangerAction(null);
  }, [dispatchCloseAllSessions, dispatchCloseSession, dispatchKillSession, pendingDangerAction]);

  const sessionBackedNodes = nodes.filter((node) => node.source === "daemon-session");
  const retainedDefaultOpen = scope === "retained" || Boolean(query.trim());
  const attentionDefaultOpen = scope === "needs-attention" ||
    Boolean(query.trim()) ||
    grouped["needs-attention"].some((node) => node.id === selectedNodeId);

  return (
    <div className="workspace-nav flex h-full flex-col">
      <div className="workspace-nav-header">
        <div className="min-w-0">
          <div className="workspace-nav-title">Workspaces</div>
          <div className="workspace-nav-subtitle">
            {nodes.length} derived from service state and sessions
          </div>
        </div>
        <div className="ml-auto flex items-center gap-1">
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                type="button"
                className="workspace-nav-toolbar-button"
                onClick={pushWorkspaceTileUrl}
                aria-label="Open tiled workspace view"
              >
                <MonitorDot className="size-3.5" />
              </button>
            </TooltipTrigger>
            <TooltipContent>Tile live remote workspaces</TooltipContent>
          </Tooltip>
          {sessionBackedNodes.length > 0 && (
            <Tooltip>
              <TooltipTrigger asChild>
                <button
                  type="button"
                  className="workspace-nav-toolbar-button"
                  onClick={() => setPendingDangerAction({ type: "close-all" })}
                  aria-label="Close all sessions"
                >
                  <Trash2 className="size-3.5" />
                </button>
              </TooltipTrigger>
              <TooltipContent>Close all daemon sessions</TooltipContent>
            </Tooltip>
          )}
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                type="button"
                className="workspace-nav-toolbar-button workspace-nav-toolbar-primary"
                onClick={() => setNewSessionOpen(true)}
                aria-label="New workspace"
              >
                <Plus className="size-3.5" />
              </button>
            </TooltipTrigger>
            <TooltipContent>New browser workspace</TooltipContent>
          </Tooltip>
        </div>
      </div>
      <Separator />
      <div className="workspace-nav-controls">
        <label className="workspace-nav-search">
          <Search className="size-3.5" />
          <span className="sr-only">Filter workspaces</span>
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Filter workspaces"
          />
          {query && (
            <button type="button" onClick={() => setQuery("")} aria-label="Clear workspace filter">
              <X className="size-3" />
            </button>
          )}
        </label>
        <div className="workspace-nav-scope" role="tablist" aria-label="Workspace scope">
          {(["all", "active", "needs-attention", "retained"] as WorkspaceScope[]).map((value) => (
            <button
              key={value}
              type="button"
              role="tab"
              aria-selected={scope === value}
              className={cn(scope === value && "workspace-nav-scope-active")}
              onClick={() => setScope(value)}
            >
              <span>{SCOPE_LABELS[value]}</span>
              <small>{value === "all" ? nodes.length : counts[value].length}</small>
            </button>
          ))}
        </div>
        {dismissedAttentionCount > 0 && (
          <button
            type="button"
            className="workspace-nav-dismissed-restore"
            onClick={restoreDismissedAttention}
          >
            Restore {dismissedAttentionCount} dismissed
          </button>
        )}
      </div>
      {selectedNode && (
        <WorkspaceNodeDetail
          node={selectedNode}
          onAction={performNodeAction}
        />
      )}
      <div className="workspace-nav-scroll min-h-0 flex-1" role="region" aria-label="Workspace list">
        <div className="workspace-nav-body">
          {serviceError && !loadedOnceRef.current && (
            <div className="workspace-nav-empty workspace-nav-error">
              {serviceError}
            </div>
          )}
          {filteredNodes.length === 0 ? (
            <div className="workspace-nav-empty">
              No workspaces match the current filter.
            </div>
          ) : (
            <>
              <WorkspaceGroup
                title="Active"
                nodes={grouped.active}
                selectedNodeId={selectedNodeId}
                onSelect={selectNode}
                onPrimaryAction={performPrimaryAction}
                onClose={(node) => setPendingDangerAction({ type: "close", node })}
                onKill={(node) => setPendingDangerAction({ type: "kill", node })}
                rowWindow={WORKSPACE_ACTIVE_ROW_WINDOW}
              />
              <WorkspaceGroup
                title="Needs attention"
                nodes={grouped["needs-attention"]}
                selectedNodeId={selectedNodeId}
                defaultOpen={attentionDefaultOpen}
                rowWindow={WORKSPACE_ATTENTION_ROW_WINDOW}
                onSelect={selectNode}
                onPrimaryAction={performPrimaryAction}
                onClose={(node) => setPendingDangerAction({ type: "close", node })}
                onKill={(node) => setPendingDangerAction({ type: "kill", node })}
                onDismiss={dismissAttentionNode}
              />
              <WorkspaceGroup
                title="Retained"
                nodes={grouped.retained}
                selectedNodeId={selectedNodeId}
                defaultOpen={retainedDefaultOpen}
                rowWindow={WORKSPACE_RETAINED_ROW_WINDOW}
                onSelect={selectNode}
                onPrimaryAction={performPrimaryAction}
                onClose={(node) => setPendingDangerAction({ type: "close", node })}
                onKill={(node) => setPendingDangerAction({ type: "kill", node })}
              />
            </>
          )}
        </div>
      </div>

      <Dialog open={newSessionOpen} onOpenChange={setNewSessionOpen}>
        <DialogContent className="sm:max-w-xl">
          <DialogHeader>
            <DialogTitle>New workspace</DialogTitle>
            <DialogDescription className="sr-only">
              Choose a local session or plan a service-owned browser and profile launch.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <label className="grid gap-1.5 text-xs font-medium">
              Name
              <input
                value={newSessionName}
                onChange={(event) => setNewSessionName(event.target.value)}
                className="workspace-nav-dialog-input"
                onKeyDown={(event) => {
                  if (event.key === "Enter") void handleCreateSubmit();
                }}
              />
            </label>
            <label className="grid gap-1.5 text-xs font-medium">
              Browser
              <select
                value={newSessionBrowser}
                onChange={(event) => setNewSessionBrowser(event.target.value)}
                className="workspace-nav-dialog-input"
              >
                {BROWSER_OPTIONS.map((browser) => (
                  <option key={browser.id} value={browser.id}>
                    {browser.label}
                  </option>
                ))}
              </select>
            </label>
            <LauncherEligibilityPanel
              preview={launcherPreview}
              planningRowId={launcherPlanLoadingId}
              planError={launcherPlanError}
              selectedRowId={selectedLauncherRowId}
              selectedAccessPlan={selectedLauncherAccessPlan}
              targetUrl={launcherTargetUrl}
              displayIsolation={launcherDisplayIsolation}
              viewStreamProvider={launcherViewStreamProvider}
              controlInputProvider={launcherControlInputProvider}
              launchingRowId={launcherLaunchLoadingId}
              launchError={launcherLaunchError}
              launchResult={launcherLaunchResult}
              onSelect={selectLauncherRow}
              onPlan={(row) => void fetchLauncherAccessPlan(row)}
              onTargetUrlChange={setLauncherTargetUrl}
              onDisplayIsolationChange={setLauncherDisplayIsolation}
              onViewStreamProviderChange={setLauncherViewStreamProvider}
              onControlInputProviderChange={setLauncherControlInputProvider}
              onLaunch={(row) => void submitLauncherRequest(row)}
            />
            {createError && (
              <p className="rounded-md border border-destructive/30 bg-destructive/10 px-2 py-1.5 text-xs text-destructive">
                {createError}
              </p>
            )}
          </div>
          <DialogFooter>
            <Button variant="ghost" size="sm" onClick={() => setNewSessionOpen(false)}>
              Cancel
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => void handleCreateSubmit()}
              disabled={creating || !newSessionName.trim()}
            >
              {creating && <Loader2 className="size-3.5 animate-spin" />}
              Create local
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <AlertDialog open={Boolean(pendingDangerAction)} onOpenChange={(open) => !open && setPendingDangerAction(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {pendingDangerAction?.type === "kill" ? "Kill workspace" :
                pendingDangerAction?.type === "close-all" ? "Close all workspaces" : "Close workspace"}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {pendingDangerAction?.type === "close-all"
                ? "Close every live daemon session shown in the workspace navigator."
                : `Apply this action to ${pendingDangerAction?.node?.label ?? "the selected workspace"}.`}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={confirmDangerAction}>
              Confirm
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
