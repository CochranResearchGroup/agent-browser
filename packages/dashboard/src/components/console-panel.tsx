"use client";

import { useCallback, useMemo, useState } from "react";
import { useAtomValue } from "jotai/react";
import { consoleLogsAtom } from "@/store/stream";
import type { SelectedWorkspaceContext } from "@/lib/selected-workspace-context";
import {
  buildSelectedWorkspaceConsoleEvidence,
  redactedConsoleEvidenceBundle,
  type ConsoleEvidenceLevel,
  type SelectedWorkspaceConsoleRow,
} from "@/lib/selected-workspace-console";
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { Check, Copy, MessageSquare, RotateCcw, Search } from "lucide-react";

type LevelFilter = "all" | ConsoleEvidenceLevel | "page_error" | "security" | "unscoped";

const LEVEL_LABELS: Record<LevelFilter, string> = {
  all: "All",
  error: "Errors",
  warning: "Warnings",
  info: "Info",
  log: "Logs",
  debug: "Debug",
  unknown: "Other",
  page_error: "Page errors",
  security: "Security",
  unscoped: "Unscoped",
};

const LEVEL_COLORS: Record<string, string> = {
  error: "text-destructive",
  warning: "text-warning",
  info: "text-blue-400",
  log: "text-muted-foreground",
  debug: "text-muted-foreground/70",
  unknown: "text-muted-foreground/70",
};

function formatClock(ts: number): string {
  return new Date(ts).toLocaleTimeString("en-US", {
    hour12: false,
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

function formatAge(ts: number | null): string {
  if (!ts) return "none";
  const seconds = Math.max(0, Math.floor((Date.now() - ts) / 1000));
  if (seconds < 10) return "now";
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  return `${Math.floor(minutes / 60)}h`;
}

function isSecurityText(text: string): boolean {
  return /(content security policy|csp|mixed content|certificate|permission|cors|blocked by client|extension)/i.test(text);
}

function rowMatchesFilter(row: SelectedWorkspaceConsoleRow, filter: LevelFilter): boolean {
  if (filter === "all") return true;
  if (filter === "page_error") return row.type === "page_error";
  if (filter === "security") return isSecurityText(row.text);
  if (filter === "unscoped") return !row.scoped;
  return row.level === filter;
}

function rowSearchText(row: SelectedWorkspaceConsoleRow): string {
  return [
    row.text,
    row.level,
    row.sourceLabel,
    row.relatedIds.browserId,
    row.relatedIds.sessionId,
    row.relatedIds.tabId,
    row.relatedIds.targetId,
  ].filter(Boolean).join(" ").toLowerCase();
}

function countForFilter(rows: SelectedWorkspaceConsoleRow[], filter: LevelFilter): number {
  return rows.filter((row) => rowMatchesFilter(row, filter)).length;
}

export function ConsolePanel({
  selectedWorkspaceContext,
}: {
  selectedWorkspaceContext?: SelectedWorkspaceContext | null;
} = {}) {
  const entries = useAtomValue(consoleLogsAtom);
  const [filter, setFilter] = useState<LevelFilter>("all");
  const [search, setSearch] = useState("");
  const [includeUnscoped, setIncludeUnscoped] = useState(false);
  const [copied, setCopied] = useState<"bundle" | string | null>(null);

  const evidence = useMemo(
    () => buildSelectedWorkspaceConsoleEvidence(selectedWorkspaceContext, entries, { includeUnscoped }),
    [selectedWorkspaceContext, entries, includeUnscoped],
  );
  const visibleRows = useMemo(() => {
    const needle = search.trim().toLowerCase();
    return evidence.rows
      .filter((row) => rowMatchesFilter(row, filter))
      .filter((row) => !needle || rowSearchText(row).includes(needle))
      .sort((a, b) => {
        const aPriority = a.scoped ? 0 : 1;
        const bPriority = b.scoped ? 0 : 1;
        if (aPriority !== bPriority) return aPriority - bPriority;
        if (a.level === "error" && b.level !== "error") return -1;
        if (a.level !== "error" && b.level === "error") return 1;
        return b.timestamp - a.timestamp;
      });
  }, [evidence.rows, filter, search]);

  const copyText = useCallback((key: "bundle" | string, text: string) => {
    navigator.clipboard.writeText(text).then(() => {
      setCopied(key);
      window.setTimeout(() => setCopied(null), 1500);
    });
  }, []);

  const resetView = useCallback(() => {
    setFilter("all");
    setSearch("");
    setIncludeUnscoped(false);
  }, []);

  const sendToChat = useCallback(() => {
    window.dispatchEvent(new CustomEvent("agent-browser-dashboard-console-send-to-chat", {
      detail: {
        workspaceId: evidence.workspaceId,
        evidenceIds: evidence.scopedRows.map((row) => row.id),
      },
    }));
  }, [evidence.scopedRows, evidence.workspaceId]);

  const filters: LevelFilter[] = [
    "all",
    "error",
    "warning",
    "page_error",
    "security",
    "info",
    "log",
    "unscoped",
  ];

  return (
    <div
      className="console-inspector flex h-full flex-col text-xs"
      data-selected-workspace-id={selectedWorkspaceContext?.node?.id ?? ""}
      data-selected-workspace-state={selectedWorkspaceContext?.state ?? ""}
      data-console-evidence-attribution={evidence.attributionQuality}
    >
      <div className="console-inspector-header shrink-0 border-b border-border/60 px-2 py-1.5">
        <div className="grid grid-cols-[minmax(0,1fr)_auto] gap-2">
          <div className="min-w-0">
            <div className="flex min-w-0 items-center gap-1.5">
              <span className="truncate font-medium text-foreground">
                {evidence.label}
              </span>
              <span className="rounded border border-border/60 px-1 py-0.5 font-mono text-[9px] text-muted-foreground">
                {evidence.state}
              </span>
              <span className="rounded border border-border/60 px-1 py-0.5 font-mono text-[9px] text-muted-foreground">
                {evidence.attributionQuality}
              </span>
            </div>
            <div className="mt-1 truncate text-[10px] text-muted-foreground">
              {evidence.summary}
            </div>
          </div>
          <div className="grid grid-cols-4 gap-1 text-[10px]">
            <Metric label="scoped" value={evidence.counts.scoped} />
            <Metric label="errors" value={evidence.counts.errors} tone={evidence.counts.errors > 0 ? "error" : undefined} />
            <Metric label="warn" value={evidence.counts.warnings} tone={evidence.counts.warnings > 0 ? "warning" : undefined} />
            <Metric label="latest" value={formatAge(evidence.latestTimestamp)} />
          </div>
        </div>
        <div className="mt-1.5 flex flex-wrap gap-1 text-[9px]">
          {evidence.sourceReadiness.map((source) => (
            <span
              key={source.source}
              title={source.reason ?? source.label}
              className={cn(
                "rounded border px-1.5 py-0.5",
                source.available
                  ? "border-primary/30 bg-primary/10 text-primary"
                  : "border-border/50 bg-background/40 text-muted-foreground/70",
              )}
            >
              {source.label}
            </span>
          ))}
        </div>
      </div>

      <div className="shrink-0 border-b border-border/60 px-2 py-1">
        <div className="flex flex-wrap items-center gap-1">
          {filters.map((item) => {
            const sourceRows = item === "unscoped" ? evidence.rows : evidence.scopedRows;
            const count = item === "all" ? evidence.counts.scoped : countForFilter(sourceRows, item);
            return (
              <button
                key={item}
                type="button"
                onClick={() => {
                  if (item === "unscoped") setIncludeUnscoped(true);
                  setFilter(item);
                }}
                className={cn(
                  "flex h-5 items-center gap-1 rounded border px-1.5 text-[10px]",
                  filter === item
                    ? "border-primary/50 bg-primary/10 text-primary"
                    : "border-border/50 text-muted-foreground hover:text-foreground",
                )}
              >
                {LEVEL_LABELS[item]}
                {count > 0 && (
                  <Badge variant="secondary" className="h-3.5 min-w-4 px-1 text-[9px] tabular-nums">
                    {count}
                  </Badge>
                )}
              </button>
            );
          })}
          <label className="ml-auto flex h-5 items-center gap-1 rounded border border-border/50 px-1.5 text-[10px] text-muted-foreground">
            <input
              type="checkbox"
              checked={includeUnscoped}
              onChange={(event) => setIncludeUnscoped(event.target.checked)}
              className="size-3 accent-primary"
            />
            fallback
          </label>
        </div>
        <div className="mt-1 flex items-center gap-1">
          <div className="relative min-w-0 flex-1">
            <Search className="pointer-events-none absolute left-1.5 top-1/2 size-3 -translate-y-1/2 text-muted-foreground" />
            <input
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="Search scoped Console evidence"
              className="h-6 w-full rounded border border-border bg-background pl-6 pr-2 text-[11px] text-foreground outline-none placeholder:text-muted-foreground/60"
            />
          </div>
          <button
            type="button"
            onClick={() => copyText("bundle", redactedConsoleEvidenceBundle(evidence))}
            className="flex size-6 items-center justify-center rounded border border-border/60 text-muted-foreground hover:text-foreground"
            title="Copy redacted Console evidence"
          >
            {copied === "bundle" ? <Check className="size-3" /> : <Copy className="size-3" />}
          </button>
          <button
            type="button"
            onClick={sendToChat}
            disabled={evidence.scopedRows.length === 0}
            className="flex size-6 items-center justify-center rounded border border-border/60 text-muted-foreground hover:text-foreground disabled:pointer-events-none disabled:opacity-40"
            title="Send Console evidence to Chat"
          >
            <MessageSquare className="size-3" />
          </button>
          <button
            type="button"
            onClick={resetView}
            className="flex size-6 items-center justify-center rounded border border-border/60 text-muted-foreground hover:text-foreground"
            title="Reset Console view filters"
          >
            <RotateCcw className="size-3" />
          </button>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto font-mono">
        {visibleRows.length === 0 ? (
          <div className="px-3 py-8 text-center text-xs text-muted-foreground">
            {evidence.unavailableReason ?? "No scoped Console entries for this workspace."}
          </div>
        ) : (
          visibleRows.map((row) => (
            <ConsoleEvidenceRow
              key={row.id}
              row={row}
              copied={copied === row.id}
              onCopy={() => copyText(row.id, JSON.stringify(row, null, 2))}
            />
          ))
        )}
      </div>
    </div>
  );
}

function Metric({
  label,
  value,
  tone,
}: {
  label: string;
  value: string | number;
  tone?: "error" | "warning";
}) {
  return (
    <div className={cn(
      "rounded border border-border/60 px-1.5 py-0.5 text-right tabular-nums",
      tone === "error" && "border-destructive/40 bg-destructive/10 text-destructive",
      tone === "warning" && "border-warning/40 bg-warning/10 text-warning",
    )}>
      <div className="text-[8px] uppercase text-muted-foreground">{label}</div>
      <div className="text-[10px] font-medium">{value}</div>
    </div>
  );
}

function ConsoleEvidenceRow({
  row,
  copied,
  onCopy,
}: {
  row: SelectedWorkspaceConsoleRow;
  copied: boolean;
  onCopy: () => void;
}) {
  const color = LEVEL_COLORS[row.level] ?? "text-muted-foreground";
  return (
    <details
      className={cn(
        "group border-b border-border/50 text-[11px]",
        row.level === "error" ? "bg-destructive/5" : row.level === "warning" ? "bg-warning/5" : "",
      )}
    >
      <summary className="grid cursor-pointer grid-cols-[3.4rem_3.8rem_minmax(0,1fr)_auto] items-start gap-2 px-2 py-1 marker:content-none">
        <span className="text-muted-foreground/70">{formatClock(row.timestamp)}</span>
        <span className={cn("uppercase", color)}>{row.type === "page_error" ? "error" : row.level}</span>
        <span className={cn("min-w-0 break-all whitespace-pre-wrap", color)}>
          {row.text}
          {(row.line != null || row.column != null) && (
            <span className="ml-1 text-muted-foreground/70">
              ({row.line ?? "?"}{row.column != null ? `:${row.column}` : ""})
            </span>
          )}
        </span>
        <span className={cn(
          "rounded border px-1 py-0.5 text-[9px]",
          row.scoped
            ? "border-primary/30 text-primary"
            : "border-border/50 text-muted-foreground",
        )}>
          {row.attribution}
        </span>
      </summary>
      <div className="grid gap-1 px-2 pb-2 pl-[7.2rem] text-[10px] text-muted-foreground">
        <div className="flex flex-wrap gap-1">
          <IdBadge label="browser" value={row.relatedIds.browserId} />
          <IdBadge label="session" value={row.relatedIds.sessionId ?? row.relatedIds.daemonSession} />
          <IdBadge label="tab" value={row.relatedIds.tabId} />
          <IdBadge label="target" value={row.relatedIds.targetId} />
          <IdBadge label="stream" value={row.relatedIds.streamPort == null ? null : String(row.relatedIds.streamPort)} />
          <IdBadge label="cdp" value={row.relatedIds.cdpPort == null ? null : String(row.relatedIds.cdpPort)} />
        </div>
        <div className="flex items-center gap-2">
          <span>{row.sourceLabel}</span>
          <span>{row.scoped ? "counted as scoped Console evidence" : "excluded from scoped counts"}</span>
          <button
            type="button"
            onClick={onCopy}
            className="ml-auto flex items-center gap-1 rounded border border-border/50 px-1.5 py-0.5 hover:text-foreground"
          >
            {copied ? <Check className="size-3" /> : <Copy className="size-3" />}
            Copy
          </button>
        </div>
      </div>
    </details>
  );
}

function IdBadge({ label, value }: { label: string; value: string | null | undefined }) {
  if (!value) return null;
  return (
    <span className="max-w-full truncate rounded border border-border/50 px-1 py-0.5">
      {label}:{value}
    </span>
  );
}
