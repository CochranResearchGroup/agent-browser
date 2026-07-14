import type { SessionInfo, TabInfo } from "../types.ts";
import {
  canOpenControlViewStream,
  canOpenViewStream,
  type ServiceViewStream as DashboardServiceViewStream,
} from "./service-view-streams.ts";

export type WorkspaceNodeState =
  | "active"
  | "busy"
  | "needs-attention"
  | "blocked"
  | "retained"
  | "view-only"
  | "controllable";

export type WorkspaceNodeGroup = "active" | "needs-attention" | "detected" | "retained";

export type WorkspaceInventoryLane = "primary" | "detected" | "launcher" | "retained" | "attention" | "hidden";

export type WorkspaceInventoryPlacement = {
  lane: WorkspaceInventoryLane;
  reason: string;
  rank: number;
};

export type WorkspaceInventoryClass =
  | "service-owned-controllable-browser"
  | "service-owned-view-only-browser"
  | "service-owned-diagnostic-browser"
  | "detected-non-owned-browser"
  | "viewer-client"
  | "retained-history"
  | "service-owned-session"
  | "service-profile-action";

export type WorkspaceNodeSource =
  | "service-browser"
  | "service-session"
  | "daemon-session"
  | "profile";

export type WorkspaceNodeRole = "target-browser" | "viewer-client";

export type WorkspaceNodeActionId =
  | "focus"
  | "inspect"
  | "stream"
  | "screenshot"
  | "view"
  | "control"
  | "launch"
  | "seed"
  | "resume"
  | "close"
  | "kill"
  | "add-tab"
  | "repair"
  | "borrow-control"
  | "copy-link"
  | "external-open";

export type WorkspaceNodeAction = {
  id: WorkspaceNodeActionId;
  label: string;
  enabled: boolean;
  reason?: string | null;
};

export type WorkspaceProfileActionabilityAction =
  | "openSharedProfileTab"
  | "reuseCompatibleTab"
  | "waitForProfileHolder"
  | "takeOverViewer"
  | "routeSwitch"
  | "launchNewBrowser"
  | "rejectDuplicateProcess";

export type WorkspaceProfileActionability = {
  recommendedAction: WorkspaceProfileActionabilityAction;
  enabled: boolean;
  reason: string;
  profileId?: string | null;
  ownerBrowserId?: string | null;
  ownerSessionIds: string[];
  activeTabIds: string[];
  browserBuild?: string | null;
  requestedBrowserBuild?: string | null;
  routeId?: string | null;
  displayAllocationId?: string | null;
};

export type WorkspaceNodeOwnership = {
  serviceName?: string | null;
  agentName?: string | null;
  taskName?: string | null;
};

export type WorkspaceNodePrimaryTab = {
  id: string;
  targetId?: string | null;
  title?: string | null;
  url?: string | null;
  lifecycle?: string | null;
  active?: boolean;
};

export type WorkspaceNodeViewStream = {
  provider?: string | null;
  url?: string | null;
  routeId?: string | null;
  displayAllocationId?: string | null;
  routePoolEntryId?: string | null;
  connectionId?: string | null;
  connectionName?: string | null;
  routeSource?: string | null;
  providerMode?: string | null;
  viewerLeaseIds?: string[];
  controllerLeaseId?: string | null;
  embeddable: boolean;
  controllable: boolean;
  readOnly: boolean;
  controlInput?: string | null;
  operatorVisibleState: string;
  operatorVisibleReason?: string | null;
  routeSummary: string;
};

export type WorkspaceRouteBoundOwnershipState =
  | "finalized"
  | "pending"
  | "rolled-back"
  | "diagnostic"
  | "retained"
  | "viewer-client";

export type WorkspaceRouteBoundOwnership = {
  state: WorkspaceRouteBoundOwnershipState;
  routeId?: string | null;
  displayAllocationId?: string | null;
  routePoolEntryId?: string | null;
  reason: string | null;
};

export type WorkspaceOwnershipDiagnosticKind =
  | "duplicate-cdp-endpoint"
  | "duplicate-display"
  | "duplicate-guacamole-route"
  | "duplicate-target"
  | "stale-retained-target"
  | "viewer-client-target"
  | "idle-route-display";

export type WorkspaceOwnershipDiagnostic = {
  kind: WorkspaceOwnershipDiagnosticKind;
  severity: "info" | "warning";
  message: string;
  relatedIds: string[];
};

export type WorkspaceNodeTakeover = {
  active: boolean;
  sessionId: string;
  ownerLabel: string;
  startedAt?: string | null;
  lastObservedAt?: string | null;
  expiresAt?: string | null;
  cleanup?: string | null;
  profileLeaseDisposition?: string | null;
  conflictSessionIds: string[];
  waitingJobIds: string[];
  queueImpact: string;
  resumeSupported: boolean;
  resumeReason: string;
};

export type WorkspaceNode = {
  id: string;
  source: WorkspaceNodeSource;
  role: WorkspaceNodeRole;
  roleReason?: string | null;
  group: WorkspaceNodeGroup;
  inventoryClass: WorkspaceInventoryClass;
  state: WorkspaceNodeState;
  label: string;
  secondaryLabel: string;
  sortLabel: string;
  health?: string | null;
  attentionReason?: string | null;
  retained: boolean;
  live: boolean;
  browserId?: string | null;
  serviceSessionId?: string | null;
  daemonSession?: string | null;
  port?: number | null;
  profileId?: string | null;
  browserBuild?: string | null;
  host?: string | null;
  process?: WorkspaceNodeProcess | null;
  ownership: WorkspaceNodeOwnership;
  primaryTab?: WorkspaceNodePrimaryTab | null;
  viewStream?: WorkspaceNodeViewStream | null;
  routeBoundOwnership?: WorkspaceRouteBoundOwnership | null;
  profileActionability?: WorkspaceProfileActionability | null;
  takeover?: WorkspaceNodeTakeover | null;
  diagnostics: WorkspaceOwnershipDiagnostic[];
  counts: {
    tabs: number;
    serviceSessions: number;
    jobs: number;
    incidents: number;
  };
  relatedIds: {
    browserIds: string[];
    serviceSessionIds: string[];
    daemonSessionNames: string[];
    tabIds: string[];
    profileIds: string[];
    jobIds: string[];
    incidentIds: string[];
  };
  actions: WorkspaceNodeAction[];
  inventoryPlacement?: WorkspaceInventoryPlacement;
};

type WorkspaceNodeWithPlacement = WorkspaceNode & {
  inventoryPlacement: WorkspaceInventoryPlacement;
};

export type WorkspaceNodeProcess = {
  pid?: number | null;
  running?: boolean | null;
  rssBytes?: number | null;
  cpuSeconds?: number | null;
  cdpPort?: number | null;
  streamPort?: number | null;
};

export type WorkspaceNodeLiveControlEligibility = {
  state: "controllable" | "view-only" | "not-controllable";
  canView: boolean;
  canControl: boolean;
  reason: string | null;
};

export type WorkspaceServiceBrowser = {
  id: string;
  profileId?: string | null;
  host?: string | null;
  health?: string | null;
  browserBuild?: string | null;
  displayName?: string | null;
  pid?: number | null;
  cdpEndpoint?: string | null;
  displayAllocationId?: string | null;
  processStats?: {
    pid?: number | null;
    running?: boolean | null;
    rssBytes?: number | null;
    cpuSeconds?: number | null;
  } | null;
  viewStreams?: WorkspaceServiceViewStream[];
  attachability?: unknown;
  activeSessionIds?: string[];
  lastError?: string | null;
};

export type WorkspaceServiceViewStream = {
  id?: string | null;
  provider?: string | null;
  url?: string | null;
  frameUrl?: string | null;
  externalUrl?: string | null;
  routeId?: string | null;
  displayAllocationId?: string | null;
  routePoolEntryId?: string | null;
  connectionId?: string | null;
  connectionName?: string | null;
  routeSource?: string | null;
  providerMode?: string | null;
  viewerLeaseIds?: string[];
  controllerLeaseId?: string | null;
  readiness?: unknown;
  remoteReadiness?: unknown;
  attachability?: unknown;
  displayContent?: unknown;
  readOnly?: boolean | null;
  controlInput?: string | null;
  routeBoundOwnership?: Partial<WorkspaceRouteBoundOwnership> | null;
};

export type WorkspaceServiceSession = {
  id: string;
  owner?: unknown;
  profileId?: string | null;
  serviceName?: string | null;
  agentName?: string | null;
  taskName?: string | null;
  browserIds?: string[];
  tabIds?: string[];
  lease?: string | null;
  cleanup?: string | null;
  profileLeaseDisposition?: string | null;
  profileLeaseConflictSessionIds?: string[];
  createdAt?: string | null;
  lastLeaseObservedAt?: string | null;
  expiresAt?: string | null;
};

export type WorkspaceServiceTab = {
  id: string;
  browserId?: string | null;
  targetId?: string | null;
  sessionId?: string | null;
  lifecycle?: string | null;
  url?: string | null;
  title?: string | null;
  ownerSessionId?: string | null;
};

export type WorkspaceServiceProfileAllocation = {
  profileId: string;
  profileName?: string | null;
  browserBuild?: string | null;
  serviceNames?: string[];
  agentNames?: string[];
  taskNames?: string[];
  browserIds?: string[];
  tabIds?: string[];
  holderSessionIds?: string[];
  exclusiveHolderSessionIds?: string[];
  waitingJobIds?: string[];
  conflictSessionIds?: string[];
  leaseState?: string | null;
  recommendedAction?: string | null;
  targetReadiness?: WorkspaceProfileTargetReadiness[];
  browserSummaries?: Array<{
    browserId?: string | null;
    health?: string | null;
    activeSessionIds?: string[];
  }>;
};

export type WorkspaceProfileTargetReadiness = {
  targetServiceId?: string | null;
  loginId?: string | null;
  state?: string | null;
  manualSeedingRequired?: boolean | null;
  recommendedAction?: string | null;
  evidence?: string | null;
};

export type WorkspaceServiceJob = {
  id: string;
  action?: string | null;
  state?: string | null;
  serviceName?: string | null;
  agentName?: string | null;
  taskName?: string | null;
  target?: unknown;
  request?: unknown;
  response?: unknown;
  result?: unknown;
  error?: string | null;
};

export type WorkspaceServiceIncident = {
  id: string;
  browserId?: string | null;
  label?: string | null;
  severity?: string | null;
  escalation?: string | null;
  recommendedAction?: string | null;
  latestMessage?: string | null;
  currentHealth?: string | null;
  acknowledgedAt?: string | null;
  resolvedAt?: string | null;
  jobIds?: string[];
};

export type WorkspaceResourceRecord = {
  pid?: number | null;
  kind?: string | null;
  disposition?: string | null;
  reasons?: string[];
  rssBytes?: number | null;
  gcAction?: string | null;
  correlation?: {
    browserId?: string | null;
    profileId?: string | null;
    sessionIds?: string[];
    displayAllocationId?: string | null;
    displayName?: string | null;
    cdpPort?: number | null;
    profilePath?: string | null;
  };
};

export type WorkspaceBrowserSessionAuthorityVerdict = {
  key?: string | null;
  browserId?: string | null;
  state?: string | null;
  viable?: boolean | null;
  needsAttention?: boolean | null;
  reasons?: string[];
};

export type WorkspaceBrowserSessionAuthority = {
  schemaVersion?: number | null;
  browserVerdicts?: WorkspaceBrowserSessionAuthorityVerdict[];
};

export type WorkspaceNodeInput = {
  daemonSessions?: SessionInfo[];
  daemonTabsByPort?: Record<number, TabInfo[]>;
  daemonEngineByPort?: Record<number, string>;
  serviceBrowsers?: WorkspaceServiceBrowser[];
  serviceSessions?: WorkspaceServiceSession[];
  serviceTabs?: WorkspaceServiceTab[];
  profileAllocations?: WorkspaceServiceProfileAllocation[];
  jobs?: WorkspaceServiceJob[];
  incidents?: WorkspaceServiceIncident[];
  resources?: WorkspaceResourceRecord[];
  browserSessionAuthority?: WorkspaceBrowserSessionAuthority | null;
  includeRetained?: boolean;
  includeHidden?: boolean;
};

const TERMINAL_BROWSER_HEALTH = new Set([
  "cdp_disconnected",
  "closed",
  "disconnected",
  "faulted",
  "not_started",
  "process_exited",
  "unreachable",
]);

const POST_TERMINATION_BROWSER_HEALTH = new Set([
  "closed",
  "not_started",
  "process_exited",
  "unreachable",
]);

const ATTENTION_BROWSER_HEALTH = new Set([
  "cdp_disconnected",
  "degraded",
  "disconnected",
  "error",
  "faulted",
  "unreachable",
]);

const ACTIVE_JOB_STATES = new Set([
  "queued",
  "running",
  "waiting",
  "pending",
  "cancelling",
]);

const MANUAL_SEEDING_STATES = new Set([
  "needs_manual_seeding",
  "manual_seeding_required",
  "stale",
  "missing",
]);

const PROFILE_CONFLICT_STATES = new Set([
  "blocked",
  "conflict",
  "exclusive_conflict",
  "lease_conflict",
]);

const INTERNAL_DASHBOARD_SERVICE_NAMES = new Set([
  "agentbrowserdashboard",
]);

export function deriveWorkspaceNodes(input: WorkspaceNodeInput): WorkspaceNode[] {
  const daemonSessions = input.daemonSessions ?? [];
  const daemonTabsByPort = input.daemonTabsByPort ?? {};
  const daemonEngineByPort = input.daemonEngineByPort ?? {};
  const serviceBrowsers = input.serviceBrowsers ?? [];
  const serviceSessions = input.serviceSessions ?? [];
  const serviceTabs = input.serviceTabs ?? [];
  const profileAllocations = input.profileAllocations ?? [];
  const jobs = input.jobs ?? [];
  const incidents = input.incidents ?? [];
  const authorityVerdictsByBrowserId = browserAuthorityVerdictsByBrowserId(input.browserSessionAuthority);
  const ownershipDiagnostics = deriveWorkspaceOwnershipDiagnostics({
    serviceBrowsers,
    serviceSessions,
    serviceTabs,
  });
  const diagnosticsByRelatedId = groupDiagnosticsByRelatedId(ownershipDiagnostics);

  const serviceSessionsByBrowserId = groupByLinkedId(
    serviceSessions,
    (session) => session.browserIds ?? [],
  );
  const serviceTabsByBrowserId = groupByLinkedId(
    serviceTabs,
    (tab) => (tab.browserId ? [tab.browserId] : []),
  );
  const serviceTabsBySessionId = groupByLinkedId(
    serviceTabs,
    (tab) => uniqueStrings([tab.sessionId, tab.ownerSessionId]),
  );
  const serviceSessionById = new Map(serviceSessions.map((session) => [session.id, session]));
  const serviceBrowserByActiveSessionId = new Map<string, WorkspaceServiceBrowser>();
  for (const browser of serviceBrowsers) {
    for (const sessionId of browser.activeSessionIds ?? []) {
      serviceBrowserByActiveSessionId.set(sessionId, browser);
    }
  }
  const profileAllocationById = new Map(
    profileAllocations.map((allocation) => [allocation.profileId, allocation]),
  );
  const browserIdsWithNodes = new Set<string>();
  const serviceSessionIdsWithNodes = new Set<string>();
  const daemonNamesWithNodes = new Set<string>();
  const profileIdsWithNodes = new Set<string>();
  const nodes: WorkspaceNodeWithPlacement[] = [];

  for (const browser of serviceBrowsers) {
    const linkedSessions = uniqueById([
      ...(browser.activeSessionIds ?? [])
        .map((sessionId) => serviceSessionById.get(sessionId))
        .filter(isDefined),
      ...(serviceSessionsByBrowserId.get(browser.id) ?? []),
    ]);
    if (isPostTerminationBrowserHistory(browser)) {
      browserIdsWithNodes.add(browser.id);
      for (const session of linkedSessions) serviceSessionIdsWithNodes.add(session.id);
      continue;
    }
    const tabs = serviceTabsByBrowserId.get(browser.id) ?? [];
    const allocation = browser.profileId
      ? profileAllocationById.get(browser.profileId)
      : undefined;
    const relatedJobs = jobs.filter((job) => jobMatches({
      job,
      browser,
      sessions: linkedSessions,
      allocation,
      tabs,
    }));
    const relatedIncidents = incidents.filter((incident) =>
      incident.browserId === browser.id || (incident.jobIds ?? []).some((jobId) => relatedJobs.some((job) => job.id === jobId)),
    );
    const diagnostics = diagnosticsForRelatedIds(diagnosticsByRelatedId, [
      relatedId("browser", browser.id),
      ...linkedSessions.map((session) => relatedId("session", session.id)),
      ...tabs.map((tab) => relatedId("tab", tab.id)),
    ]);
    const node = createBrowserWorkspaceNode({
      browser,
      sessions: linkedSessions,
      tabs,
      allocation,
      jobs: relatedJobs,
      incidents: relatedIncidents,
      diagnostics,
      authorityVerdict: authorityVerdictsByBrowserId.get(browser.id),
    });
    nodes.push(applyWorkspaceInventoryPlacement(node));
    browserIdsWithNodes.add(browser.id);
    if (browser.profileId) profileIdsWithNodes.add(browser.profileId);
    for (const session of linkedSessions) serviceSessionIdsWithNodes.add(session.id);
  }

  for (const session of serviceSessions) {
    if (serviceSessionIdsWithNodes.has(session.id)) continue;
    const tabs = serviceTabsBySessionId.get(session.id) ?? [];
    const allocation = session.profileId
      ? profileAllocationById.get(session.profileId)
      : undefined;
    const relatedJobs = jobs.filter((job) => jobMatches({ job, sessions: [session], allocation, tabs }));
    const relatedIncidents = incidents.filter((incident) =>
      (incident.jobIds ?? []).some((jobId) => relatedJobs.some((job) => job.id === jobId)),
    );
    const diagnostics = diagnosticsForRelatedIds(diagnosticsByRelatedId, [
      relatedId("session", session.id),
      ...tabs.map((tab) => relatedId("tab", tab.id)),
    ]);
    nodes.push(applyWorkspaceInventoryPlacement(createServiceSessionWorkspaceNode({
      session,
      tabs,
      allocation,
      jobs: relatedJobs,
      incidents: relatedIncidents,
      diagnostics,
    })));
    serviceSessionIdsWithNodes.add(session.id);
    if (session.profileId) profileIdsWithNodes.add(session.profileId);
  }

  for (const session of daemonSessions) {
    const serviceBrowser = serviceBrowserByActiveSessionId.get(session.session);
    if (serviceBrowser && browserIdsWithNodes.has(serviceBrowser.id)) {
      daemonNamesWithNodes.add(session.session);
      continue;
    }
    const serviceSession = serviceSessionById.get(session.session);
    if (serviceSession && serviceSessionIdsWithNodes.has(serviceSession.id)) {
      daemonNamesWithNodes.add(session.session);
      continue;
    }
    const tabs = daemonTabsByPort[session.port] ?? [];
    const node = createDaemonWorkspaceNode({
      session,
      tabs,
      engine: session.engine ?? daemonEngineByPort[session.port],
    });
    const placedNode = applyWorkspaceInventoryPlacement(node);
    if (placedNode.inventoryPlacement.lane !== "hidden" || input.includeHidden) nodes.push(placedNode);
    daemonNamesWithNodes.add(session.session);
  }

  for (const allocation of profileAllocations) {
    if (profileIdsWithNodes.has(allocation.profileId)) continue;
    const relatedJobs = jobs.filter((job) => jobMatches({ job, allocation }));
    const relatedIncidents = incidents.filter((incident) =>
      (incident.jobIds ?? []).some((jobId) => relatedJobs.some((job) => job.id === jobId)),
    );
    nodes.push(applyWorkspaceInventoryPlacement(createProfileWorkspaceNode({ allocation, jobs: relatedJobs, incidents: relatedIncidents })));
    profileIdsWithNodes.add(allocation.profileId);
  }

  return nodes
    .filter((node) => input.includeHidden || node.inventoryPlacement.lane !== "hidden")
    .filter((node) => input.includeRetained || node.inventoryPlacement.lane !== "retained")
    .sort(compareWorkspaceNodes);
}

export function deriveLiveWorkspaceNodes(input: WorkspaceNodeInput): WorkspaceNode[] {
  return deriveWorkspaceNodes(input).filter(isLiveWorkspaceNode);
}

export function isLiveWorkspaceNode(node: WorkspaceNode): boolean {
  const placement = node.inventoryPlacement ?? workspaceInventoryPlacementForNode(node);
  if (placement.lane === "hidden" || placement.lane === "retained") return false;
  if (node.role === "viewer-client" || node.inventoryClass === "viewer-client") return false;
  if (node.group === "active" || node.group === "detected") return node.live && !node.retained;
  if (node.group === "needs-attention") return isViableLiveAttentionNode(node);
  return false;
}

export function workspaceInventoryPlacementForNode(node: WorkspaceNode): WorkspaceInventoryPlacement {
  const recoveryAction = node.actions.find((action) => action.enabled && (
    action.id === "repair" ||
    action.id === "resume" ||
    action.id === "seed"
  ));
  const profileAction = node.source === "profile" && node.profileActionability?.enabled === true;
  const hasRecoveryAction = Boolean(recoveryAction || node.takeover?.active);
  const hasLiveBrowserAuthority = node.live && !node.retained && (
    Boolean(node.browserId) ||
    Boolean(node.serviceSessionId) ||
    Boolean(node.daemonSession) ||
    node.source === "service-browser" ||
    node.source === "daemon-session"
  );
  const hasDetectedViableTarget = node.inventoryClass === "detected-non-owned-browser" &&
    node.live &&
    !node.retained &&
    Boolean(
      node.viewStream?.embeddable ||
        node.viewStream?.url ||
        node.process?.cdpPort ||
        node.process?.pid ||
        node.primaryTab?.url ||
        node.primaryTab?.targetId,
    );
  const retainedHistory = node.inventoryClass === "retained-history" && node.retained && node.source !== "daemon-session";

  if (node.role === "viewer-client" || node.inventoryClass === "viewer-client") {
    return hasRecoveryAction
      ? { lane: "attention", reason: recoveryAction?.reason ?? node.attentionReason ?? "Viewer client row has an operator recovery action.", rank: 500 }
      : { lane: "hidden", reason: node.roleReason ?? "Viewer client rows are not target-browser inventory.", rank: 900 };
  }
  if (hasDetectedViableTarget) {
    return { lane: "detected", reason: "Detected non-owned browser has viable read-only evidence.", rank: 100 };
  }
  if (hasLiveBrowserAuthority && node.group !== "needs-attention") {
    return { lane: "primary", reason: "Live browser authority is viable.", rank: 0 };
  }
  if (hasLiveBrowserAuthority && hasRecoveryAction) {
    return { lane: "attention", reason: node.attentionReason ?? recoveryAction?.reason ?? "Live browser row needs operator attention.", rank: 500 };
  }
  if (node.source === "profile" && (profileAction || hasRecoveryAction)) {
    return { lane: "launcher", reason: node.profileActionability?.reason ?? node.attentionReason ?? "Profile row has an operator action.", rank: 200 };
  }
  if (node.source === "profile" && node.profileActionability && !node.profileActionability.enabled) {
    return { lane: "attention", reason: node.profileActionability.reason ?? node.attentionReason ?? "Profile row needs operator attention before launch.", rank: 500 };
  }
  if (node.group === "needs-attention" && hasRecoveryAction) {
    return { lane: "attention", reason: node.attentionReason ?? recoveryAction?.reason ?? "Workspace row has an operator recovery action.", rank: 500 };
  }
  if (retainedHistory) {
    return { lane: "retained", reason: node.attentionReason ?? "Retained browser history.", rank: 300 };
  }
  if (hasRecoveryAction) {
    return { lane: "attention", reason: node.attentionReason ?? recoveryAction?.reason ?? "Workspace row has an operator recovery action.", rank: 500 };
  }
  return { lane: "hidden", reason: "Workspace row has no live, retained, detected, or actionable inventory authority.", rank: 900 };
}

function applyWorkspaceInventoryPlacement(node: WorkspaceNode): WorkspaceNodeWithPlacement {
  return {
    ...node,
    inventoryPlacement: workspaceInventoryPlacementForNode(node),
  };
}

function isViableLiveAttentionNode(node: WorkspaceNode): boolean {
  if (!node.live || node.retained) return false;
  if (node.role === "viewer-client" || node.inventoryClass === "viewer-client") return false;
  if (node.inventoryClass === "retained-history") return false;
  if (node.source !== "service-browser" && node.source !== "daemon-session") return true;
  return Boolean(
    node.process?.running ||
      node.process?.pid ||
      node.process?.cdpPort ||
      node.process?.streamPort ||
      node.primaryTab?.targetId ||
      node.primaryTab?.url ||
      node.viewStream?.url ||
      node.viewStream?.routeId,
  );
}

export function workspaceNodeLiveControlEligibility(node: WorkspaceNode): WorkspaceNodeLiveControlEligibility {
  const viewAction = node.actions.find((action) => action.id === "view");
  const controlAction = node.actions.find((action) => action.id === "control");
  const reason = node.roleReason
    ?? node.attentionReason
    ?? controlAction?.reason
    ?? viewAction?.reason
    ?? node.viewStream?.operatorVisibleReason
    ?? null;

  if (node.role === "viewer-client" || node.inventoryClass === "viewer-client") {
    return {
      state: "not-controllable",
      canView: false,
      canControl: false,
      reason: reason ?? "Viewer clients cannot become target-browser control rows.",
    };
  }
  if (!node.live || node.retained || node.inventoryClass === "retained-history") {
    return {
      state: "not-controllable",
      canView: false,
      canControl: false,
      reason: reason ?? "Retained workspace records are not live control targets.",
    };
  }
  if (node.routeBoundOwnership && node.routeBoundOwnership.state !== "finalized") {
    return {
      state: "not-controllable",
      canView: Boolean(viewAction?.enabled && node.viewStream?.embeddable),
      canControl: false,
      reason: reason ?? node.routeBoundOwnership.reason ?? "Route-bound browser ownership is not finalized.",
    };
  }
  if (node.inventoryClass === "service-owned-view-only-browser") {
    return {
      state: "view-only",
      canView: Boolean(viewAction?.enabled && node.viewStream?.embeddable),
      canControl: false,
      reason: reason ?? "This workspace is view-only.",
    };
  }
  if (
    node.inventoryClass !== "service-owned-controllable-browser" ||
    !node.viewStream?.controllable ||
    controlAction?.enabled !== true
  ) {
    return {
      state: "not-controllable",
      canView: Boolean(viewAction?.enabled && node.viewStream?.embeddable),
      canControl: false,
      reason: reason ?? "Canonical workspace inventory does not permit live control.",
    };
  }
  return {
    state: "controllable",
    canView: Boolean(viewAction?.enabled && node.viewStream?.embeddable),
    canControl: true,
    reason: null,
  };
}

export function deriveWorkspaceOwnershipDiagnostics(input: Pick<WorkspaceNodeInput, "serviceBrowsers" | "serviceSessions" | "serviceTabs">): WorkspaceOwnershipDiagnostic[] {
  const serviceBrowsers = input.serviceBrowsers ?? [];
  const serviceSessions = input.serviceSessions ?? [];
  const serviceTabs = input.serviceTabs ?? [];
  const sessionByBrowserId = groupByLinkedId(serviceSessions, (session) => session.browserIds ?? []);
  const tabsByBrowserId = groupByLinkedId(serviceTabs, (tab) => (tab.browserId ? [tab.browserId] : []));
  const diagnostics: WorkspaceOwnershipDiagnostic[] = [];

  addDuplicateBrowserDiagnostics({
    diagnostics,
    browsers: serviceBrowsers,
    kind: "duplicate-cdp-endpoint",
    valueFor: (browser) => browser.cdpEndpoint,
    messageFor: (value, browsers) =>
      `Duplicate CDP endpoint ${value} is claimed by ${browsers.map((browser) => browser.id).join(", ")}.`,
  });

  addDuplicateBrowserDiagnostics({
    diagnostics,
    browsers: serviceBrowsers.filter((browser) => browser.host === "remote_headed" || browser.viewStreams?.some((stream) => stream.provider === "rdp_gateway")),
    kind: "duplicate-display",
    valueFor: (browser) => browser.displayName,
    messageFor: (value, browsers) =>
      `Remote display ${value} is claimed by ${browsers.map((browser) => browser.id).join(", ")}; keep shared-display contention explicit or allocate private displays.`,
  });

  addDuplicateBrowserDiagnostics({
    diagnostics,
    browsers: serviceBrowsers.filter((browser) => browser.viewStreams?.some((stream) => stream.provider === "rdp_gateway")),
    kind: "duplicate-guacamole-route",
    valueFor: (browser) => {
      const stream = browser.viewStreams?.find((candidate) => candidate.provider === "rdp_gateway");
      return stream?.routeId || stream?.connectionId || stream?.frameUrl || stream?.externalUrl || stream?.url;
    },
    messageFor: (value, browsers) =>
      `Guacamole route ${value} is shared by ${browsers.map((browser) => browser.id).join(", ")}; viewer ownership and takeover state must be service-visible.`,
  });

  const targetGroups = new Map<string, WorkspaceServiceTab[]>();
  for (const tab of serviceTabs) {
    const targetId = tab.targetId?.trim();
    if (!targetId || !isLiveServiceWorkspaceTab(tab)) continue;
    const group = targetGroups.get(targetId);
    if (group) {
      group.push(tab);
    } else {
      targetGroups.set(targetId, [tab]);
    }
  }
  for (const [targetId, tabs] of targetGroups) {
    const browserIds = uniqueStrings(tabs.map((tab) => tab.browserId));
    if (tabs.length < 2 || browserIds.length < 2) continue;
    diagnostics.push({
      kind: "duplicate-target",
      severity: "warning",
      message: `CDP target ${targetId} is claimed by multiple browser records: ${browserIds.join(", ")}.`,
      relatedIds: uniqueStrings([
        ...tabs.map((tab) => relatedId("tab", tab.id)),
        ...browserIds.map((browserId) => relatedId("browser", browserId)),
        ...tabs.flatMap((tab) => uniqueStrings([tab.sessionId, tab.ownerSessionId]).map((sessionId) => relatedId("session", sessionId))),
      ]),
    });
  }

  for (const browser of serviceBrowsers) {
    if (!isLiveBrowser(browser)) continue;
    const tabs = tabsByBrowserId.get(browser.id) ?? [];
    const liveTabs = tabs.filter((tab) => isLiveServiceWorkspaceTab(tab) && !isBlankServiceWorkspaceTab(tab));
    const staleTabs = tabs.filter((tab) => !isLiveServiceWorkspaceTab(tab) || isBlankServiceWorkspaceTab(tab));
    if (staleTabs.length === 0) continue;
    const sessions = sessionByBrowserId.get(browser.id) ?? [];
    diagnostics.push({
      kind: "stale-retained-target",
      severity: "info",
      message: liveTabs.length > 0
        ? `Retained target identity is stale for ${browser.id}; focus should fall back to a current live tab before marking the stream dead.`
        : `Browser ${browser.id} is live but retained target identity is stale or missing; distinguish target recovery from browser failure.`,
      relatedIds: uniqueStrings([
        relatedId("browser", browser.id),
        ...staleTabs.map((tab) => relatedId("tab", tab.id)),
        ...sessions.map((session) => relatedId("session", session.id)),
      ]),
    });
  }

  return diagnostics;
}

function createBrowserWorkspaceNode({
  browser,
  sessions,
  tabs,
  allocation,
  jobs,
  incidents,
  diagnostics,
  authorityVerdict,
}: {
  browser: WorkspaceServiceBrowser;
  sessions: WorkspaceServiceSession[];
  tabs: WorkspaceServiceTab[];
  allocation?: WorkspaceServiceProfileAllocation;
  jobs: WorkspaceServiceJob[];
  incidents: WorkspaceServiceIncident[];
  diagnostics: WorkspaceOwnershipDiagnostic[];
  authorityVerdict?: WorkspaceBrowserSessionAuthorityVerdict;
}): WorkspaceNode {
  const ownership = firstOwnership(sessions, allocation);
  const primaryTab = primaryServiceTab(tabs);
  const rawViewStream = selectPrimaryWorkspaceViewStream(browser.viewStreams);
  const rawPrimaryViewStream = rawViewStream
    ? cdpSnapshotFallbackViewStream(browser, rawViewStream) ?? primaryViewStream([rawViewStream])
    : null;
  const takeover = takeoverForSessions(sessions, allocation, jobs);
  const busy = jobs.some(isActiveJob);
  const terminal = isTerminalBrowser(browser);
  const diagnosticRelatedIds = uniqueStrings([
    relatedId("browser", browser.id),
    ...sessions.map((session) => relatedId("session", session.id)),
    ...tabs.map((tab) => relatedId("tab", tab.id)),
  ]);
  const viewerClient = classifyViewerClient({
    ids: uniqueStrings([
      browser.id,
      browser.profileId,
      allocation?.profileId,
      ...(browser.activeSessionIds ?? []),
      ...sessions.map((session) => session.id),
    ]),
    ownership,
    tabs,
  });
  const attentionReason = browserAttentionReason(browser, allocation, incidents, {
    includeTerminalHealth: !terminal || busy || incidents.some((incident) => !incident.resolvedAt) || Boolean(takeover),
  });
  const authority = browserAuthorityPlacement(authorityVerdict);
  const live = isLiveBrowser(browser) && authority.live;
  const routeProofDiagnostic = live
    ? idleRouteDisplayDiagnosticFor({
        browser,
        stream: rawViewStream,
        relatedIds: diagnosticRelatedIds,
      })
    : null;
  const disabledStreamReason = !live
    ? "Browser is retained, not live."
    : viewerClient.reason ?? routeProofDiagnostic?.message ?? null;
  const proofGatedViewStream = rawPrimaryViewStream && disabledStreamReason
    ? disableViewStream(rawPrimaryViewStream, disabledStreamReason)
    : rawPrimaryViewStream;
  const routeBoundOwnership = routeBoundOwnershipFor({
    live,
    viewerClient: viewerClient.active,
    stream: rawViewStream,
    viewStream: proofGatedViewStream,
  });
  const routeBoundOwnershipReason = routeBoundOwnershipControlReason(routeBoundOwnership, rawViewStream);
  const viewStream = proofGatedViewStream && routeBoundOwnershipReason
    ? disableViewStream(proofGatedViewStream, routeBoundOwnershipReason)
    : proofGatedViewStream;
  const routeBoundAttentionReason = live ? routeBoundViewStreamAttentionReason(viewStream) : null;
  const viewStreamAttentionReason = live ? disabledViewStreamAttentionReason(viewStream) : null;
  const effectiveAttentionReason = authority.reason ?? routeBoundAttentionReason ?? viewStreamAttentionReason ?? attentionReason;
  const blockedReason = takeover?.queueImpact ?? (terminal ? null : live && viewStream?.controllable ? null : profileBlockedReason(allocation));
  const state = viewerClient.active ? "needs-attention" : workspaceState({
    busy,
    blockedReason,
    attentionReason: effectiveAttentionReason,
    live,
    viewStream,
  });
  const effectiveState = !viewerClient.active && authority.forceAttention ? "needs-attention" : state;
  const viewerClientDiagnostic = viewerClientDiagnosticFor({
    reason: viewerClient.reason,
    relatedIds: diagnosticRelatedIds,
  });
  const nodeDiagnostics = uniqueDiagnostics([
    ...(viewerClientDiagnostic ? [viewerClientDiagnostic] : []),
    ...(routeProofDiagnostic ? [routeProofDiagnostic] : []),
    ...diagnostics,
  ]);
  const label = browserWorkspaceLabel({
    browser,
    sessions,
    ownership,
    primaryTab,
    profileName: allocation?.profileName,
    displayName: browser.displayName,
    fallback: browser.id,
  });
  const profileActionability = profileActionabilityForBrowser({
    browser,
    sessions,
    tabs,
    allocation,
    live,
    viewerClient: viewerClient.active,
    viewStream,
    rawViewStream,
  });
  const secondaryLabel = compactLabels([
    workspaceSessionLabel(sessions[0]?.id ?? browser.activeSessionIds?.[0]),
    browser.host,
    browser.browserBuild,
    allocation?.profileId ?? browser.profileId,
    profileActionabilitySummary(profileActionability),
    viewStream?.routeSummary,
    primaryTab?.title || primaryTab?.url,
    takeover ? `takeover by ${takeover.ownerLabel}` : null,
    viewerClient.active ? "viewer client" : null,
    nodeDiagnostics[0]?.message,
    live ? "live" : "retained",
  ]).join(" / ");

  return {
    id: `browser:${browser.id}`,
    source: "service-browser",
    role: viewerClient.active ? "viewer-client" : "target-browser",
    roleReason: viewerClient.reason,
    group: groupForState(effectiveState),
    inventoryClass: browserInventoryClass({
      viewerClient: viewerClient.active,
      live,
      state: effectiveState,
      viewStream,
    }),
    state: effectiveState,
    label,
    secondaryLabel,
    sortLabel: `${label} ${browser.id}`.toLowerCase(),
    health: browser.health ?? null,
    attentionReason: viewerClient.reason ?? blockedReason ?? effectiveAttentionReason,
    retained: !live,
    live,
    browserId: browser.id,
    serviceSessionId: sessions[0]?.id ?? null,
    profileId: browser.profileId ?? allocation?.profileId ?? null,
    browserBuild: browser.browserBuild ?? allocation?.browserBuild ?? null,
    host: browser.host ?? null,
    process: browserProcessIndicators(browser, viewStream),
    ownership,
    primaryTab,
    viewStream,
    routeBoundOwnership,
    profileActionability,
    takeover,
    diagnostics: nodeDiagnostics,
    counts: {
      tabs: tabs.length,
      serviceSessions: sessions.length,
      jobs: jobs.length,
      incidents: incidents.length,
    },
    relatedIds: {
      browserIds: [browser.id],
      serviceSessionIds: uniqueStrings(sessions.map((session) => session.id)),
      daemonSessionNames: [],
      tabIds: uniqueStrings(tabs.map((tab) => tab.id)),
      profileIds: uniqueStrings([browser.profileId, allocation?.profileId]),
      jobIds: uniqueStrings(jobs.map((job) => job.id)),
      incidentIds: uniqueStrings(incidents.map((incident) => incident.id)),
    },
    actions: viewerClient.active
      ? viewerClientActions(browserActions(browser, live, viewStream, blockedReason ?? effectiveAttentionReason, takeover), viewerClient.reason)
      : browserActions(
          browser,
          live,
          viewStream,
          blockedReason ?? effectiveAttentionReason,
          takeover,
          profileActionability,
        ),
  };
}

function browserAuthorityVerdictsByBrowserId(
  authority?: WorkspaceBrowserSessionAuthority | null,
): Map<string, WorkspaceBrowserSessionAuthorityVerdict> {
  const result = new Map<string, WorkspaceBrowserSessionAuthorityVerdict>();
  for (const verdict of authority?.browserVerdicts ?? []) {
    const browserId = verdict.browserId?.trim() || verdict.key?.trim();
    if (browserId) result.set(browserId, verdict);
  }
  return result;
}

function browserAuthorityPlacement(
  verdict?: WorkspaceBrowserSessionAuthorityVerdict,
): { live: boolean; forceAttention: boolean; reason: string | null } {
  if (!verdict) return { live: true, forceAttention: false, reason: null };
  const state = normalize(verdict.state);
  const reasons = (verdict.reasons ?? []).filter(Boolean);
  const reason = reasons.length > 0
    ? `Browser session authority: ${reasons.join("; ")}.`
    : state
      ? `Browser session authority state is ${state}.`
      : "Browser session authority marked this browser non-viable.";
  if (state === "non_viable" || verdict.viable === false && state !== "attention") {
    return { live: false, forceAttention: false, reason };
  }
  if (state === "attention" || verdict.needsAttention === true) {
    return { live: true, forceAttention: true, reason };
  }
  return { live: true, forceAttention: false, reason: null };
}

function createServiceSessionWorkspaceNode({
  session,
  tabs,
  allocation,
  jobs,
  incidents,
  diagnostics,
}: {
  session: WorkspaceServiceSession;
  tabs: WorkspaceServiceTab[];
  allocation?: WorkspaceServiceProfileAllocation;
  jobs: WorkspaceServiceJob[];
  incidents: WorkspaceServiceIncident[];
  diagnostics: WorkspaceOwnershipDiagnostic[];
}): WorkspaceNode {
  const ownership = ownershipFromSession(session, allocation);
  const primaryTab = primaryServiceTab(tabs);
  const takeover = takeoverForSession(session, allocation, jobs);
  const attentionReason = profileAttentionReason(allocation) ?? unresolvedIncidentReason(incidents);
  const blockedReason = takeover?.queueImpact ?? profileBlockedReason(allocation);
  const busy = jobs.some(isActiveJob);
  const live = (session.browserIds?.length ?? 0) > 0 || (session.tabIds?.length ?? 0) > 0;
  const state = workspaceState({ busy, blockedReason, attentionReason, live });
  const label = workspaceLabel({
    ownership,
    profileName: allocation?.profileName,
    fallback: session.id,
  });

  return {
    id: `service-session:${session.id}`,
    source: "service-session",
    role: "target-browser",
    roleReason: null,
    group: groupForState(state),
    inventoryClass: "service-owned-session",
    state,
    label,
    secondaryLabel: compactLabels([
      allocation?.profileId ?? session.profileId,
      session.lease,
      takeover ? `takeover by ${takeover.ownerLabel}` : null,
      diagnostics[0]?.message,
      primaryTab?.title || primaryTab?.url,
      live ? "live session" : "retained session",
    ]).join(" / "),
    sortLabel: `${label} ${session.id}`.toLowerCase(),
    attentionReason: blockedReason ?? attentionReason,
    retained: !live,
    live,
    serviceSessionId: session.id,
    profileId: session.profileId ?? allocation?.profileId ?? null,
    browserBuild: allocation?.browserBuild ?? null,
    process: null,
    ownership,
    primaryTab,
    viewStream: null,
    takeover,
    diagnostics,
    counts: {
      tabs: tabs.length,
      serviceSessions: 1,
      jobs: jobs.length,
      incidents: incidents.length,
    },
    relatedIds: {
      browserIds: uniqueStrings(session.browserIds ?? []),
      serviceSessionIds: [session.id],
      daemonSessionNames: [],
      tabIds: uniqueStrings([...(session.tabIds ?? []), ...tabs.map((tab) => tab.id)]),
      profileIds: uniqueStrings([session.profileId, allocation?.profileId]),
      jobIds: uniqueStrings(jobs.map((job) => job.id)),
      incidentIds: uniqueStrings(incidents.map((incident) => incident.id)),
    },
    actions: serviceSessionActions(live, blockedReason ?? attentionReason, takeover),
  };
}

function createDaemonWorkspaceNode({
  session,
  tabs,
  engine,
}: {
  session: SessionInfo;
  tabs: TabInfo[];
  engine?: string | null;
}): WorkspaceNode {
  const activeTab = tabs.find((tab) => tab.active) ?? tabs[0];
  const foreignCdp = session.ownership === "foreign_cdp";
  const detectedExternal = session.detected === true || foreignCdp;
  const portRegistered = !session.pending && !session.closing && session.port > 0;
  const hasBrowserEvidence = tabs.length > 0;
  const hasDetectedReachableEvidence = detectedExternal && portRegistered && Boolean(
    session.addressability === "cdp_reachable" ||
      session.cdpPort ||
      session.pid ||
      foreignCdp,
  );
  const live = portRegistered && (hasBrowserEvidence || hasDetectedReachableEvidence);
  const attentionReason = session.closing
    ? "Session is closing."
    : portRegistered && !hasBrowserEvidence && !hasDetectedReachableEvidence
      ? "Daemon stream port is registered but no CDP tab evidence is available."
      : null;
  const viewStream = detectedExternal ? foreignCdpSnapshotViewStream(session, live) : daemonViewStream(session, live);
  const viewerClient = classifyViewerClient({
    ids: [session.session, session.provider, session.engine, engine],
    tabs,
  });
  const state: WorkspaceNodeState = viewerClient.active
    ? "needs-attention"
    : session.pending
      ? "busy"
      : attentionReason
        ? "needs-attention"
        : live
          ? "active"
          : "retained";
  const label = activeTab?.title || session.session;
  const viewerClientDiagnostic = viewerClientDiagnosticFor({
    reason: viewerClient.reason,
    relatedIds: [relatedId("session", session.session)],
  });

  return {
    id: `daemon-session:${session.session}`,
    source: "daemon-session",
    role: viewerClient.active ? "viewer-client" : "target-browser",
    roleReason: viewerClient.reason,
    group: live && !viewerClient.active ? "detected" : groupForState(state),
    inventoryClass: daemonInventoryClass({
      viewerClient: viewerClient.active,
      live,
    }),
    state,
    label,
    secondaryLabel: compactLabels([
      session.session,
      session.provider ?? session.engine ?? engine,
      detectedExternal ? "detected external Chrome" : null,
      foreignCdp ? "foreign CDP" : null,
      detectedExternal && session.addressability === "cdp_reachable" ? "CDP reachable" : null,
      detectedExternal && session.capabilities?.lifecycle === false ? "lifecycle disabled" : null,
      detectedExternal && session.capabilities?.mutateRequiresBorrow ? "borrow required for mutation" : null,
      live && !viewerClient.active ? "not agent-browser service-owned" : null,
      session.profilePath,
      session.cdpPort ? `CDP ${session.cdpPort}` : null,
      portRegistered && !hasBrowserEvidence && !hasDetectedReachableEvidence ? "no CDP tab evidence" : null,
      activeTab?.url,
      viewerClient.active ? "viewer client" : null,
      viewerClient.reason,
    ]).join(" / "),
    sortLabel: `${label} ${session.session}`.toLowerCase(),
    health: session.pending
      ? "pending"
      : session.closing
        ? "closing"
        : portRegistered && !hasBrowserEvidence && !hasDetectedReachableEvidence
          ? "stale-stream"
          : live
            ? "live"
            : "retained",
    attentionReason: viewerClient.reason ?? attentionReason,
    retained: !live,
    live,
    daemonSession: session.session,
    port: session.port,
    process: portRegistered
      ? {
          pid: session.pid ?? null,
          cdpPort: session.cdpPort ?? (detectedExternal ? session.port : null),
          streamPort: detectedExternal ? null : session.port,
          running: live,
        }
      : null,
    ownership: {},
    primaryTab: activeTab
      ? {
          id: String(activeTab.index),
          targetId: activeTab.targetId ?? null,
          title: activeTab.title,
          url: activeTab.url,
          active: activeTab.active,
        }
      : null,
    viewStream,
    takeover: null,
    diagnostics: viewerClientDiagnostic ? [viewerClientDiagnostic] : [],
    counts: {
      tabs: tabs.length,
      serviceSessions: 0,
      jobs: 0,
      incidents: 0,
    },
    relatedIds: {
      browserIds: [],
      serviceSessionIds: [],
      daemonSessionNames: [session.session],
      tabIds: tabs.map((tab) => String(tab.index)),
      profileIds: [],
      jobIds: [],
      incidentIds: [],
    },
    actions: detectedExternal
      ? detectedBrowserActions(live, viewStream)
      : viewerClient.active
        ? viewerClientActions(daemonActions(session, live, viewStream), viewerClient.reason)
        : daemonActions(session, live, viewStream),
  };
}

function daemonViewStream(session: SessionInfo, live: boolean): WorkspaceNodeViewStream | null {
  if (!live || !session.port) return null;
  const streamUrl = `http://127.0.0.1:${session.port}/`;
  return {
    provider: "cdp_screencast",
    url: streamUrl,
    routeId: `daemon:${session.session}`,
    displayAllocationId: null,
    routePoolEntryId: null,
    connectionId: null,
    connectionName: session.session,
    routeSource: "daemon-session",
    providerMode: "single_controller",
    viewerLeaseIds: [],
    controllerLeaseId: null,
    embeddable: true,
    controllable: true,
    readOnly: false,
    controlInput: "cdp_input",
    operatorVisibleState: "ready",
    operatorVisibleReason: null,
    routeSummary: `daemon stream ${session.port} / ready`,
  };
}

function foreignCdpSnapshotViewStream(session: SessionInfo, live: boolean): WorkspaceNodeViewStream | null {
  if (!live || !session.port) return null;
  const streamUrl = `/api/session-screenshot?port=${encodeURIComponent(String(session.port))}`;
  return {
    provider: "cdp_snapshot",
    url: streamUrl,
    routeId: `foreign-cdp:${session.session}`,
    displayAllocationId: null,
    routePoolEntryId: null,
    connectionId: null,
    connectionName: session.session,
    routeSource: "foreign-cdp",
    providerMode: "read_only_snapshot_poll",
    viewerLeaseIds: [],
    controllerLeaseId: null,
    embeddable: true,
    controllable: false,
    readOnly: true,
    controlInput: null,
    operatorVisibleState: "ready",
    operatorVisibleReason: null,
    routeSummary: `foreign CDP snapshot ${session.port} / read-only / ready`,
  };
}

function createProfileWorkspaceNode({
  allocation,
  jobs,
  incidents,
}: {
  allocation: WorkspaceServiceProfileAllocation;
  jobs: WorkspaceServiceJob[];
  incidents: WorkspaceServiceIncident[];
}): WorkspaceNode {
  const ownership = ownershipFromAllocation(allocation);
  const blockedReason = profileBlockedReason(allocation);
  const attentionReason = profileAttentionReason(allocation) ?? unresolvedIncidentReason(incidents);
  const busy = jobs.some(isActiveJob);
  const state = workspaceState({
    busy,
    blockedReason,
    attentionReason,
    live: false,
  });
  const label = allocation.profileName || allocation.profileId;
  const profileActionability = profileActionabilityForAllocation(allocation);

  return {
    id: `profile:${allocation.profileId}`,
    source: "profile",
    role: "target-browser",
    roleReason: null,
    group: groupForState(state),
    inventoryClass: "service-profile-action",
    state,
    label,
    secondaryLabel: compactLabels([
      allocation.browserBuild,
      ownership.serviceName,
      allocation.leaseState,
      profileActionabilitySummary(profileActionability),
      profileReadinessSummary(allocation),
    ]).join(" / "),
    sortLabel: `${label} ${allocation.profileId}`.toLowerCase(),
    attentionReason: blockedReason ?? attentionReason,
    retained: true,
    live: false,
    profileId: allocation.profileId,
    browserBuild: allocation.browserBuild ?? null,
    process: null,
    ownership,
    primaryTab: null,
    viewStream: null,
    profileActionability,
    takeover: null,
    diagnostics: [],
    counts: {
      tabs: allocation.tabIds?.length ?? 0,
      serviceSessions: allocation.holderSessionIds?.length ?? 0,
      jobs: jobs.length,
      incidents: incidents.length,
    },
    relatedIds: {
      browserIds: uniqueStrings([
        ...(allocation.browserIds ?? []),
        ...(allocation.browserSummaries ?? []).map((summary) => summary.browserId),
      ]),
      serviceSessionIds: uniqueStrings(allocation.holderSessionIds ?? []),
      daemonSessionNames: [],
      tabIds: uniqueStrings(allocation.tabIds ?? []),
      profileIds: [allocation.profileId],
      jobIds: uniqueStrings(jobs.map((job) => job.id)),
      incidentIds: uniqueStrings(incidents.map((incident) => incident.id)),
    },
    actions: profileActions(allocation, blockedReason ?? attentionReason),
  };
}

function workspaceState({
  busy,
  blockedReason,
  attentionReason,
  live,
  viewStream,
}: {
  busy: boolean;
  blockedReason?: string | null;
  attentionReason?: string | null;
  live: boolean;
  viewStream?: WorkspaceNodeViewStream | null;
}): WorkspaceNodeState {
  if (live && viewStream?.controllable) return "controllable";
  if (live && viewStream?.embeddable) return "view-only";
  if (blockedReason) return "blocked";
  if (attentionReason) return "needs-attention";
  if (busy) return "busy";
  return live ? "active" : "retained";
}

function groupForState(state: WorkspaceNodeState): WorkspaceNodeGroup {
  if (state === "blocked" || state === "needs-attention") return "needs-attention";
  if (state === "retained") return "retained";
  return "active";
}

function browserInventoryClass({
  viewerClient,
  live,
  state,
  viewStream,
}: {
  viewerClient: boolean;
  live: boolean;
  state: WorkspaceNodeState;
  viewStream?: WorkspaceNodeViewStream | null;
}): WorkspaceInventoryClass {
  if (viewerClient) return "viewer-client";
  if (!live) return "retained-history";
  if (state === "controllable" && viewStream?.controllable) {
    return "service-owned-controllable-browser";
  }
  if (state === "view-only" && viewStream?.embeddable) {
    return "service-owned-view-only-browser";
  }
  return "service-owned-diagnostic-browser";
}

function daemonInventoryClass({
  viewerClient,
  live,
}: {
  viewerClient: boolean;
  live: boolean;
}): WorkspaceInventoryClass {
  if (viewerClient) return "viewer-client";
  if (live) return "detected-non-owned-browser";
  return "retained-history";
}

function isLiveBrowser(browser: WorkspaceServiceBrowser): boolean {
  const health = normalize(browser.health);
  if (isTerminalBrowser(browser)) return false;
  return Boolean(browser.pid || browser.cdpEndpoint || (browser.activeSessionIds?.length ?? 0) > 0 || health === "ready" || health === "healthy");
}

function isTerminalBrowser(browser: WorkspaceServiceBrowser): boolean {
  return TERMINAL_BROWSER_HEALTH.has(normalize(browser.health));
}

function isPostTerminationBrowserHistory(browser: WorkspaceServiceBrowser): boolean {
  return POST_TERMINATION_BROWSER_HEALTH.has(normalize(browser.health));
}

function browserAttentionReason(
  browser: WorkspaceServiceBrowser,
  allocation?: WorkspaceServiceProfileAllocation,
  incidents: WorkspaceServiceIncident[] = [],
  options: { includeTerminalHealth?: boolean } = {},
): string | null {
  const incidentReason = unresolvedIncidentReason(incidents);
  if (incidentReason) return incidentReason;
  const health = normalize(browser.health);
  const terminalHealth = TERMINAL_BROWSER_HEALTH.has(health);
  if ((!terminalHealth || options.includeTerminalHealth) && ATTENTION_BROWSER_HEALTH.has(health)) {
    return browser.lastError || `Browser health is ${browser.health}.`;
  }
  return profileAttentionReason(allocation);
}

function profileAttentionReason(allocation?: WorkspaceServiceProfileAllocation): string | null {
  if (!allocation) return null;
  for (const readiness of allocation.targetReadiness ?? []) {
    const state = normalize(readiness.state);
    if (readiness.manualSeedingRequired || MANUAL_SEEDING_STATES.has(state)) {
      return humanRecommendedAction(readiness.recommendedAction) || "Profile needs manual seeding before automation can proceed.";
    }
  }
  return null;
}

function profileBlockedReason(allocation?: WorkspaceServiceProfileAllocation): string | null {
  if (!allocation) return null;
  const leaseState = normalize(allocation.leaseState);
  if (
    PROFILE_CONFLICT_STATES.has(leaseState) ||
    (allocation.conflictSessionIds?.length ?? 0) > 0 ||
    (allocation.exclusiveHolderSessionIds?.length ?? 0) > 1
  ) {
    return humanRecommendedAction(allocation.recommendedAction) || "Profile has an exclusive lease conflict.";
  }
  return null;
}

function profileActionabilityForBrowser({
  browser,
  sessions,
  tabs,
  allocation,
  live,
  viewerClient,
  viewStream,
  rawViewStream,
}: {
  browser: WorkspaceServiceBrowser;
  sessions: WorkspaceServiceSession[];
  tabs: WorkspaceServiceTab[];
  allocation?: WorkspaceServiceProfileAllocation;
  live: boolean;
  viewerClient: boolean;
  viewStream?: WorkspaceNodeViewStream | null;
  rawViewStream?: WorkspaceServiceViewStream | null;
}): WorkspaceProfileActionability | null {
  const profileId = browser.profileId ?? allocation?.profileId;
  if (!profileId || viewerClient) return null;
  const ownerSessionIds = uniqueStrings([
    ...(browser.activeSessionIds ?? []),
    ...sessions.map((session) => session.id),
    ...(allocation?.holderSessionIds ?? []),
  ]);
  const activeTabIds = uniqueStrings(tabs.filter(isLiveServiceWorkspaceTab).map((tab) => tab.id));
  const requestedBrowserBuild = allocation?.browserBuild ?? null;
  const browserBuild = browser.browserBuild ?? null;
  const buildMismatch = Boolean(
    requestedBrowserBuild &&
    browserBuild &&
    normalize(requestedBrowserBuild) !== normalize(browserBuild),
  );
  const common = {
    profileId,
    ownerBrowserId: browser.id,
    ownerSessionIds,
    activeTabIds,
    browserBuild,
    requestedBrowserBuild,
    routeId: viewStream?.routeId ?? null,
    displayAllocationId: viewStream?.displayAllocationId ?? browser.displayAllocationId ?? null,
  };

  if (buildMismatch) {
    return {
      ...common,
      recommendedAction: "rejectDuplicateProcess",
      enabled: false,
      reason: `Profile ${profileId} is held by ${browser.id}, but the retained browser build is ${browserBuild} and the profile allocation expects ${requestedBrowserBuild}. Do not launch a duplicate process on this profile.`,
    };
  }

  if (live) {
    const attachability = recordFromUnknown(browser.attachability) ?? recordFromUnknown(rawViewStream?.attachability);
    const attachabilityAction = normalize(stringOrNull(
      attachability?.recommendedAction ??
      attachability?.action ??
      attachability?.nextAction,
    ));
    if (
      attachabilityAction === "service_remote_view_route_switch" ||
      attachabilityAction === "route_switch" ||
      attachabilityAction === "switch_route"
    ) {
      return {
        ...common,
        recommendedAction: "routeSwitch",
        enabled: true,
        reason: stringOrNull(attachability?.reason) ??
          `Profile ${profileId} is owned by ${browser.id}, but the route/display evidence recommends switching route before the next operation.`,
      };
    }

    if (viewStream?.controllerLeaseId) {
      return {
        ...common,
        recommendedAction: "takeOverViewer",
        enabled: true,
        reason: `Profile ${profileId} is owned by ${browser.id}, but controller lease ${viewStream.controllerLeaseId} is active; take over the viewer before control.`,
      };
    }

    const hasOwnerRoute = Boolean(browser.cdpEndpoint || ownerSessionIds.length > 0 || browser.pid);
    return {
      ...common,
      recommendedAction: hasOwnerRoute ? "openSharedProfileTab" : "reuseCompatibleTab",
      enabled: hasOwnerRoute,
      reason: hasOwnerRoute
        ? `Profile ${profileId} is already owned by retained browser ${browser.id}; open the next operation as a tab through this owner.`
        : `Profile ${profileId} has this browser row, but no service-owned tab route is available yet.`,
    };
  }

  if (profileBlockedReason(allocation)) {
    return {
      ...common,
      recommendedAction: "waitForProfileHolder",
      enabled: false,
      reason: `Profile ${profileId} is held by an exclusive owner; wait for the holder or release it before launching another process.`,
    };
  }

  return {
    ...common,
    recommendedAction: "launchNewBrowser",
    enabled: true,
    reason: `No live retained browser currently owns profile ${profileId}; launch a new browser when requested.`,
  };
}

function profileActionabilityForAllocation(
  allocation: WorkspaceServiceProfileAllocation,
): WorkspaceProfileActionability | null {
  const ownerSessionIds = uniqueStrings([
    ...(allocation.holderSessionIds ?? []),
    ...(allocation.exclusiveHolderSessionIds ?? []),
    ...(allocation.conflictSessionIds ?? []),
  ]);
  const ownerBrowserIds = uniqueStrings([
    ...(allocation.browserIds ?? []),
    ...(allocation.browserSummaries ?? []).map((summary) => summary.browserId),
  ]);
  const activeTabIds = uniqueStrings(allocation.tabIds ?? []);
  const blockedReason = profileBlockedReason(allocation);

  if (blockedReason) {
    return {
      recommendedAction: ownerSessionIds.length > 0 ? "waitForProfileHolder" : "rejectDuplicateProcess",
      enabled: false,
      reason: ownerSessionIds.length > 0
        ? `Profile ${allocation.profileId} has an exclusive holder but no compatible live browser row in the workspace inventory.`
        : `Profile ${allocation.profileId} is locked without a service-owned holder; inspect the owner before launching a duplicate process.`,
      profileId: allocation.profileId,
      ownerBrowserId: ownerBrowserIds[0] ?? null,
      ownerSessionIds,
      activeTabIds,
      browserBuild: null,
      requestedBrowserBuild: allocation.browserBuild ?? null,
      routeId: null,
      displayAllocationId: null,
    };
  }

  if (profileAttentionReason(allocation)) {
    return {
      recommendedAction: "launchNewBrowser",
      enabled: false,
      reason: `Profile ${allocation.profileId} needs manual readiness work before a browser launch or shared tab can proceed.`,
      profileId: allocation.profileId,
      ownerBrowserId: ownerBrowserIds[0] ?? null,
      ownerSessionIds,
      activeTabIds,
      browserBuild: null,
      requestedBrowserBuild: allocation.browserBuild ?? null,
      routeId: null,
      displayAllocationId: null,
    };
  }

  if ((allocation.targetReadiness?.length ?? 0) === 0) {
    return {
      recommendedAction: "launchNewBrowser",
      enabled: true,
      reason: `Profile ${allocation.profileId} has no live browser row in the workspace inventory; launch a browser when requested.`,
      profileId: allocation.profileId,
      ownerBrowserId: ownerBrowserIds[0] ?? null,
      ownerSessionIds,
      activeTabIds,
      browserBuild: null,
      requestedBrowserBuild: allocation.browserBuild ?? null,
      routeId: null,
      displayAllocationId: null,
    };
  }

  return null;
}

function profileActionabilitySummary(actionability?: WorkspaceProfileActionability | null): string | null {
  if (!actionability) return null;
  const labels: Record<WorkspaceProfileActionabilityAction, string> = {
    openSharedProfileTab: "open tab in retained profile owner",
    reuseCompatibleTab: "reuse compatible retained tab",
    waitForProfileHolder: "wait for profile holder",
    takeOverViewer: "take over viewer",
    routeSwitch: "switch route",
    launchNewBrowser: "launch new browser",
    rejectDuplicateProcess: "duplicate profile process rejected",
  };
  return labels[actionability.recommendedAction];
}

function unresolvedIncidentReason(incidents: WorkspaceServiceIncident[]): string | null {
  const incident = incidents.find((item) => !item.resolvedAt);
  if (!incident) return null;
  return humanRecommendedAction(incident.recommendedAction) || incident.latestMessage || incident.label || "Service incident needs attention.";
}

function humanRecommendedAction(action?: string | null): string | null {
  const value = action?.trim();
  if (!value) return null;
  const actionLabels: Record<string, string> = {
    reuse_holder_or_release_profile: "Use the existing browser, or release the profile when this task is done.",
    release_holder_or_redirect_waiting_jobs: "Release the profile holder, or route waiting jobs to a different browser.",
    reuse_holder_or_release_holder: "Use the existing browser, or release the current holder.",
    reuse_holder_or_create_new_profile: "Use the existing browser, or launch with a different profile.",
  };
  if (actionLabels[value]) return actionLabels[value];
  if (/^[a-z0-9_]+$/.test(value) && value.includes("_")) {
    return value.split("_").filter(Boolean).join(" ");
  }
  return value;
}

function firstOwnership(
  sessions: WorkspaceServiceSession[],
  allocation?: WorkspaceServiceProfileAllocation,
): WorkspaceNodeOwnership {
  const session = sessions.find((item) => item.serviceName || item.agentName || item.taskName);
  return {
    serviceName: session?.serviceName ?? allocation?.serviceNames?.[0] ?? null,
    agentName: session?.agentName ?? allocation?.agentNames?.[0] ?? null,
    taskName: session?.taskName ?? allocation?.taskNames?.[0] ?? null,
  };
}

function ownershipFromSession(
  session: WorkspaceServiceSession,
  allocation?: WorkspaceServiceProfileAllocation,
): WorkspaceNodeOwnership {
  return {
    serviceName: session.serviceName ?? allocation?.serviceNames?.[0] ?? null,
    agentName: session.agentName ?? allocation?.agentNames?.[0] ?? null,
    taskName: session.taskName ?? allocation?.taskNames?.[0] ?? null,
  };
}

function ownershipFromAllocation(allocation: WorkspaceServiceProfileAllocation): WorkspaceNodeOwnership {
  return {
    serviceName: allocation.serviceNames?.[0] ?? null,
    agentName: allocation.agentNames?.[0] ?? null,
    taskName: allocation.taskNames?.[0] ?? null,
  };
}

function workspaceLabel({
  ownership,
  profileName,
  displayName,
  fallback,
}: {
  ownership: WorkspaceNodeOwnership;
  profileName?: string | null;
  displayName?: string | null;
  fallback: string;
}): string {
  return compactLabels([
    ownership.serviceName,
    ownership.taskName,
    ownership.agentName,
    displayName,
    profileName,
    fallback,
  ])[0] ?? fallback;
}

function browserWorkspaceLabel({
  browser,
  sessions,
  ownership,
  primaryTab,
  profileName,
  displayName,
  fallback,
}: {
  browser: WorkspaceServiceBrowser;
  sessions: WorkspaceServiceSession[];
  ownership: WorkspaceNodeOwnership;
  primaryTab?: WorkspaceNodePrimaryTab | null;
  profileName?: string | null;
  displayName?: string | null;
  fallback: string;
}): string {
  const sessionId = sessions[0]?.id ?? browser.activeSessionIds?.[0] ?? sessionNameFromBrowserId(browser.id);
  const shippingLabel = shippingWorkspaceLabel(primaryTab, sessionId);
  if (shippingLabel) return shippingLabel;

  const serviceName = humanOwnershipLabel(ownership.serviceName);
  const taskName = serviceName ? ownership.taskName : null;
  const agentName = serviceName ? ownership.agentName : null;
  return compactLabels([
    serviceName,
    taskName,
    agentName,
    meaningfulPrimaryTabLabel(primaryTab),
    workspaceSessionLabel(sessionId),
    displayName,
    profileName,
    fallback,
  ])[0] ?? fallback;
}

function meaningfulPrimaryTabLabel(tab?: WorkspaceNodePrimaryTab | null): string | null {
  const url = (tab?.url ?? "").trim().toLowerCase();
  const title = (tab?.title ?? "").trim();
  const normalizedTitle = title.toLowerCase();
  const blankUrl = !url || url === "about:blank" || url === "chrome://newtab/";
  const blankTitle = !normalizedTitle || normalizedTitle === "about:blank" || normalizedTitle === "new tab";
  if (!blankTitle) return title;
  if (!blankUrl) return tab?.url ?? null;
  return null;
}

function humanOwnershipLabel(value?: string | null): string | null {
  const trimmed = value?.trim();
  if (!trimmed) return null;
  if (INTERNAL_DASHBOARD_SERVICE_NAMES.has(normalizeIdentifier(trimmed))) return null;
  return trimmed;
}

function normalizeIdentifier(value: string): string {
  return value.toLowerCase().replace(/[^a-z0-9]+/g, "");
}

function sessionNameFromBrowserId(browserId?: string | null): string | null {
  if (!browserId) return null;
  return browserId.startsWith("session:") ? browserId.slice("session:".length) : null;
}

function workspaceSessionLabel(sessionId?: string | null): string | null {
  const id = sessionId?.trim();
  if (!id) return null;
  const normalized = id.toLowerCase();
  if (normalized.startsWith("odollo-carrier-ups") || normalized.startsWith("odollo-ups")) {
    return "Odollo UPS";
  }
  if (normalized.startsWith("odollo-usps")) return "Odollo USPS";
  return id;
}

function shippingWorkspaceLabel(tab: WorkspaceNodePrimaryTab | null | undefined, sessionId?: string | null): string | null {
  const url = tab?.url?.trim();
  const title = tab?.title?.trim();
  const carrier = shippingCarrierFromUrl(url) ?? shippingCarrierFromTitle(title);
  const sessionLabel = workspaceSessionLabel(sessionId);
  if (!carrier && sessionLabel?.startsWith("Odollo ")) return sessionLabel;
  if (!carrier) return null;
  const trackingNumber = trackingNumberFromUrl(url);
  const prefix = sessionLabel?.startsWith("Odollo ") ? sessionLabel : `${carrier} Tracking`;
  return trackingNumber ? `${prefix}: ${trackingNumber}` : prefix;
}

function shippingCarrierFromTitle(title?: string | null): string | null {
  const normalized = title?.toLowerCase() ?? "";
  if (normalized.includes("ups")) return "UPS";
  if (normalized.includes("usps")) return "USPS";
  if (normalized.includes("fedex")) return "FedEx";
  return null;
}

function shippingCarrierFromUrl(url?: string | null): string | null {
  if (!url) return null;
  try {
    const hostname = new URL(url).hostname.toLowerCase();
    if (hostname.includes("ups.com")) return "UPS";
    if (hostname.includes("usps.com")) return "USPS";
    if (hostname.includes("fedex.com")) return "FedEx";
  } catch {
    return null;
  }
  return null;
}

function trackingNumberFromUrl(url?: string | null): string | null {
  if (!url) return null;
  try {
    const parsed = new URL(url);
    for (const key of ["tracknum", "trackingNumber", "trackingnumber", "trackNums"]) {
      const value = parsed.searchParams.get(key)?.trim();
      if (value) return value.split(/[,\s]+/)[0] ?? value;
    }
  } catch {
    return null;
  }
  return null;
}

function isLiveServiceWorkspaceTab(tab: WorkspaceServiceTab): boolean {
  const lifecycle = normalize(tab.lifecycle);
  return lifecycle === "active" || lifecycle === "ready" || lifecycle === "loading";
}

function isBlankServiceWorkspaceTab(tab: WorkspaceServiceTab): boolean {
  const url = (tab.url ?? "").trim().toLowerCase();
  const title = (tab.title ?? "").trim().toLowerCase();
  const blankUrl = !url || url === "about:blank" || url === "chrome://newtab/";
  const blankTitle = !title || title === "about:blank" || title === "new tab";
  return blankUrl && blankTitle;
}

type ViewerClientClassification = {
  active: boolean;
  reason: string | null;
};

function classifyViewerClient(input: {
  ids?: Array<string | null | undefined>;
  ownership?: WorkspaceNodeOwnership;
  tabs?: Array<WorkspaceServiceTab | TabInfo>;
}): ViewerClientClassification {
  const haystack = [
    ...(input.ids ?? []),
    input.ownership?.serviceName,
    input.ownership?.agentName,
    input.ownership?.taskName,
    ...(input.tabs ?? []).flatMap((tab) => [tab.title, tab.url]),
  ]
    .filter((value): value is string => typeof value === "string" && value.trim().length > 0)
    .map((value) => value.toLowerCase());

  if (haystack.some((value) => /(?:^|[^a-z0-9])dashboard-viewer(?:[^a-z0-9]|$)/.test(value))) {
    return {
      active: true,
      reason: "This is an Agent Browser dashboard viewer client, not the browser target behind the remote display.",
    };
  }
  if (haystack.some((value) => value.includes("/guacamole/#/client") || value.includes("guacamole client"))) {
    return {
      active: true,
      reason: "This is a Guacamole viewer client; remote control must target the browser attached inside that route display.",
    };
  }
  const hasAgentBrowserChrome = haystack.some((value) =>
    value.includes("agent-browser") ||
    value.includes("agent browser"),
  );
  const hasWorkspaceControlRoute = haystack.some((value) =>
    value.includes("workspace%3acontrol") ||
    value.includes("view=workspace") ||
    (value.includes("workspace") && value.includes("control")),
  );
  if (hasAgentBrowserChrome && hasWorkspaceControlRoute) {
    return {
      active: true,
      reason: "This browser is showing the Agent Browser control workspace, so it is a viewer client rather than an inspected target.",
    };
  }
  return { active: false, reason: null };
}

function viewerClientDiagnosticFor(input: {
  reason: string | null;
  relatedIds: string[];
}): WorkspaceOwnershipDiagnostic | null {
  if (!input.reason) return null;
  return {
    kind: "viewer-client-target",
    severity: "warning",
    message: `${input.reason} Do not use this row as evidence that the target browser is visible in the remote route.`,
    relatedIds: input.relatedIds,
  };
}

function idleRouteDisplayDiagnosticFor(input: {
  browser: WorkspaceServiceBrowser;
  stream: WorkspaceServiceViewStream | null;
  relatedIds: string[];
}): WorkspaceOwnershipDiagnostic | null {
  const reason = idleRouteDisplayReason(input.browser, input.stream);
  if (!reason) return null;
  return {
    kind: "idle-route-display",
    severity: "warning",
    message: reason,
    relatedIds: input.relatedIds,
  };
}

function idleRouteDisplayReason(browser: WorkspaceServiceBrowser, stream?: WorkspaceServiceViewStream | null): string | null {
  if (!stream || normalize(stream.provider) !== "rdp_gateway") return null;
  if (browser.host === "remote_headed" && !hasRouteDisplayBinding(stream)) {
    return compactLabels([
      "Remote-headed browser has no service-owned Guacamole route or display binding.",
      browser.displayName ? `Recorded browser display is ${browser.displayName}.` : null,
      "The projected route may show a shared terminal desktop instead of this browser.",
    ]).join(" ");
  }
  const proof = routeProofState(stream);
  if (proof.state === "route_bound_proof_missing") {
    return "Remote route operator-visible proof missing; run route-bound open or focus proof before using the Guacamole stream.";
  }
  const explicitState = readinessState(stream.remoteReadiness ?? stream.readiness);
  const normalizedState = normalize(explicitState);
  if (["idle_display", "terminal_only", "no_browser_window", "display_idle"].includes(normalizedState)) {
    return "Remote route display is idle or terminal-only; launch or focus the target browser on this display before using the Guacamole stream.";
  }

  const values = deepStringValues([stream.remoteReadiness, stream.readiness, stream.displayContent])
    .map((value) => value.toLowerCase());
  if (values.length === 0) return null;
  const hasTerminalWindow = values.some((value) =>
    value.includes("xterm") ||
    value.includes("linux terminal") ||
    value.includes("terminal-only") ||
    value.includes("terminal only") ||
    value.includes("shell window"),
  );
  const hasBrowserWindow = values.some((value) =>
    value.includes("chromium") ||
    value.includes("google chrome") ||
    value.includes("chrome browser") ||
    value.includes("firefox") ||
    value.includes("browser window"),
  );
  if (hasTerminalWindow && !hasBrowserWindow) {
    return "Remote route display appears to contain only terminal windows; launch or focus the target browser on this display before using the Guacamole stream.";
  }
  return null;
}

function hasRouteDisplayBinding(stream: WorkspaceServiceViewStream): boolean {
  const streamUrl = stream.frameUrl || stream.url || stream.externalUrl;
  return Boolean(
    stream.routeId?.trim() ||
    stream.displayAllocationId?.trim() ||
    stream.connectionId?.trim() ||
    stream.connectionName?.trim() ||
    stream.routeSource?.trim() ||
    isSpecificGuacamoleClientUrl(streamUrl),
  );
}

function isSpecificGuacamoleClientUrl(value?: string | null): boolean {
  return Boolean(value && /\/guacamole\/#\/client\/[^/?#]+/i.test(value));
}

function disableViewStream(
  stream: WorkspaceNodeViewStream,
  reason: string,
): WorkspaceNodeViewStream {
  return {
    ...stream,
    url: null,
    embeddable: false,
    controllable: false,
    readOnly: true,
    controlInput: null,
    operatorVisibleState: stream.operatorVisibleState === "ready" ? "disabled" : stream.operatorVisibleState,
    operatorVisibleReason: reason,
    routeSummary: reason,
  };
}

function routeBoundViewStreamAttentionReason(stream?: WorkspaceNodeViewStream | null): string | null {
  if (!stream || normalize(stream.provider) !== "rdp_gateway") return null;
  if (stream.operatorVisibleState === "ready") return null;
  return stream.operatorVisibleReason || stream.routeSummary || "Route-bound operator-visible proof is not ready.";
}

function disabledViewStreamAttentionReason(stream?: WorkspaceNodeViewStream | null): string | null {
  if (!stream || stream.embeddable) return null;
  return stream.operatorVisibleReason || stream.routeSummary || "View stream readiness is not openable.";
}

function viewStreamBlockedReason(stream: WorkspaceServiceViewStream): string | null {
  const readiness = stream.remoteReadiness ?? stream.readiness;
  const state = readinessState(readiness);
  const normalizedState = normalize(state);
  if (!state || normalizedState === "ready" || normalizedState === "unknown" || normalizedState === "probing") {
    return null;
  }
  const reason = readinessReason(readiness);
  return reason
    ? `View stream readiness is ${state.replaceAll("_", " ")}: ${reason}.`
    : `View stream readiness is ${state.replaceAll("_", " ")}.`;
}

function readinessReason(readiness: unknown): string | null {
  if (!readiness || typeof readiness !== "object") return null;
  if (Array.isArray(readiness)) {
    for (const item of readiness) {
      const state = readinessState(item);
      if (state && normalize(state) !== "ready") {
        return readinessReason(item);
      }
    }
    return null;
  }
  const record = recordFromUnknown(readiness);
  return stringOrNull(record?.reason ?? record?.message ?? record?.lastProviderEvent);
}

function deepStringValues(value: unknown): string[] {
  if (typeof value === "string") return [value];
  if (typeof value === "number" || typeof value === "boolean" || value == null) return [];
  if (Array.isArray(value)) return value.flatMap((item) => deepStringValues(item));
  if (typeof value === "object") {
    return Object.values(value as Record<string, unknown>).flatMap((item) => deepStringValues(item));
  }
  return [];
}

function uniqueDiagnostics(diagnostics: WorkspaceOwnershipDiagnostic[]): WorkspaceOwnershipDiagnostic[] {
  const seen = new Set<string>();
  const result: WorkspaceOwnershipDiagnostic[] = [];
  for (const diagnostic of diagnostics) {
    const key = `${diagnostic.kind}\n${diagnostic.message}`;
    if (seen.has(key)) continue;
    seen.add(key);
    result.push(diagnostic);
  }
  return result;
}

function viewerClientActions(actions: WorkspaceNodeAction[], reason: string | null): WorkspaceNodeAction[] {
  const disabledReason = reason ?? "This row is a viewer client, not a browser target.";
  const targetActionIds = new Set<WorkspaceNodeActionId>(["focus", "view", "control", "add-tab", "repair", "external-open"]);
  return actions.map((action) =>
    targetActionIds.has(action.id)
      ? { ...action, enabled: false, reason: disabledReason }
      : action,
  );
}

function serviceWorkspaceTabScore(tab: WorkspaceServiceTab): number {
  if (!isLiveServiceWorkspaceTab(tab)) return -1000;
  const lifecycle = normalize(tab.lifecycle);
  let score = lifecycle === "active" ? 400 : lifecycle === "loading" ? 320 : 300;
  if (!isBlankServiceWorkspaceTab(tab)) score += 200;
  if (tab.targetId) score += 25;
  return score;
}

function primaryServiceTab(tabs: WorkspaceServiceTab[]): WorkspaceNodePrimaryTab | null {
  const inspectableTabs = tabs.filter((tab) => isLiveServiceWorkspaceTab(tab) || !isBlankServiceWorkspaceTab(tab));
  const tab = [...inspectableTabs].sort((left, right) => serviceWorkspaceTabScore(right) - serviceWorkspaceTabScore(left))[0];
  if (!tab) return null;
  return {
    id: tab.id,
    targetId: tab.targetId ?? null,
    title: tab.title ?? null,
    url: tab.url ?? null,
    lifecycle: tab.lifecycle ?? null,
    active: normalize(tab.lifecycle) === "active",
  };
}

function primaryViewStream(streams?: WorkspaceServiceViewStream[]): WorkspaceNodeViewStream | null {
  const stream = selectPrimaryWorkspaceViewStream(streams);
  if (!stream) return null;
  const operatorVisible = routeProofState(stream);
  const readOnly = stream.readOnly === true || !stream.controlInput;
  const routeProofReady = operatorVisible.state === "ready";
  const routeProofRequired = normalize(stream.provider) === "rdp_gateway";
  const streamUrl = stream.frameUrl || stream.url || stream.externalUrl || null;
  const blockedStreamReason = viewStreamBlockedReason(stream);
  const readinessCheckedStream: DashboardServiceViewStream = {
    id: stream.id ?? undefined,
    provider: stream.provider ?? undefined,
    controlInput: stream.controlInput ?? null,
    url: streamUrl,
    frameUrl: streamUrl,
    externalUrl: stream.externalUrl ?? null,
    routeId: stream.routeId ?? null,
    displayAllocationId: stream.displayAllocationId ?? null,
    connectionId: stream.connectionId ?? null,
    connectionName: stream.connectionName ?? null,
    routeSource: stream.routeSource ?? null,
    providerMode: stream.providerMode ?? null,
    viewerLeaseIds: stream.viewerLeaseIds,
    controllerLeaseId: stream.controllerLeaseId ?? null,
    readOnly: stream.readOnly ?? undefined,
    readiness: stream.readiness,
    remoteReadiness: stream.remoteReadiness,
    attachability: stream.attachability,
  };
  const canOpenView = canOpenViewStream(readinessCheckedStream);
  const effectiveOperatorVisible = operatorVisible.state === "ready" && !canOpenView
    ? {
        state: readinessState(stream.remoteReadiness ?? stream.readiness) ?? "unavailable",
        reason: blockedStreamReason,
      }
    : operatorVisible;
  const embeddable = Boolean(streamUrl) &&
    (!routeProofRequired || routeProofReady) &&
    canOpenView;
  return {
    provider: stream.provider ?? null,
    url: streamUrl,
    routeId: stream.routeId ?? null,
    displayAllocationId: stream.displayAllocationId ?? null,
    routePoolEntryId: stream.routePoolEntryId ?? null,
    connectionId: stream.connectionId ?? null,
    connectionName: stream.connectionName ?? null,
    routeSource: stream.routeSource ?? null,
    providerMode: stream.providerMode ?? null,
    viewerLeaseIds: stream.viewerLeaseIds ?? [],
    controllerLeaseId: stream.controllerLeaseId ?? null,
    embeddable,
    controllable: embeddable &&
      !readOnly &&
      (!routeProofRequired || routeProofReady) &&
      canOpenControlViewStream(readinessCheckedStream),
    readOnly: readOnly || (routeProofRequired && !routeProofReady),
    controlInput: routeProofRequired && !routeProofReady ? null : stream.controlInput ?? null,
    operatorVisibleState: effectiveOperatorVisible.state,
    operatorVisibleReason: effectiveOperatorVisible.reason,
    routeSummary: viewStreamRouteSummary(stream, effectiveOperatorVisible),
  };
}

function cdpSnapshotFallbackViewStream(
  browser: WorkspaceServiceBrowser,
  stream: WorkspaceServiceViewStream,
): WorkspaceNodeViewStream | null {
  if (normalize(stream.provider) !== "cdp_screencast") return null;
  if (stream.frameUrl || stream.url || stream.externalUrl) return null;
  const cdpPort = portFromUrl(browser.cdpEndpoint);
  if (!cdpPort) return null;
  const streamReason = stringOrNull(
    recordFromUnknown(stream.readiness)?.reason ??
      recordFromUnknown(stream.readiness)?.state ??
      recordFromUnknown(stream.remoteReadiness)?.reason ??
      recordFromUnknown(stream.remoteReadiness)?.state,
  );
  const streamUrl = `/api/session-screenshot?port=${encodeURIComponent(String(cdpPort))}`;
  return {
    provider: "cdp_snapshot",
    url: streamUrl,
    routeId: stream.routeId ?? `service-cdp-snapshot:${browser.id}`,
    displayAllocationId: stream.displayAllocationId ?? null,
    routePoolEntryId: stream.routePoolEntryId ?? null,
    connectionId: stream.connectionId ?? null,
    connectionName: stream.connectionName ?? browser.id,
    routeSource: "service-cdp-snapshot",
    providerMode: "read_only_snapshot_poll",
    viewerLeaseIds: stream.viewerLeaseIds ?? [],
    controllerLeaseId: null,
    embeddable: true,
    controllable: false,
    readOnly: true,
    controlInput: null,
    operatorVisibleState: "ready",
    operatorVisibleReason: streamReason ? `CDP screencast unavailable: ${streamReason}` : null,
    routeSummary: streamReason
      ? `CDP snapshot ${cdpPort} / read-only fallback / ${streamReason}`
      : `CDP snapshot ${cdpPort} / read-only fallback`,
  };
}

function routeBoundOwnershipFor({
  live,
  viewerClient,
  stream,
  viewStream,
}: {
  live: boolean;
  viewerClient: boolean;
  stream: WorkspaceServiceViewStream | null;
  viewStream: WorkspaceNodeViewStream | null;
}): WorkspaceRouteBoundOwnership | null {
  if (viewerClient) {
    return {
      state: "viewer-client",
      routeId: stream?.routeId ?? viewStream?.routeId ?? null,
      displayAllocationId: stream?.displayAllocationId ?? viewStream?.displayAllocationId ?? null,
      routePoolEntryId: stream?.routePoolEntryId ?? viewStream?.routePoolEntryId ?? null,
      reason: "Viewer clients cannot own route-bound browser control.",
    };
  }
  if (!stream || normalize(stream.provider) !== "rdp_gateway") return null;
  if (!live) {
    return {
      state: "retained",
      routeId: stream.routeId ?? null,
      displayAllocationId: stream.displayAllocationId ?? null,
      routePoolEntryId: stream.routePoolEntryId ?? null,
      reason: "Retained route-bound browser records are not live control targets.",
    };
  }
  const explicitState = normalizeRouteBoundOwnershipState(stream.routeBoundOwnership?.state);
  const state = explicitState ?? (viewStream?.operatorVisibleState === "ready" && viewStream.controllable ? "finalized" : "diagnostic");
  return {
    state,
    routeId: stream.routeBoundOwnership?.routeId ?? stream.routeId ?? null,
    displayAllocationId: stream.routeBoundOwnership?.displayAllocationId ?? stream.displayAllocationId ?? null,
    routePoolEntryId: stream.routeBoundOwnership?.routePoolEntryId ?? stream.routePoolEntryId ?? null,
    reason: stream.routeBoundOwnership?.reason
      ?? (state === "diagnostic" ? viewStream?.operatorVisibleReason : null)
      ?? routeBoundOwnershipDefaultReason(state),
  };
}

function routeBoundOwnershipControlReason(
  ownership: WorkspaceRouteBoundOwnership | null,
  stream: WorkspaceServiceViewStream | null,
): string | null {
  if (!ownership || !stream || normalize(stream.provider) !== "rdp_gateway") return null;
  if (ownership.state === "finalized") return null;
  return ownership.reason ?? routeBoundOwnershipDefaultReason(ownership.state);
}

function normalizeRouteBoundOwnershipState(value: unknown): WorkspaceRouteBoundOwnershipState | null {
  if (typeof value !== "string") return null;
  const normalized = value.toLowerCase().replaceAll("_", "-");
  if (normalized === "finalized") return "finalized";
  if (normalized === "pending") return "pending";
  if (normalized === "rolled-back" || normalized === "rollback-complete") return "rolled-back";
  if (normalized === "diagnostic") return "diagnostic";
  if (normalized === "retained") return "retained";
  if (normalized === "viewer-client") return "viewer-client";
  return null;
}

function routeBoundOwnershipDefaultReason(state: WorkspaceRouteBoundOwnershipState): string {
  switch (state) {
    case "pending":
      return "Route-bound browser ownership is still pending finalization.";
    case "rolled-back":
      return "Route-bound browser ownership was rolled back.";
    case "diagnostic":
      return "Route-bound browser ownership is diagnostic, not finalized.";
    case "retained":
      return "Retained route-bound browser records are not live control targets.";
    case "viewer-client":
      return "Viewer clients cannot own route-bound browser control.";
    case "finalized":
      return "Route-bound browser ownership is finalized.";
  }
}

function browserProcessIndicators(
  browser: WorkspaceServiceBrowser,
  viewStream?: WorkspaceNodeViewStream | null,
): WorkspaceNodeProcess | null {
  const cdpPort = portFromUrl(browser.cdpEndpoint);
  const streamPort = portFromUrl(viewStream?.url);
  const stats = browser.processStats ?? null;
  if (!browser.pid && !stats && !cdpPort && !streamPort) return null;
  return {
    pid: browser.pid ?? stats?.pid ?? null,
    running: stats?.running ?? null,
    rssBytes: stats?.rssBytes ?? null,
    cpuSeconds: stats?.cpuSeconds ?? null,
    cdpPort,
    streamPort,
  };
}

function portFromUrl(value?: string | null): number | null {
  if (!value) return null;
  try {
    const port = new URL(value).port;
    return port ? Number(port) : null;
  } catch {
    const match = value.match(/:(\d{2,5})(?:\/|$)/);
    return match ? Number(match[1]) : null;
  }
}

function selectPrimaryWorkspaceViewStream(streams?: WorkspaceServiceViewStream[]): WorkspaceServiceViewStream | null {
  if (!streams?.length) return null;
  return [...streams].sort((left, right) => workspaceViewStreamScore(right) - workspaceViewStreamScore(left))[0] ?? null;
}

function workspaceViewStreamScore(stream: WorkspaceServiceViewStream): number {
  const provider = normalize(stream.provider);
  const routeSource = normalize(stream.routeSource);
  const providerMode = normalize(stream.providerMode);
  const displayAllocationId = normalize(stream.displayAllocationId);
  const streamUrl = stream.frameUrl || stream.url || stream.externalUrl;
  let score = 0;
  if (streamUrl) score += 50;
  if (provider && provider !== "cdp_screencast") score += 40;
  if (provider === "rdp_gateway") score += 20;
  if (stream.controlInput && stream.readOnly !== true) score += 15;
  if (stream.routeId || stream.connectionId || stream.connectionName) score += 20;
  if (displayAllocationId) score += 10;
  if (displayAllocationId && !displayAllocationId.includes("shared")) score += 35;
  if (routeSource === "pool" || routeSource === "generated" || routeSource === "discovered") score += 40;
  if (providerMode === "simultaneous_view") score += 20;
  if (providerMode === "single_controller") score += 10;
  if (readinessState(stream.remoteReadiness ?? stream.readiness) === "ready") score += 10;
  return score;
}

function viewStreamRouteSummary(
  stream: WorkspaceServiceViewStream,
  operatorVisible: { state: string; reason: string | null } = routeProofState(stream),
): string {
  const viewerCount = stream.viewerLeaseIds?.length ?? 0;
  const leaseLabel = stream.controllerLeaseId
    ? `${viewerCount} viewer${viewerCount === 1 ? "" : "s"}, controller leased`
    : `${viewerCount} viewer${viewerCount === 1 ? "" : "s"}`;
  return compactLabels([
    stream.routeId || stream.connectionName || stream.connectionId || stream.displayAllocationId || "unrouted",
    stream.displayAllocationId ? `display ${stream.displayAllocationId}` : null,
    stream.providerMode?.replaceAll("_", " ") ?? null,
    leaseLabel,
    operatorVisible.state === "ready" ? "operator visible" : operatorVisible.reason,
    viewStreamReadinessLabel(stream),
  ]).join(" / ");
}

function routeProofState(stream: WorkspaceServiceViewStream): { state: string; reason: string | null } {
  const provider = normalize(stream.provider);
  if (provider !== "rdp_gateway") return { state: "ready", reason: null };
  const attachability = recordFromUnknown(stream.attachability);
  if (stringOrNull(attachability?.proofState) === "ready" && normalize(stringOrNull(attachability?.state)) === "attached_ready") {
    return { state: "ready", reason: null };
  }
  const structuredProof = structuredRouteProofState(stream);
  if (structuredProof) return structuredProof;
  const displayStateValue = routeDisplayStateValue(stream);
  const displayState = normalize(typeof displayStateValue === "string" ? displayStateValue : null);
  if (displayState === "browser_window_visible") return { state: "ready", reason: null };
  if (displayState === "terminal_only") {
    return {
      state: "route_bound_terminal_only",
      reason: "Remote route display is terminal-only.",
    };
  }
  if (displayState === "empty_display" || displayState === "display_idle" || displayState === "idle_display") {
    return {
      state: "route_bound_display_idle",
      reason: "Remote route display has no visible browser window.",
    };
  }
  if (displayState === "non_browser_windows" || displayState === "no_browser_window") {
    return {
      state: "route_bound_browser_not_visible",
      reason: "Remote route display does not show a browser window.",
    };
  }
  const readiness = normalize(readinessState(stream.remoteReadiness ?? stream.readiness));
  if (readiness === "terminal_only_route" || readiness === "terminal_only") {
    return {
      state: "route_bound_terminal_only",
      reason: "Remote route readiness reports a terminal-only display.",
    };
  }
  return {
    state: "route_bound_proof_missing",
    reason: "operator-visible proof missing",
  };
}

function routeDisplayStateValue(stream: WorkspaceServiceViewStream): unknown {
  for (const source of [
    stream.displayContent,
    recordValue(stream.remoteReadiness, "displayContent"),
    recordValue(stream.readiness, "displayContent"),
  ]) {
    const state = recordValue(source, "state");
    if (typeof state === "string" && state.trim()) return state;
  }
  return null;
}

function structuredRouteProofState(stream: WorkspaceServiceViewStream): { state: string; reason: string | null } | null {
  for (const source of [stream.remoteReadiness, stream.readiness]) {
    const sourceRecord = recordFromUnknown(source);
    const operatorVisible = recordFromUnknown(sourceRecord?.operatorVisible) ?? sourceRecord;
    const operatorState = normalizedRecordState(operatorVisible);
    if (isOperatorVisibleProofState(operatorState)) {
      return routeProofResult(operatorState, stringOrNull(operatorVisible?.reason));
    }
    const components = recordFromUnknown(operatorVisible?.components);
    for (const key of ["route", "tab", "guacamole"]) {
      const component = recordFromUnknown(components?.[key]);
      const componentState = normalizedRecordState(component);
      if (isOperatorVisibleProofState(componentState)) {
        return routeProofResult(componentState, stringOrNull(component?.reason));
      }
    }
  }
  return null;
}

function normalizedRecordState(record: Record<string, unknown> | null): string | null {
  const state = stringOrNull(record?.state);
  return state ? normalize(state) : null;
}

function isOperatorVisibleProofState(state: string | null): state is string {
  return Boolean(
    state &&
      state !== "ready" &&
      state !== "not_checked" &&
      [
        "wrong_tab",
        "guacamole_route_unavailable",
        "cdp_target_unavailable",
        "stale_route_record",
      ].includes(state),
  );
}

function routeProofResult(state: string, reason: string | null): { state: string; reason: string | null } {
  return {
    state,
    reason: reason ?? routeProofReason(state),
  };
}

function routeProofReason(state: string): string {
  switch (state) {
    case "wrong_tab":
      return "Remote route display is browser-visible, but the selected tab URL does not match the requested target.";
    case "guacamole_route_unavailable":
      return "Remote route display and tab are ready, but the Guacamole operator route is unavailable.";
    case "cdp_target_unavailable":
      return "Remote route selected tab has no CDP target id.";
    case "stale_route_record":
      return "Remote route metadata points at a stale route allocation.";
    default:
      return state.replaceAll("_", " ");
  }
}

function viewStreamReadinessLabel(stream: WorkspaceServiceViewStream): string {
  const attachabilityState = stringOrNull(recordFromUnknown(stream.attachability)?.state);
  if (attachabilityState) return attachabilityState.replaceAll("_", " ");
  const readiness = stream.remoteReadiness ?? stream.readiness;
  const state = readinessState(readiness);
  if (state) return state.replaceAll("_", " ");
  return "readiness unknown";
}

function readinessState(readiness: unknown): string | null {
  if (!readiness) return null;
  if (typeof readiness === "string") return readiness.trim() || null;
  if (typeof readiness !== "object") return null;
  if (Array.isArray(readiness)) {
    for (const item of readiness) {
      const state = readinessState(item);
      if (state && state !== "ready") return state;
    }
    return null;
  }
  const record = readiness as Record<string, unknown>;
  for (const key of ["state", "status", "readiness", "lastProviderEvent"]) {
    const value = record[key];
    if (typeof value === "string" && value.trim()) return value.trim();
  }
  const components = record.components ?? record.checks ?? record.results;
  if (Array.isArray(components)) return readinessState(components);
  return null;
}

function recordValue(source: unknown, key: string): unknown {
  return source && typeof source === "object" ? (source as Record<string, unknown>)[key] : undefined;
}

function recordFromUnknown(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null;
}

function stringOrNull(value: unknown): string | null {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function actorLabel(value: unknown): string {
  if (!value) return "unknown operator";
  if (typeof value === "string") return value;
  if (typeof value !== "object") return String(value);
  const entries = Object.entries(value as Record<string, unknown>);
  if (entries.length === 0) return "unknown operator";
  const [kind, detail] = entries[0];
  return detail ? `${kind}: ${String(detail)}` : kind;
}

function takeoverForSessions(
  sessions: WorkspaceServiceSession[],
  allocation: WorkspaceServiceProfileAllocation | undefined,
  jobs: WorkspaceServiceJob[],
): WorkspaceNodeTakeover | null {
  const session = sessions.find((candidate) => normalize(candidate.lease) === "human_takeover");
  return session ? takeoverForSession(session, allocation, jobs) : null;
}

function takeoverForSession(
  session: WorkspaceServiceSession,
  allocation: WorkspaceServiceProfileAllocation | undefined,
  jobs: WorkspaceServiceJob[],
): WorkspaceNodeTakeover | null {
  if (normalize(session.lease) !== "human_takeover") return null;
  const conflictSessionIds = uniqueStrings([
    ...(session.profileLeaseConflictSessionIds ?? []),
    ...(allocation?.conflictSessionIds ?? []),
  ]);
  const waitingJobIds = uniqueStrings([
    ...(allocation?.waitingJobIds ?? []),
    ...jobs
      .filter((job) => normalize(job.state) === "waiting_profile_lease")
      .map((job) => job.id),
  ]);
  const queueImpact = waitingJobIds.length > 0
    ? `Human takeover holds the profile lease; ${waitingJobIds.length} waiting job${waitingJobIds.length === 1 ? "" : "s"} cannot continue.`
    : conflictSessionIds.length > 0
      ? `Human takeover holds the profile lease; ${conflictSessionIds.length} session conflict${conflictSessionIds.length === 1 ? "" : "s"} need review.`
      : "Human takeover holds the profile lease; automation should resume only after service-owned release.";
  return {
    active: true,
    sessionId: session.id,
    ownerLabel: actorLabel(session.owner),
    startedAt: session.createdAt ?? session.lastLeaseObservedAt ?? null,
    lastObservedAt: session.lastLeaseObservedAt ?? null,
    expiresAt: session.expiresAt ?? null,
    cleanup: session.cleanup ?? null,
    profileLeaseDisposition: session.profileLeaseDisposition ?? null,
    conflictSessionIds,
    waitingJobIds,
    queueImpact,
    resumeSupported: false,
    resumeReason: "Service contracts expose human_takeover lease state but do not yet expose a service-owned resume action.",
  };
}

function browserActions(
  browser: WorkspaceServiceBrowser,
  live: boolean,
  viewStream: WorkspaceNodeViewStream | null,
  attentionReason?: string | null,
  takeover?: WorkspaceNodeTakeover | null,
  profileActionability?: WorkspaceProfileActionability | null,
): WorkspaceNodeAction[] {
  const canViewStream = live && Boolean(viewStream?.embeddable);
  const canControlStream = live && Boolean(viewStream?.controllable);
  const streamUnavailableReason = viewStream?.operatorVisibleReason ?? viewStream?.routeSummary;
  const canOpenSharedProfileTab =
    profileActionability?.recommendedAction === "openSharedProfileTab" &&
    profileActionability.enabled;
  const actions: WorkspaceNodeAction[] = [
    ...(takeover ? [{ id: "resume" as const, label: "Resume", enabled: takeover.resumeSupported, reason: takeover.resumeReason }] : []),
    { id: "focus", label: "Focus", enabled: live, reason: live ? null : "Browser is retained, not live." },
    {
      id: "add-tab",
      label: "Add tab",
      enabled: canOpenSharedProfileTab,
      reason: canOpenSharedProfileTab
        ? null
        : profileActionability?.reason ?? "No compatible retained profile owner is available for tab creation.",
    },
    { id: "view", label: "View", enabled: canViewStream, reason: canViewStream ? null : live ? streamUnavailableReason || "No embeddable service-owned view stream." : "Browser is retained, not live." },
    { id: "control", label: "Control", enabled: canControlStream, reason: canControlStream ? null : live ? streamUnavailableReason || "No controllable service-owned view stream." : "Browser is retained, not live." },
    { id: "repair", label: "Repair", enabled: Boolean(attentionReason), reason: attentionReason ? null : "No service-owned repair reason is present." },
    { id: "close", label: "Close", enabled: live, reason: live ? null : "Browser is already retained." },
    { id: "external-open", label: "Open externally", enabled: canViewStream, reason: canViewStream ? null : live ? streamUnavailableReason || "No external stream URL is recorded." : "Browser is retained, not live." },
  ];
  return actions.filter((action) => action.id !== "external-open" || browser.viewStreams?.length);
}

function serviceSessionActions(
  live: boolean,
  reason?: string | null,
  takeover?: WorkspaceNodeTakeover | null,
): WorkspaceNodeAction[] {
  if (takeover) {
    return [
      { id: "resume", label: "Resume", enabled: takeover.resumeSupported, reason: takeover.resumeReason },
      { id: "focus", label: "Focus", enabled: live, reason: live ? null : "No live browser is attached to this retained session." },
    ];
  }
  return [
    { id: "focus", label: "Focus", enabled: live, reason: live ? null : "No live browser is attached to this retained session." },
    { id: "resume", label: "Resume", enabled: Boolean(reason), reason: reason ? null : "No paused or blocked service-owned resume state is present." },
  ];
}

function daemonActions(
  session: SessionInfo,
  live: boolean,
  viewStream?: WorkspaceNodeViewStream | null,
): WorkspaceNodeAction[] {
  const enabled = live && !session.closing;
  const canViewStream = enabled && Boolean(viewStream?.embeddable);
  const canControlStream = enabled && Boolean(viewStream?.controllable);
  return [
    { id: "focus", label: "Focus", enabled, reason: enabled ? null : "Session is not focusable yet." },
    { id: "view", label: "View", enabled: canViewStream, reason: canViewStream ? null : "No daemon stream is registered." },
    { id: "control", label: "Control", enabled: canControlStream, reason: canControlStream ? null : "No controllable daemon stream is registered." },
    { id: "add-tab", label: "Add tab", enabled, reason: enabled ? null : "Session is not ready for tab creation." },
    { id: "close", label: "Close", enabled, reason: enabled ? null : "Session is not ready to close cleanly." },
    { id: "kill", label: "Kill", enabled: live, reason: live ? null : "No live process is known." },
  ];
}

function detectedBrowserActions(live: boolean, viewStream?: WorkspaceNodeViewStream | null): WorkspaceNodeAction[] {
  const canViewSnapshot = live && Boolean(viewStream?.embeddable);
  const readOnlyReason = live
    ? null
    : "Detected browser is not live.";
  const mutateReason = live
    ? "Non-owned browsers require an explicit borrow-control action before mutation."
    : "Detected browser is not live.";
  const streamReason = live
    ? "Detected external CDP browsers must be explicitly adopted before agent-browser can start a CDP screencast stream."
    : "Detected browser is not live.";
  const lifecycleReason = "Non-owned browsers require explicit adoption before lifecycle or profile actions.";
  const repairReason = "Non-owned browsers do not use service-owned route repair.";
  return [
    { id: "inspect", label: "Inspect", enabled: live, reason: readOnlyReason },
    { id: "stream", label: "Stream", enabled: false, reason: streamReason },
    { id: "screenshot", label: "Screenshot", enabled: live, reason: readOnlyReason },
    { id: "focus", label: "Focus", enabled: false, reason: "Non-owned browsers are not focus-owned by agent-browser." },
    { id: "view", label: "View", enabled: canViewSnapshot, reason: canViewSnapshot ? null : streamReason },
    { id: "control", label: "Control", enabled: false, reason: mutateReason },
    { id: "borrow-control", label: "Borrow control", enabled: false, reason: "Borrow-control approval is not active for this non-owned browser." },
    { id: "add-tab", label: "Add tab", enabled: false, reason: mutateReason },
    { id: "repair", label: "Repair", enabled: false, reason: repairReason },
    { id: "close", label: "Close", enabled: false, reason: lifecycleReason },
    { id: "kill", label: "Kill", enabled: false, reason: lifecycleReason },
  ];
}

function profileActions(
  allocation: WorkspaceServiceProfileAllocation,
  reason?: string | null,
): WorkspaceNodeAction[] {
  const needsSeeding = Boolean(profileAttentionReason(allocation));
  const blocked = Boolean(profileBlockedReason(allocation));
  return [
    { id: "launch", label: "Launch", enabled: !needsSeeding && !blocked, reason: needsSeeding || blocked ? reason : null },
    { id: "seed", label: "Seed", enabled: needsSeeding, reason: needsSeeding ? null : "Profile readiness does not require manual seeding." },
  ];
}

function jobMatches({
  job,
  browser,
  sessions = [],
  allocation,
  tabs = [],
}: {
  job: WorkspaceServiceJob;
  browser?: WorkspaceServiceBrowser;
  sessions?: WorkspaceServiceSession[];
  allocation?: WorkspaceServiceProfileAllocation;
  tabs?: WorkspaceServiceTab[];
}): boolean {
  if (browser?.id && objectContains(job, browser.id)) return true;
  if (allocation?.profileId && objectContains(job, allocation.profileId)) return true;
  if (sessions.some((session) => objectContains(job, session.id))) return true;
  if (tabs.some((tab) => objectContains(job, tab.id))) return true;
  if (allocation?.serviceNames?.some((serviceName) => serviceName === job.serviceName)) return true;
  if (allocation?.agentNames?.some((agentName) => agentName === job.agentName)) return true;
  if (allocation?.taskNames?.some((taskName) => taskName === job.taskName)) return true;
  return sessions.some((session) =>
    (session.serviceName && session.serviceName === job.serviceName) ||
    (session.agentName && session.agentName === job.agentName) ||
    (session.taskName && session.taskName === job.taskName),
  );
}

function addDuplicateBrowserDiagnostics({
  diagnostics,
  browsers,
  kind,
  valueFor,
  messageFor,
}: {
  diagnostics: WorkspaceOwnershipDiagnostic[];
  browsers: WorkspaceServiceBrowser[];
  kind: WorkspaceOwnershipDiagnosticKind;
  valueFor: (browser: WorkspaceServiceBrowser) => string | null | undefined;
  messageFor: (value: string, browsers: WorkspaceServiceBrowser[]) => string;
}): void {
  const groups = new Map<string, WorkspaceServiceBrowser[]>();
  for (const browser of browsers) {
    const value = valueFor(browser)?.trim();
    if (!value) continue;
    const group = groups.get(value);
    if (group) {
      group.push(browser);
    } else {
      groups.set(value, [browser]);
    }
  }
  for (const [value, group] of groups) {
    if (group.length < 2) continue;
    diagnostics.push({
      kind,
      severity: "warning",
      message: messageFor(value, group),
      relatedIds: group.map((browser) => relatedId("browser", browser.id)),
    });
  }
}

function groupDiagnosticsByRelatedId(diagnostics: WorkspaceOwnershipDiagnostic[]): Map<string, WorkspaceOwnershipDiagnostic[]> {
  const groups = new Map<string, WorkspaceOwnershipDiagnostic[]>();
  for (const diagnostic of diagnostics) {
    for (const id of diagnostic.relatedIds) {
      const bucket = groups.get(id);
      if (bucket) {
        bucket.push(diagnostic);
      } else {
        groups.set(id, [diagnostic]);
      }
    }
  }
  return groups;
}

function diagnosticsForRelatedIds(
  diagnosticsByRelatedId: Map<string, WorkspaceOwnershipDiagnostic[]>,
  ids: string[],
): WorkspaceOwnershipDiagnostic[] {
  const seen = new Set<WorkspaceOwnershipDiagnostic>();
  const result: WorkspaceOwnershipDiagnostic[] = [];
  for (const id of ids) {
    for (const diagnostic of diagnosticsByRelatedId.get(id) ?? []) {
      if (seen.has(diagnostic)) continue;
      seen.add(diagnostic);
      result.push(diagnostic);
    }
  }
  return result;
}

function relatedId(kind: "browser" | "session" | "tab", id: string): string {
  return `${kind}:${id}`;
}

function objectContains(value: unknown, needle: string): boolean {
  if (!needle) return false;
  if (typeof value === "string") return value === needle || value.includes(needle);
  if (typeof value === "number" || typeof value === "boolean" || value == null) return false;
  if (Array.isArray(value)) return value.some((item) => objectContains(item, needle));
  if (typeof value === "object") {
    return Object.values(value as Record<string, unknown>).some((item) => objectContains(item, needle));
  }
  return false;
}

function isActiveJob(job: WorkspaceServiceJob): boolean {
  return ACTIVE_JOB_STATES.has(normalize(job.state));
}

function profileReadinessSummary(allocation: WorkspaceServiceProfileAllocation): string | null {
  const readiness = allocation.targetReadiness?.[0];
  if (!readiness) return null;
  return compactLabels([readiness.targetServiceId, readiness.loginId, readiness.state])[0] ?? null;
}

function compareWorkspaceNodes(left: WorkspaceNode, right: WorkspaceNode): number {
  const placementDelta =
    (left.inventoryPlacement ?? workspaceInventoryPlacementForNode(left)).rank -
    (right.inventoryPlacement ?? workspaceInventoryPlacementForNode(right)).rank;
  if (placementDelta !== 0) return placementDelta;
  return left.sortLabel.localeCompare(right.sortLabel);
}

function groupByLinkedId<T>(items: T[], links: (item: T) => string[]): Map<string, T[]> {
  const grouped = new Map<string, T[]>();
  for (const item of items) {
    for (const id of links(item)) {
      const bucket = grouped.get(id);
      if (bucket) {
        bucket.push(item);
      } else {
        grouped.set(id, [item]);
      }
    }
  }
  return grouped;
}

function uniqueById<T extends { id: string }>(items: T[]): T[] {
  const seen = new Set<string>();
  const result: T[] = [];
  for (const item of items) {
    if (seen.has(item.id)) continue;
    seen.add(item.id);
    result.push(item);
  }
  return result;
}

function uniqueStrings(values: Array<string | null | undefined>): string[] {
  return [...new Set(values.filter((value): value is string => Boolean(value?.trim())))];
}

function compactLabels(values: Array<string | number | null | undefined>): string[] {
  return values
    .map((value) => (value == null ? "" : String(value).trim()))
    .filter(Boolean);
}

function normalize(value: string | null | undefined): string {
  return (value ?? "").trim().toLowerCase();
}

function isDefined<T>(value: T | null | undefined): value is T {
  return value != null;
}
