"use client";

import type { ReactNode } from "react";
import { useAtomValue, useSetAtom } from "jotai/react";
import {
  Activity,
  Bot,
  ChevronDown,
  CircleDot,
  Compass,
  GalleryVerticalEnd,
  LayoutDashboard,
  MonitorDot,
  Plus,
  Radio,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { activeSessionInfoAtom, newSessionDialogAtom, sessionsAtom } from "@/store/sessions";
import {
  activeUrlAtom,
  browserConnectedAtom,
  recordingAtom,
  screencastingAtom,
  streamConnectedAtom,
} from "@/store/stream";
import { ThemeToggle } from "@/components/theme-toggle";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";

export type DashboardSection = "overview" | "browsers" | "service" | "activity";

const NAV_ITEMS: { id: DashboardSection; label: string; icon: typeof LayoutDashboard }[] = [
  { id: "overview", label: "Overview", icon: LayoutDashboard },
  { id: "browsers", label: "Browsers", icon: MonitorDot },
  { id: "service", label: "Service", icon: ShieldCheck },
  { id: "activity", label: "Activity", icon: Activity },
];

function StatusChip({
  label,
  value,
  tone = "neutral",
}: {
  label: string;
  value: string;
  tone?: "neutral" | "good" | "warn";
}) {
  return (
    <span
      className={cn(
        "dashboard-chip",
        tone === "good" && "dashboard-chip-good",
        tone === "warn" && "dashboard-chip-warn",
      )}
    >
      <CircleDot className="size-3" />
      <span className="hidden text-muted-foreground lg:inline">{label}</span>
      <span className="font-semibold">{value}</span>
    </span>
  );
}

function currentHost(url: string): string {
  try {
    return new URL(url).host || "blank";
  } catch {
    return url ? "custom" : "blank";
  }
}

export function AppShell({
  activeSection = "overview",
  onSectionChange,
  children,
}: {
  activeSection?: DashboardSection;
  onSectionChange?: (section: DashboardSection) => void;
  children: ReactNode;
}) {
  const sessions = useAtomValue(sessionsAtom);
  const activeSession = useAtomValue(activeSessionInfoAtom);
  const streamConnected = useAtomValue(streamConnectedAtom);
  const browserConnected = useAtomValue(browserConnectedAtom);
  const screencasting = useAtomValue(screencastingAtom);
  const recording = useAtomValue(recordingAtom);
  const activeUrl = useAtomValue(activeUrlAtom);
  const setNewSessionDialog = useSetAtom(newSessionDialogAtom);
  const activeSessionName = activeSession?.session ?? "No session";
  const sessionInitial = activeSessionName.slice(0, 1).toUpperCase();

  return (
    <div className="dashboard-root">
      <div className="dashboard-aurora dashboard-aurora-one" />
      <div className="dashboard-aurora dashboard-aurora-two" />
      <header className="dashboard-topbar">
        <div className="flex min-w-0 items-center gap-3">
          <div className="dashboard-mark">
            <Bot className="size-5" />
          </div>
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <p className="truncate text-sm font-black tracking-[-0.03em] text-foreground">
                Agent Browser
              </p>
              <span className="rounded-full border border-foreground/10 bg-foreground/5 px-2 py-0.5 text-[10px] font-bold uppercase tracking-[0.22em] text-muted-foreground">
                Service Lab
              </span>
            </div>
            <p className="truncate text-[11px] text-muted-foreground">
              Persistent browser operations for agents and humans
            </p>
          </div>
        </div>

        <nav className="dashboard-nav" aria-label="Dashboard navigation">
          {NAV_ITEMS.map((item) => (
            <button
              key={item.label}
              type="button"
              onClick={() => onSectionChange?.(item.id)}
              className={cn(
                "dashboard-nav-item",
                activeSection === item.id && "dashboard-nav-item-active",
              )}
            >
              <item.icon className="size-3.5" />
              <span>{item.label}</span>
            </button>
          ))}
        </nav>

        <div className="ml-auto flex min-w-0 items-center gap-2">
          <div className="hidden items-center gap-2 xl:flex">
            <StatusChip
              label="Stream"
              value={streamConnected ? "live" : "waiting"}
              tone={streamConnected ? "good" : "warn"}
            />
            <StatusChip
              label="Browser"
              value={browserConnected ? "ready" : "idle"}
              tone={browserConnected ? "good" : "neutral"}
            />
            <StatusChip
              label="Cast"
              value={recording ? "recording" : screencasting ? "viewing" : "standby"}
              tone={recording || screencasting ? "good" : "neutral"}
            />
          </div>
          <Button
            size="sm"
            className="dashboard-primary-action"
            onClick={() => setNewSessionDialog(true)}
          >
            <Plus className="size-3.5" />
            <span className="hidden sm:inline">New session</span>
          </Button>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button type="button" className="dashboard-avatar-chip">
                <span className="dashboard-avatar">{sessionInitial || "A"}</span>
                <span className="hidden min-w-0 text-left md:block">
                  <span className="block max-w-32 truncate text-xs font-bold">
                    {activeSessionName}
                  </span>
                  <span className="block max-w-32 truncate text-[10px] text-muted-foreground">
                    {currentHost(activeUrl)}
                  </span>
                </span>
                <ChevronDown className="size-3 text-muted-foreground" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-64">
              <DropdownMenuLabel>
                <span className="block text-xs text-muted-foreground">Operator</span>
                <span className="block truncate text-sm text-foreground">{activeSessionName}</span>
              </DropdownMenuLabel>
              <DropdownMenuSeparator />
              <DropdownMenuItem>
                <GalleryVerticalEnd className="size-4" />
                {sessions.length} tracked {sessions.length === 1 ? "session" : "sessions"}
              </DropdownMenuItem>
              <DropdownMenuItem>
                <Compass className="size-4" />
                {currentHost(activeUrl)}
              </DropdownMenuItem>
              <DropdownMenuItem>
                <Radio className="size-4" />
                Stream {streamConnected ? "connected" : "waiting"}
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem onSelect={(event) => event.preventDefault()}>
                <Sparkles className="size-4" />
                Theme
                <span className="ml-auto">
                  <ThemeToggle />
                </span>
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </header>

      <main className="dashboard-main">{children}</main>
    </div>
  );
}
