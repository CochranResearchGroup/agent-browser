"use client";

import { useRef, useEffect, useState, useCallback, useMemo } from "react";
import { useChat } from "@ai-sdk/react";
import { DefaultChatTransport } from "ai";
import { Streamdown } from "streamdown";
import { getChatApiUrl } from "@/store/chat";
import { activeSessionNameAtom } from "@/store/sessions";
import { useAtomValue } from "jotai/react";
import type { SelectedWorkspaceContext } from "@/lib/selected-workspace-context";
import {
  CONTEXTUAL_CHAT_PROVIDER_ID,
  buildSelectedWorkspaceChatPacket,
  selectedWorkspaceChatPacketSummary,
  validateSelectedWorkspaceChatPacket,
  type CodexWorkspaceObservation,
  type SelectedWorkspaceChatEvidenceSource,
  type SelectedWorkspaceChatPacket,
} from "@/lib/selected-workspace-chat-packet";
import {
  APP_INTELLIGENCE_INSPECT_API_URL,
  APP_INTELLIGENCE_OPERATOR_STATUS_API_URL,
  APP_INTELLIGENCE_OPERATOR_TURN_API_URL,
} from "@/lib/dashboard-api";
import {
  updateDashboardWorkspaceUrlSelection,
  type DashboardWorkspaceUrlSelection,
} from "@/lib/workspace-url-selection";
import { shikiTheme } from "@/lib/shiki-theme";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { ArrowUp, Square, Trash2, ChevronRight, Loader, Copy, Check, Download, ShieldCheck, Search } from "lucide-react";

type ExtraProps = { node?: unknown };
type MdImgProps = React.ImgHTMLAttributes<HTMLImageElement> & ExtraProps;
type MdHeadingProps = React.HTMLAttributes<HTMLHeadingElement> & ExtraProps;
type MdAnchorProps = React.AnchorHTMLAttributes<HTMLAnchorElement> & ExtraProps;
type MdPreProps = React.HTMLAttributes<HTMLPreElement> & ExtraProps;
type MdCodeProps = React.HTMLAttributes<HTMLElement> & ExtraProps;

const chatComponents = {
  img: ({ node: _node, src, alt, ...props }: MdImgProps) => {
    if (typeof src === "string" && src.startsWith("data:image/")) {
      return <img src={src} alt={alt} className="rounded-md border border-border max-w-full my-1" {...props} />;
    }
    return null;
  },
  h1: ({ node: _node, ...props }: MdHeadingProps) => <p className="font-bold" {...props} />,
  h2: ({ node: _node, ...props }: MdHeadingProps) => <p className="font-bold" {...props} />,
  h3: ({ node: _node, ...props }: MdHeadingProps) => <p className="font-bold" {...props} />,
  h4: ({ node: _node, ...props }: MdHeadingProps) => <p className="font-bold" {...props} />,
  h5: ({ node: _node, ...props }: MdHeadingProps) => <p className="font-bold" {...props} />,
  h6: ({ node: _node, ...props }: MdHeadingProps) => <p className="font-bold" {...props} />,
  a: ({ node: _node, href, children, ...props }: MdAnchorProps) => (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      className="text-primary underline underline-offset-2"
      {...props}
    >
      {children}
    </a>
  ),
  pre: ({ node: _node, ...props }: MdPreProps) => (
    <pre
      className="text-[11px] bg-background border border-border rounded-md p-2 my-1.5 whitespace-pre-wrap break-all"
      {...props}
    />
  ),
  code: ({ className, children, node: _node, ...props }: MdCodeProps) => {
    if (className?.includes("language-")) {
      return <code className={className} {...props}>{children}</code>;
    }
    return (
      <span
        className="text-[11px] bg-secondary/60 px-1 py-0.5 rounded text-foreground font-mono break-all"
        {...props}
      >
        {children}
      </span>
    );
  },
};

const STORAGE_PREFIX = "dashboard-chat-";
const IMAGE_DATA_URL_RE = /data:image\/[^;]+;base64,[A-Za-z0-9+/=]+/g;

function stripImagesForStorage(messages: unknown[]): unknown[] {
  const json = JSON.stringify(messages);
  return JSON.parse(json.replace(IMAGE_DATA_URL_RE, "[image stripped]"));
}

const SUGGESTIONS = [
  "Inspect viewport readiness",
  "Explain why this browser is not viewable",
  "Summarize the selected workspace state",
  "List the next read-only checks",
];

const DEFAULT_EVIDENCE_INCLUDE: Partial<Record<SelectedWorkspaceChatEvidenceSource, boolean>> = {
  workspace: true,
  activity: true,
  stream: true,
};

interface ToolInvocationPart {
  type: string;
  toolCallId: string;
  state: string;
  input?: Record<string, unknown>;
  output?: unknown;
}

function isToolPart(part: { type: string }): part is ToolInvocationPart {
  return part.type.startsWith("tool-");
}

function truncateOutput(text: string, maxLines = 30): string {
  const lines = text.split("\n");
  if (lines.length <= maxLines) return text;
  return lines.slice(0, maxLines).join("\n") + `\n... (${lines.length - maxLines} more lines)`;
}

function parseOutputObject(raw: unknown): Record<string, unknown> | null {
  if (typeof raw === "string") {
    try {
      const parsed = JSON.parse(raw);
      if (typeof parsed === "object" && parsed !== null) return parsed;
    } catch { /* not JSON */ }
    return null;
  }
  if (typeof raw === "object" && raw !== null) return raw as Record<string, unknown>;
  return null;
}

function formatOutput(raw: unknown): string | null {
  if (typeof raw === "string") {
    if (!raw.trim()) return null;
    const obj = parseOutputObject(raw);
    if (obj) {
      if (typeof obj.text === "string" && obj.image) return obj.text as string;
      const { image: _, ...rest } = obj;
      return JSON.stringify(rest, null, 2);
    }
    return raw;
  }
  if (typeof raw === "object" && raw !== null) {
    const r = raw as Record<string, unknown>;
    if (typeof r.text === "string" && r.image) return r.text as string;
    const { image: _, ...rest } = r;
    return JSON.stringify(rest, null, 2);
  }
  return null;
}

function extractImageUrl(raw: unknown): string | null {
  const obj = parseOutputObject(raw);
  if (!obj) return null;
  const img = obj.image;
  if (typeof img === "string" && img.startsWith("data:image/")) return img;
  return null;
}

function ToolCallBlock({ part, onImageLoad }: { part: ToolInvocationPart; onImageLoad?: () => void }) {
  const [expanded, setExpanded] = useState(false);
  const toolName = part.type.split("-").slice(1).join("-");
  const command = (part.input as { command?: string })?.command ?? toolName;
  const isDone = part.state === "output-available";
  const isRunning = !isDone;
  const output = isDone ? formatOutput(part.output) : null;
  const hasOutput = !!output;
  const imageUrl = isDone ? extractImageUrl(part.output) : null;
  const canExpand = hasOutput && !isRunning;

  return (
    <div className="space-y-1.5">
      <div
        className={cn(
          "rounded-md text-[10px] font-mono overflow-hidden border border-border",
          canExpand && "cursor-pointer",
        )}
        onClick={() => canExpand && setExpanded(!expanded)}
      >
        <div className={cn(
          "px-2 py-1 flex items-center gap-2",
          expanded && hasOutput ? "border-b border-border bg-secondary/30" : "bg-secondary/30",
        )}>
          {isRunning ? (
            <Loader className="size-3 shrink-0 animate-spin text-muted-foreground" />
          ) : (
            <ChevronRight
              className={cn(
                "size-3 shrink-0 text-muted-foreground transition-transform duration-200",
                expanded && "rotate-90",
              )}
            />
          )}
          <span className={cn(
            "truncate",
            isRunning ? "text-foreground/80 shimmer-text" : "text-foreground/80",
          )}>{command}</span>
        </div>
        {expanded && hasOutput && (
          <div className="max-h-[300px] overflow-y-auto">
            <pre className="px-2 py-1.5 text-foreground/80 whitespace-pre-wrap break-all leading-relaxed">
              {truncateOutput(output)}
            </pre>
          </div>
        )}
      </div>
      {imageUrl && (
        <img
          src={imageUrl}
          alt="Screenshot"
          className="rounded-md border border-border max-w-full"
          onLoad={onImageLoad}
        />
      )}
    </div>
  );
}

const DEFAULT_CONTEXT_WINDOW = 128000;

function estimateTokens(text: string): number {
  return Math.ceil(text.length / 4);
}

function formatTokenCount(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return `${n}`;
}

function ContextMeter({ used, total }: { used: number; total: number }) {
  const ratio = Math.min(used / total, 1);
  const size = 16;
  const strokeWidth = 2;
  const r = (size - strokeWidth) / 2;
  const circumference = 2 * Math.PI * r;
  const offset = circumference * (1 - ratio);
  const color =
    ratio > 0.9 ? "text-destructive" : ratio > 0.7 ? "text-yellow-500" : "text-muted-foreground/50";

  return (
    <div
      className="relative shrink-0"
      title={`${formatTokenCount(used)} / ${formatTokenCount(total)} tokens`}
    >
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke="currentColor"
          strokeWidth={strokeWidth}
          className="text-border"
        />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke="currentColor"
          strokeWidth={strokeWidth}
          strokeDasharray={circumference}
          strokeDashoffset={offset}
          strokeLinecap="round"
          className={cn(color, "transition-[stroke-dashoffset] duration-300")}
          transform={`rotate(-90 ${size / 2} ${size / 2})`}
        />
      </svg>
    </div>
  );
}

function useTimeAgo(ts: number | undefined) {
  const [, setTick] = useState(0);
  useEffect(() => {
    if (!ts) return;
    const id = setInterval(() => setTick((t) => t + 1), 30_000);
    return () => clearInterval(id);
  }, [ts]);
  if (!ts) return "";
  const diff = Math.floor((Date.now() - ts) / 1000);
  if (diff < 5) return "just now";
  if (diff < 60) return `${diff}s ago`;
  const mins = Math.floor(diff / 60);
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  return `${hrs}h ago`;
}

function MessageFooter({ provider, timestamp, text }: { provider: string; timestamp?: number; text: string }) {
  const [copied, setCopied] = useState(false);
  const timeAgo = useTimeAgo(timestamp);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(text).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  }, [text]);

  return (
    <div className="flex items-center gap-2 pt-0.5 text-[10px] text-muted-foreground/50">
      <span>{provider}</span>
      {timeAgo && (
        <>
          <span>·</span>
          <span>{timeAgo}</span>
        </>
      )}
      <button
        type="button"
        onClick={handleCopy}
        className="ml-auto hover:text-muted-foreground transition-colors"
        aria-label="Copy message"
      >
        {copied ? <Check className="size-3" /> : <Copy className="size-3" />}
      </button>
    </div>
  );
}

interface CodexInspectionEntry {
  id: string;
  prompt: string;
  observation?: CodexWorkspaceObservation;
  failure?: {
    code?: string;
    message?: string;
  };
  ledger?: {
    runId?: string;
    contextPacketHash?: string;
    eventLogPath?: string | null;
    normalizedEventLogPath?: string | null;
    observationPath?: string | null;
    threadId?: string | null;
    turnId?: string | null;
    appServerReady?: boolean;
    appServerReason?: string;
    appServerTransport?: string | null;
    appServerCliVersion?: string | null;
  };
}

interface CodexInspectResponse {
  success?: boolean;
  error?: string;
  data?: {
    observation?: CodexWorkspaceObservation;
    failure?: CodexInspectionEntry["failure"];
    ledger?: CodexInspectionEntry["ledger"];
  };
}

type DashboardAuthUser = {
  username: string;
  displayName?: string;
  role?: string;
};

type OperatorToolGroup = {
  id: string;
  label: string;
  enabled: boolean;
  reason?: string;
  tools?: string[];
};

type OperatorStatusResponse = {
  success?: boolean;
  error?: string;
  data?: {
    mode?: string;
    provider?: string;
    ready?: boolean;
    authenticatedUser?: DashboardAuthUser;
    toolGroups?: OperatorToolGroup[];
  };
};

type OperatorDashboardAction = {
  id: string;
  label: string;
  kind: string;
  requiresConfirmation?: boolean;
  selection?: DashboardWorkspaceUrlSelection;
  reason?: string;
};

type OperatorToolCall = {
  id: string;
  group: string;
  tool: string;
  status: string;
  summary?: string;
  output?: Record<string, unknown>;
};

type OperatorGuidance = {
  summary?: string;
  targetAssessment?: string;
  recommendedActions?: Array<{
    label: string;
    reason: string;
    toolGroup: string;
    requiresConfirmation: boolean;
  }>;
  risks?: Array<{
    summary: string;
    severity: string;
  }>;
  confirmationRequired?: boolean;
  confidence?: string;
};

type OperatorTurnResponse = {
  success?: boolean;
  error?: string;
  data?: {
    runId?: string;
    mode?: string;
    target?: Record<string, unknown>;
    summary?: string;
    operatorGuidance?: OperatorGuidance;
    operatorGuidanceFailure?: { code?: string; message?: string } | null;
    proposedNextSteps?: Array<{ label: string; reason: string }>;
    toolGroups?: OperatorToolGroup[];
    dashboardActions?: OperatorDashboardAction[];
    toolCalls?: OperatorToolCall[];
    ledger?: unknown;
  };
};

function CodexProviderSummary({
  packet,
  packetSummary,
  packetErrors,
  runStatus,
}: {
  packet: SelectedWorkspaceChatPacket | null;
  packetSummary: string;
  packetErrors: string[];
  runStatus: string;
}) {
  return (
    <div className="rounded-md border border-border/60 bg-secondary/15 px-2 py-1.5 text-[10px]">
      <div className="grid grid-cols-[minmax(0,1fr)_auto] gap-2">
        <div className="min-w-0">
          <div className="flex min-w-0 items-center gap-1.5 text-foreground">
            <ShieldCheck className="size-3 shrink-0 text-primary" />
            <span className="truncate font-medium">{packet?.workspace.label ?? "No workspace selected"}</span>
            {packet?.workspace.state && (
              <span className="rounded border border-border/60 px-1 py-0.5 font-mono text-[9px] text-muted-foreground">
                {packet.workspace.state}
              </span>
            )}
          </div>
          <div className="mt-1 truncate text-muted-foreground">
            {packet ? packetSummary || packet.workspace.label : "Select a workspace to inspect with Chat"}
          </div>
        </div>
        <div className="flex shrink-0 flex-wrap justify-end gap-1">
          <span className="rounded border border-primary/30 bg-primary/10 px-1.5 py-0.5 text-primary">
            Codex app server
          </span>
          <span className="rounded border border-border/60 px-1.5 py-0.5 text-muted-foreground">
            read-only
          </span>
          <span className="rounded border border-border/60 px-1.5 py-0.5 text-muted-foreground">
            {packetErrors.length ? "failed" : packet ? "ready" : "unavailable"}
          </span>
          <span className="rounded border border-border/60 px-1.5 py-0.5 text-muted-foreground">
            {packet?.evidence[0]?.freshness ?? "no evidence"}
          </span>
          <span className="rounded border border-border/60 px-1.5 py-0.5 text-muted-foreground">
            {runStatus}
          </span>
        </div>
      </div>
      {packetErrors.length > 0 && (
        <div className="mt-1 text-destructive/80">
          {packetErrors.join(" ")}
        </div>
      )}
    </div>
  );
}

function CodexEvidenceSelector({
  packet,
  onToggle,
}: {
  packet: SelectedWorkspaceChatPacket | null;
  onToggle: (source: SelectedWorkspaceChatEvidenceSource, included: boolean) => void;
}) {
  if (!packet) {
    return (
      <div className="rounded-md border border-border/60 px-2 py-1.5 text-[10px] text-muted-foreground">
        Evidence selector unavailable until a workspace is selected.
      </div>
    );
  }
  return (
    <div
      className="grid grid-cols-2 gap-1 text-[10px] sm:grid-cols-3"
      data-chat-evidence-selector="ready"
    >
      {packet.evidence.map((item) => (
        <label
          key={item.id}
          title={item.unavailableReason ?? item.summary}
          className={cn(
            "flex min-w-0 items-center gap-1 rounded border px-1.5 py-1",
            item.available
              ? "border-border/60 bg-secondary/20 text-foreground"
              : "border-border/40 bg-background/30 text-muted-foreground/60",
          )}
        >
          <input
            type="checkbox"
            checked={item.included}
            disabled={!item.available}
            onChange={(event) => onToggle(item.source, event.target.checked)}
            className="size-3 shrink-0 accent-primary"
          />
          <span className="min-w-0 truncate">{item.sourceLabel}</span>
          <span className="ml-auto shrink-0 font-mono text-[9px] text-muted-foreground">
            {item.available ? item.freshness : "unavailable"}
          </span>
        </label>
      ))}
    </div>
  );
}

function CodexInspectionBlock({ entry }: { entry: CodexInspectionEntry }) {
  const { observation, failure, ledger } = entry;
  const evidenceRefs = observation ? observationEvidenceRefs(observation) : [];
  return (
    <div className="space-y-2 rounded-md border border-border/60 bg-background/40 p-2 text-xs">
      <div className="space-y-1">
        <div className="text-[10px] uppercase text-muted-foreground">Request</div>
        <div className="text-muted-foreground whitespace-pre-wrap">{entry.prompt}</div>
      </div>
      {observation ? (
        <div className="grid gap-2 md:grid-cols-2">
          <div className="space-y-1 md:col-span-2">
            <div className="text-[10px] uppercase text-muted-foreground">Summary</div>
            <div className="text-foreground leading-relaxed">{observation.summary}</div>
          </div>
          <div className="space-y-1">
            <div className="text-[10px] uppercase text-muted-foreground">Detected state</div>
            <div className="rounded border border-border/50 px-2 py-1 text-[11px] text-muted-foreground">
              {observation.detectedState}
            </div>
          </div>
          <div className="space-y-1">
            <div className="text-[10px] uppercase text-muted-foreground">Evidence references</div>
            <div className="flex flex-wrap gap-1">
              {evidenceRefs.length ? evidenceRefs.map((ref) => (
                <span key={ref} className="rounded border border-border/50 px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
                  {ref}
                </span>
              )) : (
                <span className="text-[11px] text-destructive/80">No evidence references returned</span>
              )}
            </div>
          </div>
        </div>
      ) : (
        <div className="space-y-1">
          <div className="text-[10px] uppercase text-destructive/80">Inspection failure</div>
          <div className="text-destructive/90 leading-relaxed">
            {failure?.code ? `${failure.code}: ` : ""}{failure?.message ?? "Codex app-server inspection failed."}
          </div>
        </div>
      )}
      {observation && observation.blockers.length > 0 && (
        <CodexObservationList
          title="Blockers"
          items={observation.blockers.map((item) => `${item.severity}: ${item.summary}`)}
        />
      )}
      {observation && observation.risks.length > 0 && (
        <CodexObservationList title="Risks" items={observation.risks.map((item) => item.summary)} />
      )}
      {observation && observation.suggestedNextInspections.length > 0 && (
        <CodexObservationList
          title="Suggested next read-only inspections"
          items={observation.suggestedNextInspections.map((item) => `${item.label}: ${item.reason}`)}
        />
      )}
      <div className="space-y-1">
        <div className="text-[10px] uppercase text-muted-foreground">Run ledger</div>
        <div className="flex flex-wrap items-center gap-1.5 text-[10px] text-muted-foreground">
          <span>{observation?.provider ?? CONTEXTUAL_CHAT_PROVIDER_ID}</span>
          {ledger?.runId && <span className="font-mono">run {ledger.runId.slice(0, 8)}</span>}
          {observation && <span>{observation.confidence} confidence</span>}
          {ledger?.threadId && <span className="font-mono">thread {ledger.threadId.slice(0, 8)}</span>}
          {ledger?.turnId && <span className="font-mono">turn {ledger.turnId.slice(0, 8)}</span>}
          {ledger?.contextPacketHash && <span className="font-mono">packet {ledger.contextPacketHash.slice(0, 12)}</span>}
          <span>{observation ? "validation passed" : "validation failed"}</span>
        </div>
        {(ledger?.eventLogPath || ledger?.normalizedEventLogPath || ledger?.observationPath) && (
          <details className="rounded border border-border/50 px-2 py-1 text-[10px] text-muted-foreground">
            <summary className="cursor-pointer">event log</summary>
            <div className="mt-1 space-y-0.5 font-mono break-all">
              {ledger.eventLogPath && <div>raw {ledger.eventLogPath}</div>}
              {ledger.normalizedEventLogPath && <div>normalized {ledger.normalizedEventLogPath}</div>}
              {ledger.observationPath && <div>observation {ledger.observationPath}</div>}
            </div>
          </details>
        )}
      </div>
    </div>
  );
}

function observationEvidenceRefs(observation: CodexWorkspaceObservation): string[] {
  return Array.from(new Set([
    ...observation.blockers.flatMap((item) => item.evidenceIds),
    ...observation.risks.flatMap((item) => item.evidenceIds),
    ...observation.suggestedNextInspections.flatMap((item) => item.evidenceIds),
  ].filter(Boolean)));
}

function CodexObservationList({ title, items }: { title: string; items: string[] }) {
  return (
    <div className="space-y-1">
      <div className="text-[10px] uppercase text-muted-foreground">{title}</div>
      <div className="space-y-1">
        {items.map((item, index) => (
          <div key={`${title}-${index}`} className="rounded border border-border/50 px-2 py-1 text-[11px] text-muted-foreground">
            {item}
          </div>
        ))}
      </div>
    </div>
  );
}

function OperatorStatusBlock({
  user,
  status,
  latestTurn,
  error,
  onDashboardAction,
  appliedActionId,
}: {
  user: DashboardAuthUser;
  status: OperatorStatusResponse["data"] | null;
  latestTurn: OperatorTurnResponse["data"] | null;
  error: string | null;
  onDashboardAction: (action: OperatorDashboardAction) => void;
  appliedActionId: string | null;
}) {
  const groups = latestTurn?.toolGroups ?? status?.toolGroups ?? [];
  const toolCalls = latestTurn?.toolCalls ?? [];
  const dashboardActions = latestTurn?.dashboardActions ?? [];
  const guidance = latestTurn?.operatorGuidance;
  const guidanceFailure = latestTurn?.operatorGuidanceFailure;
  const target = latestTurn?.target ?? {};
  const targetFacts = [
    ["Workspace", target.workspaceId],
    ["Browser", target.browserId],
    ["Session", target.sessionId],
    ["Tab", target.tabId],
    ["Profile", target.profileId],
    ["State", target.state],
  ].filter(([, value]) => typeof value === "string" && value.length > 0) as Array<[string, string]>;
  return (
    <div
      className="space-y-2 rounded-md border border-primary/30 bg-primary/5 p-2 text-xs"
      data-superuser-operator-agent="ready"
    >
      <div className="grid grid-cols-[minmax(0,1fr)_auto] gap-2">
        <div className="min-w-0">
          <div className="text-[10px] uppercase text-muted-foreground">Operate</div>
          <div className="truncate font-medium text-foreground">Superuser operator</div>
          <div className="truncate text-[10px] text-muted-foreground">
            {user.displayName || user.username} · {user.role}
          </div>
        </div>
        <div className="flex flex-wrap justify-end gap-1 text-[10px]">
          <span className="rounded border border-primary/30 bg-primary/10 px-1.5 py-0.5 text-primary">
            Codex app server
          </span>
          <span className="rounded border border-border/60 px-1.5 py-0.5 text-muted-foreground">
            {status?.ready ? "ready" : "starting"}
          </span>
          <span className="rounded border border-border/60 px-1.5 py-0.5 text-muted-foreground">
            audited
          </span>
        </div>
      </div>
      <div className="grid gap-1 sm:grid-cols-2">
        {groups.map((group) => (
          <div key={group.id} className="rounded border border-border/60 px-2 py-1">
            <div className="flex items-center gap-2">
              <span className="truncate text-[11px] font-medium">{group.label}</span>
              <span className="ml-auto shrink-0 rounded border border-border/50 px-1 py-0.5 text-[9px] text-muted-foreground">
                {group.enabled ? "enabled" : "disabled"}
              </span>
            </div>
            <div className="mt-0.5 truncate text-[10px] text-muted-foreground">
              {group.reason ?? `${group.tools?.length ?? 0} tools`}
            </div>
          </div>
        ))}
      </div>
      {latestTurn && (
        <div className="space-y-1 rounded border border-border/60 px-2 py-1">
          <div className="flex items-center gap-2 text-[10px] text-muted-foreground">
            <span className="uppercase">Operator turn</span>
            {latestTurn.runId && <span className="font-mono">{latestTurn.runId.slice(0, 18)}</span>}
          </div>
          <div className="text-[11px] text-foreground">{latestTurn.summary}</div>
          {guidance && (
            <div className="space-y-1 rounded border border-primary/20 bg-primary/5 px-2 py-1 text-[10px]">
              <div className="flex items-center gap-2">
                <span className="uppercase text-muted-foreground">Codex operator guidance</span>
                {guidance.confidence && (
                  <span className="ml-auto rounded border border-border/50 px-1 py-0.5 text-[9px] text-muted-foreground">
                    {guidance.confidence}
                  </span>
                )}
              </div>
              {guidance.targetAssessment && (
                <div className="text-foreground">{guidance.targetAssessment}</div>
              )}
              {guidance.recommendedActions?.length ? (
                <div className="grid gap-1">
                  {guidance.recommendedActions.map((action) => (
                    <div key={`${action.toolGroup}-${action.label}`} className="rounded border border-border/50 px-1.5 py-1">
                      <div className="flex min-w-0 items-center gap-1.5">
                        <span className="truncate font-medium text-foreground">{action.label}</span>
                        <span className="ml-auto shrink-0 font-mono text-[9px] text-muted-foreground">
                          {action.toolGroup}
                        </span>
                      </div>
                      <div className="mt-0.5 text-muted-foreground">{action.reason}</div>
                    </div>
                  ))}
                </div>
              ) : null}
              {guidance.risks?.length ? (
                <div className="flex flex-wrap gap-1">
                  {guidance.risks.map((risk) => (
                    <span key={risk.summary} className="rounded border border-border/50 px-1.5 py-0.5 text-muted-foreground">
                      {risk.severity}: {risk.summary}
                    </span>
                  ))}
                </div>
              ) : null}
            </div>
          )}
          {guidanceFailure?.message && (
            <div className="rounded border border-destructive/40 bg-destructive/10 px-2 py-1 text-[10px] text-destructive/90">
              {guidanceFailure.code ? `${guidanceFailure.code}: ` : ""}{guidanceFailure.message}
            </div>
          )}
          {targetFacts.length ? (
            <div className="grid grid-cols-2 gap-1 text-[10px] sm:grid-cols-3">
              {targetFacts.map(([label, value]) => (
                <div key={label} className="min-w-0 rounded border border-border/50 px-1.5 py-0.5">
                  <span className="text-muted-foreground">{label}</span>{" "}
                  <span className="font-mono text-foreground break-all">{value}</span>
                </div>
              ))}
            </div>
          ) : null}
          {toolCalls.length ? (
            <div className="space-y-1">
              <div className="text-[10px] uppercase text-muted-foreground">Tool calls</div>
              <div className="grid gap-1">
                {toolCalls.map((call) => (
                  <div key={call.id} className="rounded border border-border/50 px-2 py-1 text-[10px]">
                    <div className="flex min-w-0 items-center gap-2">
                      <span className="truncate font-mono text-foreground">{call.tool}</span>
                      <span className="ml-auto shrink-0 rounded border border-border/50 px-1 py-0.5 text-[9px] text-muted-foreground">
                        {call.status}
                      </span>
                    </div>
                    {call.summary && <div className="mt-0.5 text-muted-foreground">{call.summary}</div>}
                  </div>
                ))}
              </div>
            </div>
          ) : null}
          {dashboardActions.length ? (
            <div className="space-y-1">
              <div className="text-[10px] uppercase text-muted-foreground">Dashboard actions</div>
              <div className="flex flex-wrap gap-1">
                {dashboardActions.map((action) => (
                  <button
                    key={action.id}
                    type="button"
                    onClick={() => onDashboardAction(action)}
                    className="rounded-md border border-primary/40 bg-primary/10 px-2 py-1 text-[10px] text-primary transition-colors hover:bg-primary/15"
                    title={action.reason}
                  >
                    {appliedActionId === action.id ? "Applied" : action.label}
                  </button>
                ))}
              </div>
            </div>
          ) : null}
          {latestTurn.proposedNextSteps?.length ? (
            <div className="space-y-1">
              {latestTurn.proposedNextSteps.map((step) => (
                <div key={step.label} className="text-[10px] text-muted-foreground">
                  <span className="font-medium text-foreground">{step.label}</span>: {step.reason}
                </div>
              ))}
            </div>
          ) : null}
        </div>
      )}
      {error && (
        <div className="rounded border border-destructive/40 bg-destructive/10 px-2 py-1 text-[10px] text-destructive/90">
          {error}
        </div>
      )}
    </div>
  );
}

export function ChatPanel({
  selectedWorkspaceContext,
  authenticatedUser,
}: {
  selectedWorkspaceContext?: SelectedWorkspaceContext | null;
  authenticatedUser?: DashboardAuthUser | null;
} = {}) {
  const [input, setInput] = useState("");
  const [chatMode, setChatMode] = useState<"inspect" | "operate">("inspect");
  const [errorDismissed, setErrorDismissed] = useState(false);
  const [codexInspections, setCodexInspections] = useState<CodexInspectionEntry[]>([]);
  const [codexError, setCodexError] = useState<string | null>(null);
  const [codexSubmitting, setCodexSubmitting] = useState(false);
  const [operatorStatus, setOperatorStatus] = useState<OperatorStatusResponse["data"] | null>(null);
  const [operatorTurn, setOperatorTurn] = useState<OperatorTurnResponse["data"] | null>(null);
  const [operatorError, setOperatorError] = useState<string | null>(null);
  const [operatorSubmitting, setOperatorSubmitting] = useState(false);
  const [appliedOperatorActionId, setAppliedOperatorActionId] = useState<string | null>(null);
  const [evidenceInclude, setEvidenceInclude] = useState<Partial<Record<SelectedWorkspaceChatEvidenceSource, boolean>>>(DEFAULT_EVIDENCE_INCLUDE);
  const [copiedArtifact, setCopiedArtifact] = useState<"observation" | "packet" | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const sessionName = useAtomValue(activeSessionNameAtom);
  const chatId = sessionName || "default";
  const storageKey = `${STORAGE_PREFIX}${chatId}`;
  const sessionRef = useRef(chatId);
  sessionRef.current = chatId;
  const messageTimestamps = useRef<Record<string, number>>({});

  const transport = useRef(
    new DefaultChatTransport({
      api: getChatApiUrl(),
      body: () => ({
        session: sessionRef.current,
        provider: CONTEXTUAL_CHAT_PROVIDER_ID,
      }),
    }),
  ).current;

  const { messages, stop, status, setMessages, error } = useChat({
    id: chatId,
    transport,
    onError: () => setErrorDismissed(false),
  });

  const visibleError = error && !errorDismissed ? error : undefined;
  const isSuperuser = authenticatedUser?.role === "superuser";
  const isLoading = status === "streaming" || status === "submitted" || codexSubmitting || operatorSubmitting;
  const hasMessages = messages.length > 0 || codexInspections.length > 0 || !!visibleError || !!codexError || !!operatorTurn || !!operatorError;
  const selectedWorkspacePacket = useMemo(() => {
    if (!selectedWorkspaceContext) return null;
    return buildSelectedWorkspaceChatPacket(selectedWorkspaceContext, { include: evidenceInclude });
  }, [selectedWorkspaceContext, evidenceInclude]);
  const selectedWorkspacePacketErrors = useMemo(
    () => (selectedWorkspacePacket ? validateSelectedWorkspaceChatPacket(selectedWorkspacePacket) : []),
    [selectedWorkspacePacket],
  );
  const packetSummary = useMemo(
    () => (selectedWorkspacePacket ? selectedWorkspaceChatPacketSummary(selectedWorkspacePacket) : ""),
    [selectedWorkspacePacket],
  );
  const latestCodexInspection = codexInspections[codexInspections.length - 1] ?? null;
  const latestRunStatus = codexSubmitting
    ? "running"
    : operatorSubmitting
      ? "operator running"
    : latestCodexInspection?.observation
      ? "succeeded"
      : operatorTurn
        ? "operator staged"
      : latestCodexInspection?.failure
        ? "failed"
        : codexError || operatorError
          ? "failed"
          : "idle";

  useEffect(() => {
    if (!isSuperuser && chatMode === "operate") {
      setChatMode("inspect");
    }
  }, [chatMode, isSuperuser]);

  useEffect(() => {
    if (!isSuperuser || chatMode !== "operate" || operatorStatus) return;
    let cancelled = false;
    async function loadOperatorStatus() {
      try {
        const response = await fetch(APP_INTELLIGENCE_OPERATOR_STATUS_API_URL, {
          cache: "no-store",
          credentials: "same-origin",
        });
        const payload = (await response.json()) as OperatorStatusResponse;
        if (cancelled) return;
        if (!response.ok || !payload.success || !payload.data) {
          setOperatorError(payload.error || `Operator status failed with HTTP ${response.status}`);
          return;
        }
        setOperatorStatus(payload.data);
      } catch (err) {
        if (!cancelled) setOperatorError(err instanceof Error ? err.message : String(err));
      }
    }
    void loadOperatorStatus();
    return () => {
      cancelled = true;
    };
  }, [chatMode, isSuperuser, operatorStatus]);

  useEffect(() => {
    for (const msg of messages) {
      if (msg.role === "assistant" && !messageTimestamps.current[msg.id]) {
        messageTimestamps.current[msg.id] = Date.now();
      }
    }
  }, [messages]);

  const estimatedTokens = useMemo(() => {
    let total = 0;
    for (const msg of messages) {
      for (const part of msg.parts) {
        if (part.type === "text") total += estimateTokens(part.text);
        else if (isToolPart(part)) {
          if (part.input) total += estimateTokens(JSON.stringify(part.input));
          if (part.output) {
            const raw = typeof part.output === "string" ? part.output : JSON.stringify(part.output);
            const stripped = raw.replace(/"image"\s*:\s*"data:[^"]*"/g, '"image":"[omitted]"');
            total += estimateTokens(stripped);
          }
        }
      }
    }
    if (selectedWorkspacePacket) total += estimateTokens(JSON.stringify(selectedWorkspacePacket));
    for (const entry of codexInspections) total += estimateTokens(JSON.stringify(entry.observation));
    return total;
  }, [messages, selectedWorkspacePacket, codexInspections]);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const scrollToBottom = useCallback(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, []);

  useEffect(() => {
    scrollToBottom();
  }, [messages, codexInspections, visibleError, codexError, scrollToBottom]);

  // Restore messages from localStorage when chatId changes
  useEffect(() => {
    try {
      const stored = localStorage.getItem(storageKey);
      if (stored) {
        const parsed = JSON.parse(stored);
        if (Array.isArray(parsed) && parsed.length > 0) {
          setMessages(parsed);
          return;
        }
      }
    } catch {
      // ignore
    }
    setMessages([]);
  }, [chatId, storageKey, setMessages]);

  // Persist messages to localStorage (strip base64 images to save space)
  useEffect(() => {
    if (isLoading) return;
    if (messages.length === 0) {
      localStorage.removeItem(storageKey);
      return;
    }
    try {
      localStorage.setItem(storageKey, JSON.stringify(stripImagesForStorage(messages)));
    } catch {
      // ignore quota
    }
  }, [messages, isLoading, storageKey]);

  const inspectWithCodex = useCallback(async (prompt: string, packet: SelectedWorkspaceChatPacket) => {
    setCodexSubmitting(true);
    setCodexError(null);
    try {
      const response = await fetch(APP_INTELLIGENCE_INSPECT_API_URL, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          provider: CONTEXTUAL_CHAT_PROVIDER_ID,
          prompt,
          packet,
        }),
      });
      const data = (await response.json()) as CodexInspectResponse;
      if (!response.ok && !data.data?.failure) {
        throw new Error(data.error || `Codex inspection failed with HTTP ${response.status}`);
      }
      if (!data.data?.observation && !data.data?.failure) {
        throw new Error(data.error || `Codex inspection failed with HTTP ${response.status}`);
      }
      const observation = data.data.observation;
      const failure = data.data.failure;
      const ledger = data.data.ledger;
      setCodexInspections((prev) => [
        ...prev,
        {
          id: observation?.runId ?? ledger?.runId ?? `codex-inspection-${Date.now()}`,
          prompt,
          observation,
          failure,
          ledger,
        },
      ]);
    } catch (err) {
      setCodexError(err instanceof Error ? err.message : String(err));
    } finally {
      setCodexSubmitting(false);
    }
  }, []);

  const toggleEvidence = useCallback((source: SelectedWorkspaceChatEvidenceSource, included: boolean) => {
    setEvidenceInclude((prev) => ({ ...prev, [source]: included }));
  }, []);

  const runDefaultInspection = useCallback(() => {
    const prompt = "Inspect selected workspace";
    if (!selectedWorkspacePacket) {
      setCodexError("Select a workspace before using contextual Chat.");
      return;
    }
    if (selectedWorkspacePacketErrors.length > 0) {
      setCodexError(selectedWorkspacePacketErrors.join(" "));
      return;
    }
    void inspectWithCodex(prompt, selectedWorkspacePacket);
  }, [inspectWithCodex, selectedWorkspacePacket, selectedWorkspacePacketErrors]);

  const runOperatorTurn = useCallback(async (prompt: string) => {
    if (!isSuperuser) {
      setOperatorError("Superuser role required.");
      return;
    }
    setOperatorSubmitting(true);
    setOperatorError(null);
    try {
      const response = await fetch(APP_INTELLIGENCE_OPERATOR_TURN_API_URL, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "same-origin",
        body: JSON.stringify({
          prompt,
          packet: selectedWorkspacePacket,
        }),
      });
      const payload = (await response.json()) as OperatorTurnResponse;
      if (!response.ok || !payload.success || !payload.data) {
        throw new Error(payload.error || `Operator turn failed with HTTP ${response.status}`);
      }
      setOperatorTurn(payload.data);
      setAppliedOperatorActionId(null);
      if (payload.data.toolGroups) {
        setOperatorStatus((prev) => prev ? { ...prev, toolGroups: payload.data?.toolGroups } : prev);
      }
    } catch (err) {
      setOperatorError(err instanceof Error ? err.message : String(err));
    } finally {
      setOperatorSubmitting(false);
    }
  }, [isSuperuser, selectedWorkspacePacket]);

  const applyOperatorDashboardAction = useCallback((action: OperatorDashboardAction) => {
    if (action.kind === "set_selected_workspace" && action.selection) {
      updateDashboardWorkspaceUrlSelection(action.selection, "push");
      setAppliedOperatorActionId(action.id);
      return;
    }
    setOperatorError(`Unsupported dashboard action: ${action.kind}`);
  }, []);

  const copyArtifact = useCallback(async (artifact: "observation" | "packet") => {
    const payload = artifact === "packet"
      ? selectedWorkspacePacket
      : latestCodexInspection
        ? {
            observation: latestCodexInspection.observation ?? null,
            failure: latestCodexInspection.failure ?? null,
            ledger: latestCodexInspection.ledger ?? null,
          }
        : null;
    if (!payload) return;
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    setCopiedArtifact(artifact);
    setTimeout(() => setCopiedArtifact(null), 2000);
  }, [latestCodexInspection, selectedWorkspacePacket]);

  const handleSubmit = useCallback(
    (e: React.FormEvent) => {
      e.preventDefault();
      if (!input.trim() || isLoading) return;
      if (chatMode === "operate") {
        void runOperatorTurn(input.trim());
        setInput("");
        return;
      }
      if (!selectedWorkspacePacket) {
        setCodexError("Select a workspace before using contextual Chat.");
        return;
      }
      if (selectedWorkspacePacketErrors.length > 0) {
        setCodexError(selectedWorkspacePacketErrors.join(" "));
        return;
      }
      void inspectWithCodex(input.trim(), selectedWorkspacePacket);
      setInput("");
    },
    [chatMode, input, isLoading, selectedWorkspacePacket, selectedWorkspacePacketErrors, inspectWithCodex, runOperatorTurn],
  );

  const lastCompactedId = useRef<string | null>(null);
  useEffect(() => {
    if (isLoading || messages.length === 0) return;
    const lastAssistant = [...messages].reverse().find((m) => m.role === "assistant");
    if (!lastAssistant) return;
    if (lastAssistant.id === lastCompactedId.current) return;
    const meta = (lastAssistant as any).metadata as
      | { compacted?: boolean; summary?: string; keepLastN?: number }
      | undefined;
    if (!meta?.compacted || typeof meta.keepLastN !== "number") return;

    lastCompactedId.current = lastAssistant.id;
    const keep = meta.keepLastN;
    if (keep >= messages.length) return;

    const summaryMsg = {
      id: `compaction-${Date.now()}`,
      role: "assistant" as const,
      parts: [
        {
          type: "text" as const,
          text: `*Earlier messages were summarized to stay within the context window.*`,
        },
      ],
    };

    const kept = messages.slice(messages.length - keep);
    setMessages([summaryMsg as any, ...kept]);
  }, [isLoading, messages, setMessages]);

  const handleClear = useCallback(() => {
    setMessages([]);
    setCodexInspections([]);
    setCodexError(null);
    setOperatorTurn(null);
    setOperatorError(null);
    setErrorDismissed(true);
    localStorage.removeItem(storageKey);
    requestAnimationFrame(() => inputRef.current?.focus());
  }, [setMessages, storageKey]);

  const handleDownload = useCallback(() => {
    const data = messages.map((msg) => ({
      id: msg.id,
      role: msg.role,
      parts: msg.parts.map((p) => {
        if (p.type === "text") return { type: "text", text: p.text };
        if (p.type === "file") return { type: "file", filename: (p as any).filename };
        if (isToolPart(p)) {
          const out = typeof p.output === "string" ? p.output : JSON.stringify(p.output);
          const stripped = out?.replace(/"image":"data:[^"]*"/g, '"image":"[stripped]"');
          return {
            type: p.type,
            toolName: (p as any).toolName,
            state: (p as any).state,
            input: (p as any).input,
            output: stripped,
          };
        }
        return { type: p.type };
      }),
    }));
    const json = JSON.stringify({
      session: chatId,
      provider: CONTEXTUAL_CHAT_PROVIDER_ID,
      selectedWorkspacePacket,
      codexInspections,
      messages: data,
    }, null, 2);
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `chat-${chatId}-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }, [messages, chatId, selectedWorkspacePacket, codexInspections]);

  const hasVisibleContent = (parts: (typeof messages)[number]["parts"]): boolean => {
    return parts.some(
      (p) => (p.type === "text" && p.text.length > 0) || p.type === "file" || isToolPart(p),
    );
  };

  return (
    <div
      className="flex h-full flex-col"
      data-selected-workspace-id={selectedWorkspaceContext?.node?.id ?? ""}
      data-selected-workspace-state={selectedWorkspaceContext?.state ?? ""}
      data-codex-app-server-contextual-chat="ready"
      data-contextual-chat-provider={CONTEXTUAL_CHAT_PROVIDER_ID}
    >
      {hasMessages && (
        <div className="flex items-center justify-end gap-2 px-3 py-1.5 shrink-0 border-b border-border/40">
          <button
            onClick={handleDownload}
            className="text-muted-foreground hover:text-foreground transition-colors shrink-0"
            aria-label="Download conversation"
          >
            <Download className="size-3" />
          </button>
          <button
            onClick={handleClear}
            className="text-muted-foreground hover:text-foreground transition-colors shrink-0"
            aria-label="Clear conversation"
          >
            <Trash2 className="size-3" />
          </button>
        </div>
      )}

      <ScrollArea className="flex-1 min-h-0">
        <div className="p-3 space-y-3">
          <div className="space-y-2">
            <div className="flex flex-wrap items-center gap-1 text-[10px]" data-chat-mode-selector="ready">
              <button
                type="button"
                onClick={() => setChatMode("inspect")}
                className={cn(
                  "rounded-md border px-2 py-1 transition-colors",
                  chatMode === "inspect"
                    ? "border-primary/40 bg-primary/10 text-primary"
                    : "border-border/60 bg-secondary/20 text-muted-foreground hover:text-foreground",
                )}
              >
                Inspect
              </button>
              {isSuperuser && (
                <button
                  type="button"
                  onClick={() => setChatMode("operate")}
                  className={cn(
                    "rounded-md border px-2 py-1 transition-colors",
                    chatMode === "operate"
                      ? "border-primary/40 bg-primary/10 text-primary"
                      : "border-border/60 bg-secondary/20 text-muted-foreground hover:text-foreground",
                  )}
                >
                  Operate
                </button>
              )}
              {isSuperuser && (
                <span className="rounded border border-border/60 px-1.5 py-0.5 text-muted-foreground">
                  superuser
                </span>
              )}
            </div>
            <CodexProviderSummary
              packet={selectedWorkspacePacket}
              packetSummary={packetSummary}
              packetErrors={selectedWorkspacePacketErrors}
              runStatus={latestRunStatus}
            />
            {chatMode === "operate" && isSuperuser && authenticatedUser && (
              <OperatorStatusBlock
                user={authenticatedUser}
                status={operatorStatus}
                latestTurn={operatorTurn}
                error={operatorError}
                onDashboardAction={applyOperatorDashboardAction}
                appliedActionId={appliedOperatorActionId}
              />
            )}
            <CodexEvidenceSelector packet={selectedWorkspacePacket} onToggle={toggleEvidence} />
            <div className="flex flex-wrap gap-1.5" data-chat-prompt-actions="ready">
              <button
                type="button"
                onClick={runDefaultInspection}
                disabled={chatMode !== "inspect" || isLoading || !selectedWorkspacePacket || selectedWorkspacePacketErrors.length > 0}
                className="inline-flex items-center gap-1 rounded-md border border-primary/40 bg-primary/10 px-2 py-1 text-[10px] text-primary transition-colors hover:bg-primary/15 disabled:opacity-40"
              >
                <Search className="size-3" />
                Inspect selected workspace
              </button>
              {chatMode === "operate" && isSuperuser && (
                <button
                  type="button"
                  onClick={() => void runOperatorTurn(input.trim() || "Plan the next safe operator action for the selected workspace.")}
                  disabled={operatorSubmitting}
                  className="inline-flex items-center gap-1 rounded-md border border-primary/40 bg-primary/10 px-2 py-1 text-[10px] text-primary transition-colors hover:bg-primary/15 disabled:opacity-40"
                >
                  <Search className="size-3" />
                  Plan operator action
                </button>
              )}
              <button
                type="button"
                onClick={() => inputRef.current?.focus()}
                className="rounded-md border border-border/60 bg-secondary/30 px-2 py-1 text-[10px] text-muted-foreground transition-colors hover:text-foreground"
              >
                Ask follow-up
              </button>
              <button
                type="button"
                onClick={() => void copyArtifact("observation")}
                disabled={!latestCodexInspection}
                className="inline-flex items-center gap-1 rounded-md border border-border/60 bg-secondary/30 px-2 py-1 text-[10px] text-muted-foreground transition-colors hover:text-foreground disabled:opacity-40"
              >
                {copiedArtifact === "observation" ? <Check className="size-3" /> : <Copy className="size-3" />}
                Copy observation
              </button>
              <button
                type="button"
                onClick={() => void copyArtifact("packet")}
                disabled={!selectedWorkspacePacket}
                className="inline-flex items-center gap-1 rounded-md border border-border/60 bg-secondary/30 px-2 py-1 text-[10px] text-muted-foreground transition-colors hover:text-foreground disabled:opacity-40"
              >
                {copiedArtifact === "packet" ? <Check className="size-3" /> : <Copy className="size-3" />}
                Copy evidence packet
              </button>
            </div>
            {!hasMessages && !isLoading && (
              <div className="flex flex-wrap gap-1.5">
                {SUGGESTIONS.map((s) => (
                  <button
                    key={s}
                    type="button"
                    onClick={() => {
                      setInput(s);
                      if (selectedWorkspacePacket && selectedWorkspacePacketErrors.length === 0) {
                        void inspectWithCodex(s, selectedWorkspacePacket);
                      }
                    }}
                    className="text-[10px] px-2 py-1 rounded-md border bg-secondary/50 text-muted-foreground hover:text-foreground hover:bg-secondary transition-colors"
                  >
                    {s}
                  </button>
                ))}
              </div>
            )}
          </div>

          {codexInspections.map((entry) => (
            <CodexInspectionBlock key={entry.id} entry={entry} />
          ))}

          {messages.map((message) => {
            if (message.id.startsWith("compaction-")) {
              return (
                <div key={message.id} className="flex items-center gap-2 text-[10px] text-muted-foreground/60">
                  <div className="flex-1 border-t border-border/40" />
                  <span>Earlier messages summarized</span>
                  <div className="flex-1 border-t border-border/40" />
                </div>
              );
            }
            if (!hasVisibleContent(message.parts)) return null;
            return (
              <div key={message.id}>
                {message.role === "user" ? (
                  <div className="space-y-1.5">
                    {message.parts.some((p) => p.type === "file") && (
                      <div className="flex flex-wrap gap-1.5">
                        {message.parts
                          .filter((p): p is Extract<typeof p, { type: "file" }> => p.type === "file")
                          .map((p, i) => (
                            <img
                              key={i}
                              src={p.url}
                              alt={p.filename ?? "uploaded image"}
                              className="max-h-24 rounded-md border border-border object-cover"
                            />
                          ))}
                      </div>
                    )}
                    <div className="text-xs text-muted-foreground whitespace-pre-wrap leading-relaxed">
                      {message.parts
                        .filter((p): p is Extract<typeof p, { type: "text" }> => p.type === "text")
                        .map((p) => p.text)
                        .join("")}
                    </div>
                  </div>
                ) : (
                  <div className="space-y-1.5">
                    {(() => {
                      type Group = { type: "tools" | "text"; items: (typeof message.parts)[number][] };
                      const groups: Group[] = [];
                      for (const part of message.parts) {
                        const groupType = isToolPart(part) ? "tools" : "text";
                        const last = groups[groups.length - 1];
                        if (last && last.type === groupType) {
                          last.items.push(part);
                        } else {
                          groups.push({ type: groupType, items: [part] });
                        }
                      }

                      return groups.map((group, gi) => {
                        if (group.type === "tools") {
                          return (
                            <div key={gi} className="space-y-0.5">
                              {group.items.map((part) => {
                                if (!isToolPart(part)) return null;
                                return <ToolCallBlock key={part.toolCallId} part={part} onImageLoad={scrollToBottom} />;
                              })}
                            </div>
                          );
                        }
                        const combinedText = group.items
                          .filter((p): p is Extract<typeof p, { type: "text" }> => p.type === "text" && !!p.text)
                          .map((p) => p.text)
                          .join("");
                        if (!combinedText) return null;
                        return (
                          <div key={gi} className="text-xs text-foreground">
                            <Streamdown
                              shikiTheme={shikiTheme}
                              controls={false}
                              components={chatComponents}
                            >
                              {combinedText}
                            </Streamdown>
                          </div>
                        );
                      });
                    })()}
                    {(() => {
                      const isLast = message === messages[messages.length - 1];
                      const isComplete = !isLast || !isLoading;
                      if (!isComplete) return null;
                      const fullText = message.parts
                        .filter((p): p is Extract<typeof p, { type: "text" }> => p.type === "text" && !!p.text)
                        .map((p) => p.text)
                        .join("");
                      return (
                          <MessageFooter
                          provider={CONTEXTUAL_CHAT_PROVIDER_ID}
                          timestamp={messageTimestamps.current[message.id]}
                          text={fullText}
                        />
                      );
                    })()}
                  </div>
                )}
              </div>
            );
          })}

          {isLoading && messages.length > 0 && (() => {
            const lastMsg = messages[messages.length - 1];
            const lastPart = lastMsg?.parts[lastMsg.parts.length - 1];
            const noVisibleContent = !lastMsg || !hasVisibleContent(lastMsg.parts);
            const lastIsCompletedTool = lastPart && isToolPart(lastPart) && lastPart.state === "output-available";
            if (noVisibleContent || lastIsCompletedTool) {
              return (
                <span className="text-[11px] text-muted-foreground shimmer-text">
                  Working...
                </span>
              );
            }
            return null;
          })()}

          {visibleError && (
            <div className="text-[10px] text-destructive/80 bg-destructive/10 rounded-md px-2 py-1.5">
              {(() => {
                try {
                  const parsed = JSON.parse(visibleError.message);
                  return parsed.message || parsed.error || visibleError.message;
                } catch {
                  return visibleError.message || "Something went wrong.";
                }
              })()}
            </div>
          )}

          {codexError && (
            <div className="text-[10px] text-destructive/80 bg-destructive/10 rounded-md px-2 py-1.5">
              {codexError}
            </div>
          )}

          <div ref={messagesEndRef} />
        </div>
      </ScrollArea>

      <div className="shrink-0 border-t border-border">
        <form onSubmit={handleSubmit}>
          <div className="px-3 pt-2 pb-1.5">
            <textarea
              ref={inputRef}
              value={input}
              onChange={(e) => {
                setInput(e.target.value);
                e.target.style.height = "auto";
                e.target.style.height = `${e.target.scrollHeight}px`;
              }}
              rows={1}
              placeholder={chatMode === "operate" ? "Ask the superuser operator what to do..." : "Ask follow-up about selected workspace evidence..."}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  handleSubmit(e);
                }
              }}
              onPaste={(e) => {
                if (e.clipboardData?.items && Array.from(e.clipboardData.items).some((item) => item.type.startsWith("image/"))) {
                  e.preventDefault();
                  setCodexError("Contextual Chat accepts redacted workspace packets only.");
                }
              }}
              className="w-full bg-transparent text-xs text-foreground outline-none resize-none max-h-24 leading-relaxed placeholder:text-muted-foreground"
            />
          </div>
          <div className="flex items-center justify-between px-3 pb-2">
            <div className="flex min-w-0 items-center gap-1.5 text-[10px] text-muted-foreground">
              <ShieldCheck className="size-3 shrink-0 text-primary" />
              <span className="truncate">Codex app server</span>
            </div>
            <div className="flex items-center gap-2">
              {hasMessages && (
                <ContextMeter used={estimatedTokens} total={DEFAULT_CONTEXT_WINDOW} />
              )}
              {isLoading ? (
                <button
                  type="button"
                  onClick={() => stop()}
                  className="bg-primary text-primary-foreground rounded-full p-1 hover:bg-primary/90 transition-colors shrink-0"
                  aria-label="Stop"
                >
                  <Square className="size-3 fill-current" />
                </button>
              ) : (
                <button
                  type="submit"
                  disabled={!input.trim() || !selectedWorkspacePacket || selectedWorkspacePacketErrors.length > 0}
                  className="bg-primary text-primary-foreground rounded-full p-1 hover:bg-primary/90 transition-colors disabled:opacity-30 shrink-0"
                  aria-label="Inspect with Codex"
                >
                  <ArrowUp className="size-3" />
                </button>
              )}
            </div>
          </div>
        </form>
      </div>
    </div>
  );
}
