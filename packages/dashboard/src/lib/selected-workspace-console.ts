import type { ConsoleEntry } from "../types.ts";
import type { SelectedWorkspaceContext } from "./selected-workspace-context.ts";

export type ConsoleEvidenceLevel = "error" | "warning" | "info" | "log" | "debug" | "unknown";
export type ConsoleEvidenceSource = "live-stream" | "retained-console" | "page-errors" | "global-fallback";
export type ConsoleAttributionQuality = "scoped" | "partially-scoped" | "unscoped" | "missing";

export type SelectedWorkspaceConsoleRelatedIds = {
  workspaceId: string | null;
  browserId: string | null;
  sessionId: string | null;
  daemonSession: string | null;
  tabId: string | null;
  targetId: string | null;
  streamPort: number | null;
  cdpPort: number | null;
};

export type SelectedWorkspaceConsoleRow = {
  id: string;
  type: "console" | "page_error";
  level: ConsoleEvidenceLevel;
  text: string;
  timestamp: number;
  source: ConsoleEvidenceSource;
  sourceLabel: string;
  attribution: ConsoleAttributionQuality;
  line: number | null;
  column: number | null;
  relatedIds: SelectedWorkspaceConsoleRelatedIds;
  scoped: boolean;
};

export type SelectedWorkspaceConsoleSourceReadiness = {
  source: ConsoleEvidenceSource;
  label: string;
  available: boolean;
  reason: string | null;
};

export type SelectedWorkspaceConsoleEvidence = {
  workspaceId: string | null;
  label: string;
  state: string;
  summary: string;
  rows: SelectedWorkspaceConsoleRow[];
  scopedRows: SelectedWorkspaceConsoleRow[];
  unscopedRows: SelectedWorkspaceConsoleRow[];
  counts: {
    total: number;
    scoped: number;
    errors: number;
    warnings: number;
    info: number;
    logs: number;
    pageErrors: number;
    runtimeExceptions: number;
    security: number;
    unscoped: number;
  };
  sourceReadiness: SelectedWorkspaceConsoleSourceReadiness[];
  latestTimestamp: number | null;
  attributionQuality: ConsoleAttributionQuality;
  unavailableReason: string | null;
};

export type BuildSelectedWorkspaceConsoleEvidenceOptions = {
  includeUnscoped?: boolean;
  now?: number;
  maxRows?: number;
};

const DEFAULT_MAX_ROWS = 200;
const SENSITIVE_PATTERN = /(authorization|bearer\s+[a-z0-9._-]+|password\s*[=:]\s*[^&\s]+|token\s*[=:]\s*[^&\s]+|secret\s*[=:]\s*[^&\s]+|cookie\s*[=:]\s*[^&\s]+)/gi;

export function buildSelectedWorkspaceConsoleEvidence(
  context: SelectedWorkspaceContext | null | undefined,
  entries: ConsoleEntry[],
  options: BuildSelectedWorkspaceConsoleEvidenceOptions = {},
): SelectedWorkspaceConsoleEvidence {
  const includeUnscoped = options.includeUnscoped ?? false;
  const maxRows = options.maxRows ?? DEFAULT_MAX_ROWS;
  const selectedStreamPort = context?.runtime.streamPort ?? null;
  const hasSelection = Boolean(context?.node);
  const normalizedRows = entries
    .slice(-maxRows)
    .map((entry, index) => buildConsoleRow(entry, index, context ?? null))
    .filter((row) => includeUnscoped || row.scoped);
  const scopedRows = normalizedRows.filter((row) => row.scoped);
  const unscopedRows = normalizedRows.filter((row) => !row.scoped);
  const sourceReadiness = buildSourceReadiness(context ?? null, entries);
  const latestTimestamp = scopedRows.reduce<number | null>(
    (latest, row) => latest == null || row.timestamp > latest ? row.timestamp : latest,
    null,
  );
  const hasScopedSource = selectedStreamPort != null &&
    entries.some((entry) => {
      const entryRecord = entry as ConsoleEntry & { streamPort?: number | null; source?: ConsoleEvidenceSource };
      return entryRecord.streamPort === selectedStreamPort;
    });
  const unavailableReason = !hasSelection
    ? "Select a workspace to scope Console evidence."
    : selectedStreamPort == null
      ? "The selected workspace does not report a stream port, so live Console attribution is unavailable."
      : !hasScopedSource
        ? `Listening on stream ${selectedStreamPort}, but no Console events from that stream have arrived.`
      : null;

  return {
    workspaceId: context?.node?.id ?? null,
    label: context?.label ?? "No workspace selected",
    state: context?.state ?? "none",
    summary: consoleSummary(scopedRows, unscopedRows, unavailableReason),
    rows: normalizedRows,
    scopedRows,
    unscopedRows,
    counts: {
      total: normalizedRows.length,
      scoped: scopedRows.length,
      errors: scopedRows.filter((row) => row.level === "error").length,
      warnings: scopedRows.filter((row) => row.level === "warning").length,
      info: scopedRows.filter((row) => row.level === "info").length,
      logs: scopedRows.filter((row) => row.level === "log" || row.level === "debug").length,
      pageErrors: scopedRows.filter((row) => row.type === "page_error").length,
      runtimeExceptions: scopedRows.filter((row) => row.type === "page_error").length,
      security: scopedRows.filter((row) => isSecurityConsoleText(row.text)).length,
      unscoped: unscopedRows.length,
    },
    sourceReadiness,
    latestTimestamp,
    attributionQuality: scopedRows.length > 0
      ? "scoped"
      : unscopedRows.length > 0
        ? "unscoped"
        : selectedStreamPort != null
          ? "missing"
          : "missing",
    unavailableReason,
  };
}

export function consoleEvidenceForChat(evidence: SelectedWorkspaceConsoleEvidence) {
  if (evidence.scopedRows.length === 0) {
    const reason = evidence.unavailableReason
      ?? "No scoped Console entries are available for the selected workspace.";
    return {
      available: false,
      summary: reason,
      facts: {
        status: "unavailable",
        reason,
        sourceReadiness: evidence.sourceReadiness,
        unscopedCount: evidence.counts.unscoped,
      },
    };
  }

  return {
    available: true,
    summary: evidence.summary,
    facts: {
      counts: evidence.counts,
      sourceReadiness: evidence.sourceReadiness,
      latestTimestamp: evidence.latestTimestamp,
      rows: evidence.scopedRows.slice(0, 25).map((row) => ({
        id: row.id,
        level: row.level,
        type: row.type,
        text: row.text,
        timestamp: row.timestamp,
        line: row.line,
        column: row.column,
        relatedIds: row.relatedIds,
      })),
    },
  };
}

export function redactedConsoleEvidenceBundle(evidence: SelectedWorkspaceConsoleEvidence): string {
  return JSON.stringify({
    workspaceId: evidence.workspaceId,
    label: evidence.label,
    state: evidence.state,
    summary: evidence.summary,
    counts: evidence.counts,
    rows: evidence.scopedRows,
    sourceReadiness: evidence.sourceReadiness,
  }, null, 2);
}

function buildConsoleRow(
  entry: ConsoleEntry,
  index: number,
  context: SelectedWorkspaceContext | null,
): SelectedWorkspaceConsoleRow {
  const entryRecord = entry as ConsoleEntry & { streamPort?: number | null; source?: ConsoleEvidenceSource };
  const streamPort = typeof entryRecord.streamPort === "number" ? entryRecord.streamPort : null;
  const selectedStreamPort = context?.runtime.streamPort ?? null;
  const source = entryRecord.source === "retained-console"
    ? "retained-console"
    : streamPort == null
      ? "global-fallback"
      : "live-stream";
  const scoped = selectedStreamPort != null && streamPort === selectedStreamPort;
  const type = entry.type === "page_error" ? "page_error" : "console";
  const text = redactConsoleText(entry.type === "page_error" ? entry.text : entry.text);
  return {
    id: `console.${streamPort ?? "global"}.${entry.timestamp}.${index}`,
    type,
    level: normalizeConsoleLevel(entry),
    text,
    timestamp: entry.timestamp,
    source,
    sourceLabel: source === "live-stream"
      ? "Live stream"
      : source === "retained-console"
        ? "Retained console"
        : "Global fallback",
    attribution: scoped ? "scoped" : streamPort == null ? "unscoped" : "missing",
    line: entry.type === "page_error" ? entry.line : null,
    column: entry.type === "page_error" ? entry.column : null,
    relatedIds: scoped
      ? {
          workspaceId: context?.node?.id ?? null,
          browserId: context?.node?.browserId ?? null,
          sessionId: context?.node?.serviceSessionId ?? null,
          daemonSession: context?.daemonSession?.session ?? null,
          tabId: context?.primaryTab?.id ?? null,
          targetId: context?.primaryTab?.targetId ?? null,
          streamPort,
          cdpPort: context?.runtime.cdpPort ?? null,
        }
      : {
          workspaceId: null,
          browserId: null,
          sessionId: null,
          daemonSession: null,
          tabId: null,
          targetId: null,
          streamPort,
          cdpPort: null,
        },
    scoped,
  };
}

function buildSourceReadiness(
  context: SelectedWorkspaceContext | null,
  entries: ConsoleEntry[],
): SelectedWorkspaceConsoleSourceReadiness[] {
  const selectedStreamPort = context?.runtime.streamPort ?? null;
  const liveEntryCount = selectedStreamPort == null
    ? 0
    : entries.filter((entry) => (entry as ConsoleEntry & { streamPort?: number | null }).streamPort === selectedStreamPort).length;
  const retainedEntryCount = entries.filter((entry) =>
    (entry as ConsoleEntry & { source?: ConsoleEvidenceSource }).source === "retained-console"
  ).length;
  return [
    {
      source: "live-stream",
      label: "Live stream",
      available: selectedStreamPort != null,
      reason: selectedStreamPort == null
        ? "Selected workspace does not report a stream port."
        : liveEntryCount > 0
          ? null
          : "Live stream is attributable, but no Console entries have arrived yet.",
    },
    {
      source: "retained-console",
      label: "Retained console",
      available: selectedStreamPort != null,
      reason: selectedStreamPort == null
        ? "Retained console reads need a selected workspace stream port."
        : retainedEntryCount > 0
          ? null
          : "Retained console polling is active, but no retained rows are available yet.",
    },
    {
      source: "page-errors",
      label: "Page errors",
      available: selectedStreamPort != null,
      reason: selectedStreamPort == null
        ? "Page-error attribution needs a selected workspace stream port."
        : null,
    },
    {
      source: "global-fallback",
      label: "Global fallback",
      available: entries.some((entry) => (entry as ConsoleEntry & { streamPort?: number | null }).streamPort == null),
      reason: "Global fallback rows are labeled unscoped and excluded from scoped counts by default.",
    },
  ];
}

function normalizeConsoleLevel(entry: ConsoleEntry): ConsoleEvidenceLevel {
  if (entry.type === "page_error") return "error";
  const level = entry.level.toLowerCase();
  if (level === "error" || level === "warning" || level === "info" || level === "debug") return level;
  if (level === "warn") return "warning";
  if (level === "log") return "log";
  return "unknown";
}

function consoleSummary(
  scopedRows: SelectedWorkspaceConsoleRow[],
  unscopedRows: SelectedWorkspaceConsoleRow[],
  unavailableReason: string | null,
): string {
  if (scopedRows.length === 0) {
    return unavailableReason
      ?? `No scoped Console entries. ${unscopedRows.length} unscoped fallback row${unscopedRows.length === 1 ? "" : "s"} available.`;
  }
  const errors = scopedRows.filter((row) => row.level === "error").length;
  const warnings = scopedRows.filter((row) => row.level === "warning").length;
  return `${scopedRows.length} scoped Console entr${scopedRows.length === 1 ? "y" : "ies"}: ${errors} error${errors === 1 ? "" : "s"}, ${warnings} warning${warnings === 1 ? "" : "s"}.`;
}

function redactConsoleText(text: string): string {
  return text
    .replace(/([a-z][a-z0-9+.-]*:\/\/)([^/@\s]+)@/gi, "$1[redacted]@")
    .replace(SENSITIVE_PATTERN, "[redacted]");
}

function isSecurityConsoleText(text: string): boolean {
  return /(content security policy|csp|mixed content|certificate|permission|cors|blocked by client|extension)/i.test(text);
}
