"use client";

import { useState, useEffect } from "react";
import { takeScreenshot, takeSnapshot, getEnvStatus } from "./actions/browse";
import type {
  ScreenshotResult,
  SnapshotResult,
  Mode,
  EnvStatus,
} from "./actions/browse";

type Action = "screenshot" | "snapshot";

function formatError(raw: string): string {
  let cleaned = raw.replace(/<[^>]*>/g, " ").replace(/\s+/g, " ").trim();
  const match = cleaned.match(
    /(?:error|Error)[:\s]*(.{1,200})/,
  );
  if (match) cleaned = match[1].trim();
  if (cleaned.length > 300) cleaned = cleaned.slice(0, 300) + "...";
  return cleaned || raw.slice(0, 300);
}

function SegmentedControl<T extends string>({
  value,
  onChange,
  options,
}: {
  value: T;
  onChange: (v: T) => void;
  options: { value: T; label: string }[];
}) {
  return (
    <div className="inline-flex rounded-lg bg-surface border border-border p-0.5">
      {options.map((opt) => (
        <button
          key={opt.value}
          type="button"
          onClick={() => onChange(opt.value)}
          className={`
            px-3 py-1.5 text-[13px] font-medium rounded-md transition-all cursor-pointer
            ${
              value === opt.value
                ? "bg-background text-foreground shadow-sm"
                : "text-muted hover:text-foreground"
            }
          `}
        >
          {opt.label}
        </button>
      ))}
    </div>
  );
}

function EnvBadge({
  label,
  value,
  status,
}: {
  label: string;
  value?: string;
  status: "ok" | "warn" | "missing";
}) {
  const colors = {
    ok: "bg-emerald-50 text-emerald-700 border-emerald-200 dark:bg-emerald-950 dark:text-emerald-400 dark:border-emerald-900",
    warn: "bg-amber-50 text-amber-700 border-amber-200 dark:bg-amber-950 dark:text-amber-400 dark:border-amber-900",
    missing:
      "bg-red-50 text-red-700 border-red-200 dark:bg-red-950 dark:text-red-400 dark:border-red-900",
  };
  const icons = { ok: "\u2713", warn: "\u26A0", missing: "\u2717" };

  return (
    <div
      className={`inline-flex items-center gap-1.5 text-[11px] font-medium px-2 py-0.5 rounded-md border ${colors[status]}`}
    >
      <span>{icons[status]}</span>
      <span className="font-mono">{label}</span>
      {value && (
        <span className="opacity-60 max-w-[120px] truncate">{value}</span>
      )}
    </div>
  );
}

function ErrorDisplay({ error }: { error: string }) {
  const isHtml = /<[a-z][\s\S]*>/i.test(error);
  const message = isHtml ? formatError(error) : error;
  const showRaw = isHtml && error.length > 100;

  return (
    <div className="rounded-lg border border-red-200 bg-red-50 dark:border-red-900 dark:bg-red-950/50 overflow-hidden">
      <div className="flex items-start gap-2.5 p-4">
        <span className="text-red-500 shrink-0 mt-0.5">{"\u2717"}</span>
        <div className="min-w-0">
          <p className="text-sm font-medium text-red-700 dark:text-red-400 mb-1">
            Request failed
          </p>
          <p className="text-[13px] text-red-600 dark:text-red-400/80 leading-relaxed">
            {message}
          </p>
        </div>
      </div>
      {showRaw && (
        <details className="border-t border-red-200 dark:border-red-900">
          <summary className="px-4 py-2 text-[11px] font-medium text-red-500 cursor-pointer hover:bg-red-100 dark:hover:bg-red-950 transition-colors">
            Show raw response
          </summary>
          <pre className="px-4 py-3 text-[11px] leading-relaxed text-red-600/70 dark:text-red-400/50 font-mono overflow-auto max-h-[200px] bg-red-100/50 dark:bg-red-950/30">
            {error}
          </pre>
        </details>
      )}
    </div>
  );
}

function ModeCard({
  selected,
  onSelect,
  title,
  description,
  diagram,
  badges,
}: {
  selected: boolean;
  onSelect: () => void;
  title: string;
  description: string;
  diagram: string;
  badges?: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className={`
        flex-1 text-left rounded-xl border p-4 transition-all cursor-pointer
        ${
          selected
            ? "border-foreground bg-background ring-1 ring-foreground/10"
            : "border-border bg-background hover:border-gray-400"
        }
      `}
    >
      <div className="flex items-center gap-2 mb-2">
        <div
          className={`
            size-4 rounded-full border-2 flex items-center justify-center transition-colors
            ${selected ? "border-foreground" : "border-gray-300"}
          `}
        >
          {selected && <div className="size-2 rounded-full bg-foreground" />}
        </div>
        <span className="text-sm font-semibold">{title}</span>
      </div>
      <p className="text-[13px] text-muted leading-relaxed mb-3 pl-6">
        {description}
      </p>
      {badges && (
        <div className="flex flex-wrap gap-1.5 mb-3 pl-6">{badges}</div>
      )}
      <pre className="bg-surface text-[11px] leading-relaxed text-muted rounded-lg p-3 overflow-auto font-mono border border-border">
        {diagram}
      </pre>
    </button>
  );
}

export default function Home() {
  const [url, setUrl] = useState("https://example.com");
  const [loading, setLoading] = useState(false);
  const [action, setAction] = useState<Action>("screenshot");
  const [mode, setMode] = useState<Mode>("serverless");
  const [screenshotResult, setScreenshotResult] =
    useState<ScreenshotResult | null>(null);
  const [snapshotResult, setSnapshotResult] =
    useState<SnapshotResult | null>(null);
  const [envStatus, setEnvStatus] = useState<EnvStatus | null>(null);

  useEffect(() => {
    getEnvStatus().then(setEnvStatus);
  }, []);

  function clearResults() {
    setScreenshotResult(null);
    setSnapshotResult(null);
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);
    setScreenshotResult(null);
    setSnapshotResult(null);

    if (action === "screenshot") {
      const result = await takeScreenshot(url, mode);
      setScreenshotResult(result);
    } else {
      const result = await takeSnapshot(url, mode);
      setSnapshotResult(result);
    }
    setLoading(false);
  }

  const hasResult = screenshotResult || snapshotResult;

  const envWarning =
    envStatus &&
    mode === "serverless" &&
    !envStatus.serverless.isVercel &&
    !envStatus.serverless.hasChromiumPath
      ? "Running locally without CHROMIUM_PATH. The app will try to use your system Chrome. Set CHROMIUM_PATH if Chrome is not in the default location."
      : null;

  return (
    <div className="min-h-screen flex flex-col">
      <header className="border-b border-border">
        <div className="max-w-3xl mx-auto px-6 h-14 flex items-center gap-3">
          <span className="text-sm font-semibold tracking-tight">
            agent-browser
          </span>
          <span className="text-muted text-sm">/</span>
          <span className="text-sm text-muted">Next.js Example</span>
        </div>
      </header>

      <main className="flex-1 max-w-3xl mx-auto w-full px-6 py-12">
        <div className="mb-10">
          <h1 className="text-2xl font-semibold tracking-tight mb-2">
            Browser Automation
          </h1>
          <p className="text-muted text-[15px] leading-relaxed max-w-lg">
            Take screenshots and accessibility snapshots of any URL using
            agent-browser, powered by Vercel serverless functions.
          </p>
        </div>

        <form onSubmit={handleSubmit} className="space-y-6">
          <div className="flex gap-2">
            <div className="relative flex-1">
              <input
                type="url"
                value={url}
                onChange={(e) => { setUrl(e.target.value); clearResults(); }}
                placeholder="https://example.com"
                required
                className="w-full h-10 px-3 text-sm bg-background border border-border rounded-lg outline-none transition-colors focus:border-foreground focus:ring-1 focus:ring-foreground/10 placeholder:text-gray-400"
              />
            </div>
            <button
              type="submit"
              disabled={loading}
              className={`
                h-10 px-5 text-sm font-medium rounded-lg transition-all inline-flex items-center gap-2 cursor-pointer
                ${
                  loading
                    ? "bg-gray-400 text-white cursor-wait"
                    : "bg-foreground text-background hover:opacity-90"
                }
              `}
            >
              {loading && (
                <svg
                  className="animate-spin size-3.5"
                  viewBox="0 0 24 24"
                  fill="none"
                >
                  <circle
                    cx="12"
                    cy="12"
                    r="10"
                    stroke="currentColor"
                    strokeWidth="3"
                    className="opacity-25"
                  />
                  <path
                    d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
                    fill="currentColor"
                    className="opacity-75"
                  />
                </svg>
              )}
              {loading ? "Running..." : "Run"}
            </button>
          </div>

          <div className="flex items-center gap-4">
            <SegmentedControl<Action>
              value={action}
              onChange={(v) => { setAction(v); clearResults(); }}
              options={[
                { value: "screenshot", label: "Screenshot" },
                { value: "snapshot", label: "Snapshot" },
              ]}
            />
            <span className="text-[13px] text-muted">
              {action === "screenshot"
                ? "Captures a full-page PNG image"
                : "Returns the accessibility tree"}
            </span>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <ModeCard
              selected={mode === "serverless"}
              onSelect={() => { setMode("serverless"); clearResults(); }}
              title="Serverless Function"
              description="Runs @sparticuz/chromium + puppeteer-core directly in a Vercel function. No external dependencies."
              diagram={[
                "Vercel Function",
                "+----------------------------+",
                "| Next.js Server Action      |",
                "| puppeteer-core             |",
                "| @sparticuz/chromium        |",
                "+----------------------------+",
              ].join("\n")}
              badges={
                envStatus && (
                  <EnvBadge
                    label="@sparticuz/chromium"
                    status={
                      envStatus.serverless.isVercel
                        ? "ok"
                        : envStatus.serverless.hasChromiumPath
                          ? "ok"
                          : "warn"
                    }
                    value={
                      envStatus.serverless.isVercel
                        ? "auto"
                        : envStatus.serverless.hasChromiumPath
                          ? "CHROMIUM_PATH"
                          : "system Chrome"
                    }
                  />
                )
              }
            />
            <ModeCard
              selected={mode === "sandbox"}
              onSelect={() => { setMode("sandbox"); clearResults(); }}
              title="Vercel Sandbox"
              description="Ephemeral microVM with agent-browser + Chrome. Isolated environment, no cold-start binary limits."
              diagram={[
                "Vercel",
                "+---------+    +------------------+",
                "| Action  | -> | Sandbox (microVM)|",
                "+---------+    | agent-browser    |",
                "               | Chrome           |",
                "               +------------------+",
              ].join("\n")}
              badges={
                envStatus && (
                  <EnvBadge
                    label="AGENT_BROWSER_SNAPSHOT_ID"
                    status={
                      envStatus.sandbox.hasSnapshot ? "ok" : "warn"
                    }
                  />
                )
              }
            />
          </div>

          {envWarning && (
            <div className="flex gap-3 rounded-lg border border-amber-200 bg-amber-50 p-3 text-[13px] leading-relaxed text-amber-800 dark:border-amber-900 dark:bg-amber-950 dark:text-amber-300">
              <span className="shrink-0 mt-0.5">{"\u26A0"}</span>
              <div>
                <p className="font-medium mb-0.5">Local development note</p>
                <p className="text-amber-700 dark:text-amber-400">
                  {envWarning}
                </p>
                <code className="mt-2 block text-[11px] font-mono bg-amber-100 dark:bg-amber-900/50 rounded px-2 py-1.5 text-amber-900 dark:text-amber-200">
                  CHROMIUM_PATH=/path/to/chromium
                </code>
              </div>
            </div>
          )}
        </form>

        {hasResult && (
          <div className="mt-10 pt-8 border-t border-border">
            {screenshotResult &&
              (screenshotResult.ok ? (
                <div>
                  <div className="flex items-center justify-between mb-4">
                    <h2 className="text-sm font-semibold">
                      {screenshotResult.title}
                    </h2>
                    <span className="text-xs text-muted font-mono bg-surface px-2 py-1 rounded-md border border-border">
                      screenshot
                    </span>
                  </div>
                  <div className="rounded-xl border border-border overflow-hidden">
                    <img
                      src={`data:image/png;base64,${screenshotResult.screenshot}`}
                      alt={screenshotResult.title}
                      className="w-full block"
                    />
                  </div>
                </div>
              ) : (
                <ErrorDisplay error={screenshotResult.error ?? "Unknown error"} />
              ))}

            {snapshotResult &&
              (snapshotResult.ok ? (
                <div>
                  <div className="flex items-center justify-between mb-4">
                    <h2 className="text-sm font-semibold">
                      {snapshotResult.title}
                    </h2>
                    <span className="text-xs text-muted font-mono bg-surface px-2 py-1 rounded-md border border-border">
                      snapshot
                    </span>
                  </div>
                  <pre className="bg-surface rounded-xl border border-border p-5 overflow-auto text-[13px] leading-relaxed font-mono max-h-[500px]">
                    {snapshotResult.snapshot}
                  </pre>
                </div>
              ) : (
                <ErrorDisplay error={snapshotResult.error ?? "Unknown error"} />
              ))}
          </div>
        )}
      </main>

    </div>
  );
}
