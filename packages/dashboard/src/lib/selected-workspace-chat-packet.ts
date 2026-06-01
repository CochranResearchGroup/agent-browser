import type { SelectedWorkspaceContext } from "./selected-workspace-context.ts";

export const CONTEXTUAL_CHAT_PROVIDER_ID = "codex-app-server" as const;
export const SELECTED_WORKSPACE_CHAT_PACKET_VERSION = "selected-workspace-chat.v1" as const;
export const CODEX_WORKSPACE_OBSERVATION_VERSION = "codex-workspace-observation.v1" as const;

export type ContextualChatProviderId = typeof CONTEXTUAL_CHAT_PROVIDER_ID;

export type SelectedWorkspaceChatEvidenceSource =
  | "workspace"
  | "activity"
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
  summary: string;
  facts: Record<string, unknown>;
  freshness: string;
  included: boolean;
};

export type SelectedWorkspaceChatPacketOptions = {
  createdAt?: string;
  include?: Partial<Record<SelectedWorkspaceChatEvidenceSource, boolean>>;
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
      unavailableEvidence("activity", include.activity === true),
      unavailableEvidence("console", include.console === true),
      unavailableEvidence("network", include.network === true),
      unavailableEvidence("storage", include.storage === true),
      unavailableEvidence("extensions", include.extensions === true),
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
    included,
  };
}

function unavailableEvidence(
  source: Exclude<SelectedWorkspaceChatEvidenceSource, "workspace">,
  requested: boolean,
): SelectedWorkspaceChatEvidence {
  return {
    id: `${source}.unavailable`,
    source,
    summary: `${source} evidence provider is not implemented in this slice.`,
    facts: {
      status: "unavailable",
      plannedProviderSlice: plannedProviderSlice(source),
    },
    freshness: "unavailable",
    included: false,
  };
}

function plannedProviderSlice(source: SelectedWorkspaceChatEvidenceSource): string {
  if (source === "activity" || source === "console") return "P12-B";
  if (source === "network" || source === "storage") return "P12-C";
  if (source === "extensions") return "P12-D";
  return "P12-E";
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
