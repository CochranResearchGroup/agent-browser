import type { SessionInfo, TabInfo } from "../types.ts";
import type { DashboardWorkspaceUrlSelection } from "./workspace-url-selection.ts";
import {
  deriveWorkspaceNodes,
  type WorkspaceNode,
  type WorkspaceNodeAction,
  type WorkspaceInventoryClass,
  type WorkspaceNodeInput,
  type WorkspaceNodeOwnership,
  type WorkspaceNodePrimaryTab,
  type WorkspaceNodeProcess,
  type WorkspaceResourceRecord,
  type WorkspaceNodeRole,
  type WorkspaceNodeState,
  type WorkspaceNodeViewStream,
  type WorkspaceOwnershipDiagnostic,
  type WorkspaceServiceBrowser,
  type WorkspaceServiceIncident,
  type WorkspaceServiceJob,
  type WorkspaceServiceProfileAllocation,
  type WorkspaceServiceSession,
  type WorkspaceServiceTab,
} from "./service-workspaces.ts";

export type SelectedWorkspaceSource =
  | "service-browser"
  | "service-session"
  | "daemon-session"
  | "profile"
  | "none";

export type SelectedWorkspaceState = WorkspaceNodeState | "none" | "missing";

export type SelectedWorkspaceRuntime = WorkspaceNodeProcess & {
  lastFrameAt?: number | null;
};

export type SelectedWorkspaceEvidenceRow = {
  label: string;
  value: string;
};

export type SelectedWorkspaceEvidence = {
  summary: string;
  rows: SelectedWorkspaceEvidenceRow[];
};

export type SelectedWorkspaceContext = {
  selection: DashboardWorkspaceUrlSelection;
  node: WorkspaceNode | null;
  source: SelectedWorkspaceSource;
  label: string;
  state: SelectedWorkspaceState;
  inventoryClass: WorkspaceInventoryClass | "none";
  role: WorkspaceNodeRole | "none";
  roleReason: string | null;
  missingReason: string | null;
  live: boolean;
  retained: boolean;
  viewable: boolean;
  controllable: boolean;
  browser: WorkspaceServiceBrowser | null;
  daemonSession: SessionInfo | null;
  serviceSessions: WorkspaceServiceSession[];
  tabs: WorkspaceServiceTab[];
  primaryTab: WorkspaceNodePrimaryTab | null;
  profileAllocation: WorkspaceServiceProfileAllocation | null;
  jobs: WorkspaceServiceJob[];
  incidents: WorkspaceServiceIncident[];
  resources: WorkspaceResourceRecord[];
  stream: WorkspaceNodeViewStream | null;
  runtime: SelectedWorkspaceRuntime;
  ownership: WorkspaceNodeOwnership;
  actions: WorkspaceNodeAction[];
  diagnostics: WorkspaceOwnershipDiagnostic[];
  evidence: SelectedWorkspaceEvidence;
  refreshedAt: number;
};

export type SelectedWorkspaceContextInput = WorkspaceNodeInput & {
  selection: DashboardWorkspaceUrlSelection;
  nodes?: WorkspaceNode[];
  lastFrameAtByStreamPort?: Record<number, number>;
  refreshedAt?: number;
};

const EMPTY_RUNTIME: SelectedWorkspaceRuntime = {
  pid: null,
  running: null,
  rssBytes: null,
  cpuSeconds: null,
  cdpPort: null,
  streamPort: null,
  lastFrameAt: null,
};

export function buildSelectedWorkspaceContext(input: SelectedWorkspaceContextInput): SelectedWorkspaceContext {
  const selection = input.selection;
  const nodes = input.nodes ?? deriveWorkspaceNodes({ ...input, includeRetained: true, includeHidden: true });
  const node = selectWorkspaceNode(selection, nodes);
  const serviceBrowsers = input.serviceBrowsers ?? [];
  const serviceSessions = input.serviceSessions ?? [];
  const serviceTabs = input.serviceTabs ?? [];
  const profileAllocations = input.profileAllocations ?? [];
  const jobs = input.jobs ?? [];
  const incidents = input.incidents ?? [];
  const resources = input.resources ?? [];
  const daemonSessions = input.daemonSessions ?? [];
  const refreshedAt = input.refreshedAt ?? Date.now();
  const source: SelectedWorkspaceSource = node?.source ?? "none";
  const browser = node?.browserId
    ? serviceBrowsers.find((candidate) => candidate.id === node.browserId) ?? null
    : null;
  const daemonSession = findDaemonSession({ selection, node, daemonSessions });
  const relatedSessionIds = new Set(node?.relatedIds.serviceSessionIds ?? []);
  if (node?.serviceSessionId) relatedSessionIds.add(node.serviceSessionId);
  const relatedTabIds = new Set(node?.relatedIds.tabIds ?? []);
  const relatedProfileIds = new Set(node?.relatedIds.profileIds ?? []);
  if (node?.profileId) relatedProfileIds.add(node.profileId);
  const relatedJobIds = new Set(node?.relatedIds.jobIds ?? []);
  const relatedIncidentIds = new Set(node?.relatedIds.incidentIds ?? []);
  const contextSessions = serviceSessions.filter((session) => relatedSessionIds.has(session.id));
  const contextTabs = serviceTabs.filter((tab) =>
    relatedTabIds.has(tab.id) ||
    Boolean(node?.browserId && tab.browserId === node.browserId) ||
    Boolean(node?.serviceSessionId && (tab.sessionId === node.serviceSessionId || tab.ownerSessionId === node.serviceSessionId)),
  );
  const profileAllocation = profileAllocations.find((allocation) =>
    relatedProfileIds.has(allocation.profileId) ||
    Boolean(node?.browserId && allocation.browserIds?.includes(node.browserId)) ||
    Boolean(node?.serviceSessionId && allocation.holderSessionIds?.includes(node.serviceSessionId)),
  ) ?? null;
  const contextJobs = jobs.filter((job) => relatedJobIds.has(job.id));
  const contextIncidents = incidents.filter((incident) => relatedIncidentIds.has(incident.id));
  const runtime = selectedRuntime(node?.process ?? null, daemonSession, input.lastFrameAtByStreamPort);
  const contextResources = resources.filter((resource) =>
    resourceMatchesSelection(resource, {
      node,
      browser,
      profileAllocation,
      serviceSessions: contextSessions,
      runtime,
    }),
  );
  const missingReason = node ? null : missingSelectionReason(selection);
  const evidence = selectedWorkspaceEvidence({
    selection,
    node,
    browser,
    daemonSession,
    serviceSessions: contextSessions,
    tabs: contextTabs,
    profileAllocation,
    jobs: contextJobs,
    incidents: contextIncidents,
    resources: contextResources,
    runtime,
  });

  return {
    selection,
    node,
    source,
    label: node?.label ?? (missingReason ? "Workspace not found" : "No workspace selected"),
    state: node?.state ?? (missingReason ? "missing" : "none"),
    inventoryClass: node?.inventoryClass ?? "none",
    role: node?.role ?? "none",
    roleReason: node?.roleReason ?? null,
    missingReason,
    live: node?.live ?? false,
    retained: node?.retained ?? false,
    viewable: Boolean(node?.viewStream?.embeddable || node?.viewStream?.url),
    controllable: Boolean(node?.viewStream?.controllable),
    browser,
    daemonSession,
    serviceSessions: contextSessions,
    tabs: contextTabs,
    primaryTab: node?.primaryTab ?? null,
    profileAllocation,
    jobs: contextJobs,
    incidents: contextIncidents,
    resources: contextResources,
    stream: node?.viewStream ?? null,
    runtime,
    ownership: node?.ownership ?? {},
    actions: node?.actions ?? [],
    diagnostics: node?.diagnostics ?? [],
    evidence,
    refreshedAt,
  };
}

export function selectWorkspaceNode(
  selection: DashboardWorkspaceUrlSelection,
  nodes: WorkspaceNode[],
): WorkspaceNode | null {
  const directBrowserId = bareId(selection.browserId, "browser");
  const workspaceBrowserId = prefixedId(selection.workspaceId, "browser");
  const browserId = directBrowserId || workspaceBrowserId;
  if (browserId) {
    const node = nodes.find((candidate) => candidate.browserId === browserId || candidate.relatedIds.browserIds.includes(browserId));
    if (node) return node;
  }

  const workspaceDaemonSessionId = prefixedId(selection.workspaceId, "daemon-session");
  if (workspaceDaemonSessionId) {
    const node = nodes.find((candidate) =>
      candidate.daemonSession === workspaceDaemonSessionId ||
      candidate.relatedIds.daemonSessionNames.includes(workspaceDaemonSessionId),
    );
    if (node) return node;
  }

  const directSessionId = bareId(selection.sessionId, "session");
  const workspaceSessionId = prefixedId(selection.workspaceId, "session");
  const sessionId = directSessionId || workspaceSessionId;
  if (sessionId) {
    const node = nodes.find((candidate) =>
      candidate.serviceSessionId === sessionId ||
      candidate.daemonSession === sessionId ||
      candidate.relatedIds.serviceSessionIds.includes(sessionId) ||
      candidate.relatedIds.daemonSessionNames.includes(sessionId),
    );
    if (node) return node;
  }

  const profileId = bareId(selection.profileId, "profile") || prefixedId(selection.workspaceId, "profile");
  if (profileId) {
    const node = nodes.find((candidate) => candidate.profileId === profileId || candidate.relatedIds.profileIds.includes(profileId));
    if (node) return node;
  }

  const tabId = bareId(selection.tabId, "target") || bareId(selection.tabId, "tab") || prefixedId(selection.workspaceId, "target") || prefixedId(selection.workspaceId, "tab");
  if (tabId) {
    const node = nodes.find((candidate) => candidate.relatedIds.tabIds.includes(tabId) || candidate.primaryTab?.id === tabId || candidate.primaryTab?.targetId === tabId);
    if (node) return node;
  }

  const jobId = bareId(selection.jobId, "job") || prefixedId(selection.workspaceId, "job");
  if (jobId) {
    const node = nodes.find((candidate) => candidate.relatedIds.jobIds.includes(jobId));
    if (node) return node;
  }

  if (selection.workspaceId) {
    const node = nodes.find((candidate) => candidate.id === selection.workspaceId);
    if (node) return node;
  }

  return null;
}

export function selectedWorkspaceDiagnosticBundle(context: SelectedWorkspaceContext): Record<string, unknown> {
  return {
    selection: context.selection,
    workspace: {
      id: context.node?.id ?? null,
      source: context.source,
      label: context.label,
      state: context.state,
      inventoryClass: context.inventoryClass,
      role: context.role,
      roleReason: context.roleReason,
      health: context.node?.health ?? null,
      missingReason: context.missingReason,
      live: context.live,
      retained: context.retained,
      viewable: context.viewable,
      controllable: context.controllable,
    },
    ids: {
      browserId: context.node?.browserId ?? context.browser?.id ?? null,
      daemonSession: context.daemonSession?.session ?? context.node?.daemonSession ?? null,
      serviceSessionIds: context.serviceSessions.map((session) => session.id),
      tabIds: context.tabs.map((tab) => tab.id),
      profileId: context.node?.profileId ?? context.profileAllocation?.profileId ?? null,
      jobIds: context.jobs.map((job) => job.id),
      incidentIds: context.incidents.map((incident) => incident.id),
    },
    runtime: context.runtime,
    stream: context.stream
      ? {
          provider: context.stream.provider ?? null,
          url: context.stream.url ?? null,
          routeId: context.stream.routeId ?? null,
          routeSource: context.stream.routeSource ?? null,
          embeddable: context.stream.embeddable,
          controllable: context.stream.controllable,
          readOnly: context.stream.readOnly,
          controlInput: context.stream.controlInput ?? null,
          routeSummary: context.stream.routeSummary,
        }
      : null,
    primaryTab: context.primaryTab,
    resources: context.resources,
    ownership: context.ownership,
    diagnostics: context.diagnostics,
    evidence: context.evidence.rows,
    refreshedAt: context.refreshedAt,
  };
}

export function selectedWorkspaceChatSummary(context: SelectedWorkspaceContext): string {
  const facts = [
    `${context.label} (${context.state})`,
    context.node?.browserId ? `browser ${context.node.browserId}` : null,
    context.daemonSession?.session ? `session ${context.daemonSession.session}` : context.node?.serviceSessionId ? `session ${context.node.serviceSessionId}` : null,
    context.primaryTab?.url ? `page ${context.primaryTab.url}` : null,
    context.stream?.provider ? `stream ${context.stream.provider}` : null,
    context.runtime.pid ? `pid ${context.runtime.pid}` : null,
  ].filter(Boolean);
  return facts.join(" | ");
}

export function formatSelectedWorkspaceRuntimeValue(value: number | boolean | null | undefined, unit?: "bytes" | "seconds"): string {
  if (value === null || value === undefined) return "unknown";
  if (typeof value === "boolean") return value ? "yes" : "no";
  if (unit === "bytes") return formatBytes(value);
  if (unit === "seconds") return `${value.toFixed(1)}s`;
  return String(value);
}

function findDaemonSession({
  selection,
  node,
  daemonSessions,
}: {
  selection: DashboardWorkspaceUrlSelection;
  node: WorkspaceNode | null;
  daemonSessions: SessionInfo[];
}): SessionInfo | null {
  const sessionName = node?.daemonSession ||
    node?.relatedIds.daemonSessionNames[0] ||
    bareId(selection.sessionId, "session") ||
    prefixedId(selection.workspaceId, "daemon-session");
  if (!sessionName) return null;
  return daemonSessions.find((session) => session.session === sessionName) ?? null;
}

function selectedRuntime(
  process: WorkspaceNodeProcess | null,
  daemonSession: SessionInfo | null,
  lastFrameAtByStreamPort: Record<number, number> | undefined,
): SelectedWorkspaceRuntime {
  const runtime = { ...EMPTY_RUNTIME, ...(process ?? {}) };
  const daemonPort = daemonSession?.port ?? null;
  if (!runtime.streamPort && daemonPort != null && daemonPort > 0) {
    runtime.streamPort = daemonPort;
  }
  if (runtime.streamPort && lastFrameAtByStreamPort?.[runtime.streamPort]) {
    runtime.lastFrameAt = lastFrameAtByStreamPort[runtime.streamPort];
  }
  return runtime;
}

function selectedWorkspaceEvidence(input: {
  selection: DashboardWorkspaceUrlSelection;
  node: WorkspaceNode | null;
  browser: WorkspaceServiceBrowser | null;
  daemonSession: SessionInfo | null;
  serviceSessions: WorkspaceServiceSession[];
  tabs: WorkspaceServiceTab[];
  profileAllocation: WorkspaceServiceProfileAllocation | null;
  jobs: WorkspaceServiceJob[];
  incidents: WorkspaceServiceIncident[];
  resources: WorkspaceResourceRecord[];
  runtime: SelectedWorkspaceRuntime;
}): SelectedWorkspaceEvidence {
  const rows: SelectedWorkspaceEvidenceRow[] = [
    row("Selection", selectionLabel(input.selection)),
    row("Workspace", input.node?.id),
    row("Source", input.node?.source),
    row("Inventory class", input.node?.inventoryClass),
    row("Role", input.node?.role),
    row("Role reason", input.node?.roleReason),
    row("State", input.node?.state),
    row("Health", input.node?.health),
    row("Browser", input.browser?.id ?? input.node?.browserId),
    row("Daemon session", input.daemonSession?.session ?? input.node?.daemonSession),
    row("Service sessions", input.serviceSessions.map((session) => session.id).join(", ")),
    row("Profile", input.profileAllocation?.profileId ?? input.node?.profileId),
    row("Tabs", input.tabs.length ? String(input.tabs.length) : input.node?.counts.tabs == null ? null : String(input.node.counts.tabs)),
    row("PID", input.runtime.pid == null ? null : String(input.runtime.pid)),
    row("CDP port", input.runtime.cdpPort == null ? null : String(input.runtime.cdpPort)),
    row("Stream port", input.runtime.streamPort == null ? null : String(input.runtime.streamPort)),
    row("Jobs", input.jobs.length ? String(input.jobs.length) : null),
    row("Incidents", input.incidents.length ? String(input.incidents.length) : null),
    row("Resource candidates", resourceCount(input.resources, "candidate")),
    row("Protected resources", resourceCount(input.resources, "protected")),
    row("Resource reasons", resourceReasons(input.resources)),
  ].filter((candidate) => candidate.value !== "unknown");
  const summary = input.node
    ? `${input.node.label} is ${input.node.state}${input.node.attentionReason ? `: ${input.node.attentionReason}` : ""}`
    : selectionHasValue(input.selection)
      ? "The selected workspace no longer maps to live service or daemon state."
      : "No workspace is selected.";
  return { summary, rows };
}

function resourceMatchesSelection(resource: WorkspaceResourceRecord, input: {
  node: WorkspaceNode | null;
  browser: WorkspaceServiceBrowser | null;
  profileAllocation: WorkspaceServiceProfileAllocation | null;
  serviceSessions: WorkspaceServiceSession[];
  runtime: SelectedWorkspaceRuntime;
}): boolean {
  const correlation = resource.correlation ?? {};
  if (input.runtime.pid != null && resource.pid === input.runtime.pid) return true;
  if (input.node?.browserId && correlation.browserId === input.node.browserId) return true;
  if (input.browser?.id && correlation.browserId === input.browser.id) return true;
  if (input.node?.profileId && correlation.profileId === input.node.profileId) return true;
  if (input.profileAllocation?.profileId && correlation.profileId === input.profileAllocation.profileId) return true;
  const sessionIds = new Set(correlation.sessionIds ?? []);
  return input.serviceSessions.some((session) => sessionIds.has(session.id));
}

function resourceCount(resources: WorkspaceResourceRecord[], disposition: string): string | null {
  const count = resources.filter((resource) => resource.disposition === disposition).length;
  return count > 0 ? String(count) : null;
}

function resourceReasons(resources: WorkspaceResourceRecord[]): string | null {
  const reasons = Array.from(new Set(resources.flatMap((resource) => resource.reasons ?? [])));
  return reasons.length > 0 ? reasons.slice(0, 4).join(", ") : null;
}

function row(label: string, value: unknown): SelectedWorkspaceEvidenceRow {
  return { label, value: stringifyValue(value) };
}

function selectionLabel(selection: DashboardWorkspaceUrlSelection): string {
  const parts = [
    selection.workspaceId ? `workspace=${selection.workspaceId}` : null,
    selection.browserId ? `browser=${selection.browserId}` : null,
    selection.sessionId ? `session=${selection.sessionId}` : null,
    selection.tabId ? `tab=${selection.tabId}` : null,
    selection.profileId ? `profile=${selection.profileId}` : null,
    selection.jobId ? `job=${selection.jobId}` : null,
  ].filter(Boolean);
  return parts.join(" ");
}

function selectionHasValue(selection: DashboardWorkspaceUrlSelection): boolean {
  return Boolean(selection.workspaceId || selection.browserId || selection.sessionId || selection.tabId || selection.profileId || selection.jobId);
}

function missingSelectionReason(selection: DashboardWorkspaceUrlSelection): string | null {
  if (!selectionHasValue(selection)) return null;
  return "Selection is stale or the browser is no longer reported by the service.";
}

function prefixedId(value: string | null | undefined, prefix: string): string | null {
  const expected = `${prefix}:`;
  return value?.startsWith(expected) ? value.slice(expected.length) : null;
}

function bareId(value: string | null | undefined, prefix: string): string | null {
  if (!value) return null;
  return prefixedId(value, prefix) ?? value;
}

function stringifyValue(value: unknown): string {
  if (value === null || value === undefined || value === "") return "unknown";
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return JSON.stringify(value);
}

function formatBytes(value: number): string {
  if (!Number.isFinite(value) || value < 0) return "unknown";
  if (value < 1024) return `${value} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let next = value / 1024;
  let unit = units[0];
  for (let index = 1; index < units.length && next >= 1024; index += 1) {
    next /= 1024;
    unit = units[index];
  }
  return `${next.toFixed(next >= 10 ? 0 : 1)} ${unit}`;
}
