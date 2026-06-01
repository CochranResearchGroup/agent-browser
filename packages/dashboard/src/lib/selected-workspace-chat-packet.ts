import type { SelectedWorkspaceContext } from "./selected-workspace-context.ts";
import {
  consoleEvidenceForChat,
  type SelectedWorkspaceConsoleEvidence,
} from "./selected-workspace-console.ts";

export const CONTEXTUAL_CHAT_PROVIDER_ID = "codex-app-server" as const;
export const SELECTED_WORKSPACE_CHAT_PACKET_VERSION = "selected-workspace-chat.v1" as const;
export const CODEX_WORKSPACE_OBSERVATION_VERSION = "codex-workspace-observation.v1" as const;

export type ContextualChatProviderId = typeof CONTEXTUAL_CHAT_PROVIDER_ID;

export type SelectedWorkspaceChatEvidenceSource =
  | "workspace"
  | "activity"
  | "stream"
  | "console"
  | "network"
  | "storage"
  | "extensions";

export type SelectedWorkspaceChatPacket = {
  version: typeof SELECTED_WORKSPACE_CHAT_PACKET_VERSION;
  createdAt: string;
  provider: ContextualChatProviderId;
  selection: {
    workspaceId: string | null;
    browserId: string | null;
    sessionId: string | null;
    tabId: string | null;
    profileId: string | null;
    jobId: string | null;
  };
  workspace: {
    id: string | null;
    label: string;
    source: string;
    state: string;
    health: string | null;
    live: boolean;
    retained: boolean;
    viewable: boolean;
    controllable: boolean;
    missingReason: string | null;
  };
  runtime: {
    pid: number | null;
    running: boolean | null;
    rssBytes: number | null;
    cpuSeconds: number | null;
    cdpPort: number | null;
    streamPort: number | null;
    lastFrameAt: number | null;
  };
  page: {
    title: string | null;
    url: string | null;
    targetId: string | null;
    lifecycle: string | null;
    active: boolean | null;
  };
  stream: {
    provider: string | null;
    routeSummary: string | null;
    controlInput: string | null;
    embeddable: boolean;
    controllable: boolean;
  };
  ownership: {
    serviceName: string | null;
    agentName: string | null;
    taskName: string | null;
  };
  evidence: SelectedWorkspaceChatEvidence[];
  redaction: {
    secretsOmitted: true;
    screenshotsIncluded: false;
    rawStorageIncluded: false;
    rawHeadersIncluded: false;
  };
};

export type SelectedWorkspaceChatEvidence = {
  id: string;
  source: SelectedWorkspaceChatEvidenceSource;
  sourceLabel: string;
  summary: string;
  facts: Record<string, unknown>;
  available: boolean;
  unavailableReason: string | null;
  freshness: string;
  included: boolean;
};

export type SelectedWorkspaceChatPacketOptions = {
  createdAt?: string;
  include?: Partial<Record<SelectedWorkspaceChatEvidenceSource, boolean>>;
  consoleEvidence?: SelectedWorkspaceConsoleEvidence | null;
};

export type CodexWorkspaceObservation = {
  version: typeof CODEX_WORKSPACE_OBSERVATION_VERSION;
  provider: ContextualChatProviderId;
  runId: string;
  threadId: string | null;
  createdAt: string;
  workspaceId: string | null;
  summary: string;
  detectedState: string;
  blockers: Array<{
    severity: "info" | "warning" | "blocked";
    summary: string;
    evidenceIds: string[];
  }>;
  risks: Array<{
    summary: string;
    evidenceIds: string[];
  }>;
  suggestedNextInspections: Array<{
    label: string;
    reason: string;
    evidenceIds: string[];
  }>;
  unsupportedActions: Array<{
    label: string;
    reason: string;
  }>;
  confidence: "low" | "medium" | "high";
};

const SENSITIVE_KEY_PATTERN = /(authorization|bearer|cookie|password|secret|storage|token|credential|screenshot|header|body|localstorage|sessionstorage)/i;

export function buildSelectedWorkspaceChatPacket(
  context: SelectedWorkspaceContext,
  options: SelectedWorkspaceChatPacketOptions = {},
): SelectedWorkspaceChatPacket {
  const include = options.include ?? {};
  const createdAt = options.createdAt ?? new Date().toISOString();
  const workspaceEvidence = buildWorkspaceEvidence(context, include.workspace !== false);
  const activityEvidence = buildActivityEvidence(context, include.activity !== false);
  const streamEvidence = buildStreamEvidence(context, include.stream !== false);
  const consoleEvidence = buildConsoleEvidence(options.consoleEvidence ?? null, include.console === true);
  return {
    version: SELECTED_WORKSPACE_CHAT_PACKET_VERSION,
    createdAt,
    provider: CONTEXTUAL_CHAT_PROVIDER_ID,
    selection: { ...context.selection },
    workspace: {
      id: context.node?.id ?? null,
      label: context.label,
      source: context.source,
      state: context.state,
      health: context.node?.health ?? null,
      live: context.live,
      retained: context.retained,
      viewable: context.viewable,
      controllable: context.controllable,
      missingReason: context.missingReason,
    },
    runtime: {
      pid: context.runtime.pid ?? null,
      running: context.runtime.running ?? null,
      rssBytes: context.runtime.rssBytes ?? null,
      cpuSeconds: context.runtime.cpuSeconds ?? null,
      cdpPort: context.runtime.cdpPort ?? null,
      streamPort: context.runtime.streamPort ?? null,
      lastFrameAt: context.runtime.lastFrameAt ?? null,
    },
    page: {
      title: context.primaryTab?.title ?? null,
      url: context.primaryTab?.url ?? null,
      targetId: context.primaryTab?.targetId ?? null,
      lifecycle: context.primaryTab?.lifecycle ?? null,
      active: context.primaryTab?.active ?? null,
    },
    stream: {
      provider: context.stream?.provider ?? null,
      routeSummary: context.stream?.routeSummary ?? null,
      controlInput: context.stream?.controlInput ?? null,
      embeddable: context.stream?.embeddable ?? false,
      controllable: context.stream?.controllable ?? false,
    },
    ownership: {
      serviceName: context.ownership.serviceName ?? null,
      agentName: context.ownership.agentName ?? null,
      taskName: context.ownership.taskName ?? null,
    },
    evidence: [
      workspaceEvidence,
      activityEvidence,
      streamEvidence,
      consoleEvidence,
      unavailableEvidence("network"),
      unavailableEvidence("storage"),
      unavailableEvidence("extensions"),
    ],
    redaction: {
      secretsOmitted: true,
      screenshotsIncluded: false,
      rawStorageIncluded: false,
      rawHeadersIncluded: false,
    },
  };
}

export function selectedWorkspaceChatPacketSummary(packet: SelectedWorkspaceChatPacket): string {
  const parts = [
    `${packet.workspace.label} (${packet.workspace.state})`,
    packet.workspace.id ? `workspace ${packet.workspace.id}` : null,
    packet.page.url ? `page ${packet.page.url}` : null,
    packet.stream.provider ? `stream ${packet.stream.provider}` : null,
    packet.runtime.pid ? `pid ${packet.runtime.pid}` : null,
  ].filter(Boolean);
  return parts.join(" | ");
}

export function validateSelectedWorkspaceChatPacket(packet: SelectedWorkspaceChatPacket): string[] {
  const errors: string[] = [];
  if (packet.version !== SELECTED_WORKSPACE_CHAT_PACKET_VERSION) errors.push("Unsupported selected workspace chat packet version.");
  if (packet.provider !== CONTEXTUAL_CHAT_PROVIDER_ID) errors.push("Selected workspace chat packet provider must be codex-app-server.");
  if (!packet.redaction?.secretsOmitted) errors.push("Selected workspace chat packet must omit secrets.");
  const serialized = JSON.stringify(packet);
  for (const forbidden of ["password", "bearer ", "authorization", "cookie=", "localStorage", "sessionStorage", "data:image/"]) {
    if (serialized.toLowerCase().includes(forbidden.toLowerCase())) {
      errors.push(`Selected workspace chat packet contains forbidden sensitive marker: ${forbidden}`);
    }
  }
  return errors;
}

function buildWorkspaceEvidence(
  context: SelectedWorkspaceContext,
  included: boolean,
): SelectedWorkspaceChatEvidence {
  return {
    id: "workspace.summary",
    source: "workspace",
    sourceLabel: "Workspace",
    summary: context.evidence.summary,
    facts: redactFacts({
      workspaceId: context.node?.id ?? null,
      label: context.label,
      source: context.source,
      state: context.state,
      health: context.node?.health ?? null,
      live: context.live,
      retained: context.retained,
      viewable: context.viewable,
      controllable: context.controllable,
      missingReason: context.missingReason,
      browserId: context.node?.browserId ?? null,
      sessionId: context.daemonSession?.session ?? context.node?.serviceSessionId ?? null,
      profileId: context.profileAllocation?.profileId ?? context.node?.profileId ?? null,
      pageTitle: context.primaryTab?.title ?? null,
      pageUrl: context.primaryTab?.url ?? null,
      targetId: context.primaryTab?.targetId ?? null,
      streamProvider: context.stream?.provider ?? null,
      streamRouteSummary: context.stream?.routeSummary ?? null,
      pid: context.runtime.pid ?? null,
      cdpPort: context.runtime.cdpPort ?? null,
      streamPort: context.runtime.streamPort ?? null,
      serviceName: context.ownership.serviceName ?? null,
      agentName: context.ownership.agentName ?? null,
      taskName: context.ownership.taskName ?? null,
      jobs: context.jobs.length,
      incidents: context.incidents.length,
      diagnostics: context.diagnostics.length,
      evidenceRows: context.evidence.rows,
    }),
    freshness: freshnessLabel(context.refreshedAt),
    available: true,
    unavailableReason: null,
    included,
  };
}

function buildActivityEvidence(
  context: SelectedWorkspaceContext,
  included: boolean,
): SelectedWorkspaceChatEvidence {
  const jobStates = countBy(context.jobs.map((job) => typeof job.state === "string" ? job.state : "unknown"));
  const incidentSeverities = countBy(context.incidents.map((incident) => typeof incident.severity === "string" ? incident.severity : "unknown"));
  const enabledActions = context.actions.filter((action) => action.enabled).map((action) => action.label);
  const unavailableActions = context.actions
    .filter((action) => !action.enabled)
    .map((action) => ({
      label: action.label,
      reason: action.reason ?? "No service contract or runtime support is currently reported.",
    }));
  const diagnostics = context.diagnostics.map((diagnostic) => ({
    kind: diagnostic.kind,
    severity: diagnostic.severity,
    message: diagnostic.message,
    relatedIds: diagnostic.relatedIds,
  }));
  const summary = [
    `${context.jobs.length} related job${context.jobs.length === 1 ? "" : "s"}`,
    `${context.incidents.length} incident${context.incidents.length === 1 ? "" : "s"}`,
    `${context.diagnostics.length} diagnostic${context.diagnostics.length === 1 ? "" : "s"}`,
  ].join(", ");
  return {
    id: "activity.summary",
    source: "activity",
    sourceLabel: "Activity summary",
    summary: `Selected workspace activity summary: ${summary}.`,
    facts: redactFacts({
      jobCount: context.jobs.length,
      jobIds: context.jobs.map((job) => job.id),
      jobStates,
      incidentCount: context.incidents.length,
      incidentIds: context.incidents.map((incident) => incident.id),
      incidentSeverities,
      diagnostics,
      enabledActions,
      unavailableActions,
    }),
    available: true,
    unavailableReason: null,
    freshness: freshnessLabel(context.refreshedAt),
    included,
  };
}

function buildStreamEvidence(
  context: SelectedWorkspaceContext,
  included: boolean,
): SelectedWorkspaceChatEvidence {
  const stream = context.stream;
  const facts = {
    provider: stream?.provider ?? null,
    routeSummary: stream?.routeSummary ?? null,
    routeId: stream?.routeId ?? null,
    routeSource: stream?.routeSource ?? null,
    controlInput: stream?.controlInput ?? null,
    cdpPort: context.runtime.cdpPort ?? null,
    streamPort: context.runtime.streamPort ?? null,
    lastFrameAt: context.runtime.lastFrameAt ?? null,
    embeddable: stream?.embeddable ?? false,
    controllable: stream?.controllable ?? false,
    readOnly: stream?.readOnly ?? null,
    viewable: context.viewable,
    workspaceControllable: context.controllable,
    unavailableReasons: context.actions
      .filter((action) => (action.id === "view" || action.id === "control") && !action.enabled)
      .map((action) => ({ action: action.id, reason: action.reason ?? "No stream readiness reason is reported." })),
  };
  const summary = stream
    ? `Stream readiness: ${stream.provider ?? "unknown provider"} via ${stream.routeSummary ?? "unknown route"}.`
    : "Stream readiness: no service-owned view stream is reported for the selected workspace.";
  return {
    id: "stream.readiness",
    source: "stream",
    sourceLabel: "Stream readiness",
    summary,
    facts: redactFacts(facts),
    available: true,
    unavailableReason: null,
    freshness: freshnessLabel(context.refreshedAt),
    included,
  };
}

function buildConsoleEvidence(
  evidence: SelectedWorkspaceConsoleEvidence | null,
  included: boolean,
): SelectedWorkspaceChatEvidence {
  if (!evidence) return unavailableEvidence("console");
  const chatEvidence = consoleEvidenceForChat(evidence);
  return {
    id: chatEvidence.available ? "console.summary" : "console.unavailable",
    source: "console",
    sourceLabel: chatEvidence.available ? "Console" : "Console unavailable",
    summary: chatEvidence.summary,
    facts: redactFacts(chatEvidence.facts),
    available: chatEvidence.available,
    unavailableReason: chatEvidence.available ? null : chatEvidence.summary,
    freshness: chatEvidence.available && evidence.latestTimestamp
      ? freshnessLabel(evidence.latestTimestamp)
      : chatEvidence.available
        ? "fresh"
        : "unavailable",
    included: chatEvidence.available ? included : false,
  };
}

function unavailableEvidence(
  source: Exclude<SelectedWorkspaceChatEvidenceSource, "workspace" | "activity" | "stream">,
): SelectedWorkspaceChatEvidence {
  const label = evidenceSourceLabel(source);
  const reason = `${label} evidence is not implemented for selected-workspace Chat in this slice.`;
  return {
    id: `${source}.unavailable`,
    source,
    sourceLabel: label,
    summary: reason,
    facts: {
      status: "unavailable",
      reason,
      plannedProviderSlice: plannedProviderSlice(source),
    },
    freshness: "unavailable",
    available: false,
    unavailableReason: reason,
    included: false,
  };
}

function plannedProviderSlice(source: SelectedWorkspaceChatEvidenceSource): string {
  if (source === "activity" || source === "console") return "P12-B";
  if (source === "stream") return "P12-A";
  if (source === "network" || source === "storage") return "P12-C";
  if (source === "extensions") return "P12-D";
  return "P12-E";
}

function evidenceSourceLabel(source: SelectedWorkspaceChatEvidenceSource): string {
  if (source === "workspace") return "Workspace";
  if (source === "activity") return "Activity summary";
  if (source === "stream") return "Stream readiness";
  if (source === "console") return "Console unavailable";
  if (source === "network") return "Network unavailable";
  if (source === "storage") return "Storage unavailable";
  return "Extensions unavailable";
}

function countBy(values: string[]): Record<string, number> {
  const counts: Record<string, number> = {};
  for (const value of values) {
    counts[value] = (counts[value] ?? 0) + 1;
  }
  return counts;
}

function redactFacts(value: unknown): Record<string, unknown> {
  const redacted = redactValue(value);
  return typeof redacted === "object" && redacted !== null && !Array.isArray(redacted)
    ? redacted as Record<string, unknown>
    : {};
}

function redactValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(redactValue);
  if (typeof value !== "object" || value === null) return value;
  if (isSensitiveEvidenceRow(value)) {
    return {
      label: value.label,
      value: "[redacted]",
    };
  }
  const output: Record<string, unknown> = {};
  for (const [key, nested] of Object.entries(value)) {
    if (SENSITIVE_KEY_PATTERN.test(key)) {
      output[key] = "[redacted]";
    } else {
      output[key] = redactValue(nested);
    }
  }
  return output;
}

function isSensitiveEvidenceRow(value: object): value is { label: string; value: unknown } {
  if (!("label" in value) || typeof value.label !== "string") return false;
  return SENSITIVE_KEY_PATTERN.test(value.label);
}

function freshnessLabel(timestamp: number): string {
  if (!Number.isFinite(timestamp) || timestamp <= 0) return "unknown";
  const ageMs = Math.max(0, Date.now() - timestamp);
  if (ageMs < 10_000) return "fresh";
  if (ageMs < 60_000) return "recent";
  return "stale";
}
