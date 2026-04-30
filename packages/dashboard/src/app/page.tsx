"use client";

import { useState } from "react";
import { useAtomValue, useSetAtom } from "jotai/react";
import { activePortAtom, sessionsAtom, newSessionDialogAtom } from "@/store/sessions";
import { useSessionsSync } from "@/store/sessions";
import { useStreamSync, hasConsoleErrorsAtom, consoleLogsAtom } from "@/store/stream";
import { useActivitySync } from "@/store/activity";
import { activeExtensionsAtom } from "@/store/sessions";
import { useChatStatusSync } from "@/store/chat";
import { useMediaQuery } from "@/hooks/use-media-query";
import { Viewport } from "@/components/viewport";
import { ActivityFeed } from "@/components/activity-feed";
import { ChatPanel } from "@/components/chat-panel";
import { ConsolePanel } from "@/components/console-panel";
import { StoragePanel } from "@/components/storage-panel";
import { ExtensionsPanel } from "@/components/extensions-panel";
import { NetworkPanel } from "@/components/network-panel";
import { SessionTree } from "@/components/session-tree";
import { AppShell, type DashboardSection } from "@/components/app-shell";
import { ServicePanel } from "@/components/service-panel";
import {
  ResizablePanelGroup,
  ResizablePanel,
  ResizableHandle,
} from "@/components/ui/resizable";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { Plus } from "lucide-react";

export default function DashboardPage() {
  const [activeSection, setActiveSection] = useState<DashboardSection>("overview");
  const activePort = useAtomValue(activePortAtom);
  useStreamSync(activePort);
  useSessionsSync();
  useActivitySync();
  useChatStatusSync();

  const sessions = useAtomValue(sessionsAtom);
  const hasSessions = sessions.length > 0;
  const setNewSessionDialog = useSetAtom(newSessionDialogAtom);
  const isDesktop = useMediaQuery("(min-width: 768px)");
  const hasConsoleErrors = useAtomValue(hasConsoleErrorsAtom);
  const activeExtensions = useAtomValue(activeExtensionsAtom);
  const primaryPanel = activeSection === "service"
    ? <ServicePanel />
    : activeSection === "activity"
      ? <ActivityFeed />
      : <Viewport />;

  const sidePanel = (
    <Tabs defaultValue="chat" className="flex h-full flex-col">
      <div className="shrink-0 px-2 pt-1">
        <TabsList variant="line" className="h-7 w-full">
          <TabsTrigger value="chat" className="text-[11px]">Chat</TabsTrigger>
          <TabsTrigger value="activity" className="text-[11px]">Activity</TabsTrigger>
          <TabsTrigger value="service" className="text-[11px]">Service</TabsTrigger>
          <TabsTrigger value="console" className="text-[11px]">
            Console
            {hasConsoleErrors && (
              <span className="ml-1 inline-flex size-1.5 rounded-full bg-destructive" />
            )}
          </TabsTrigger>
          <TabsTrigger value="network" className="text-[11px]">Network</TabsTrigger>
          <TabsTrigger value="storage" className="text-[11px]">Storage</TabsTrigger>
          <TabsTrigger value="extensions" className="text-[11px]">
            Extensions
            {activeExtensions.length > 0 && (
              <span className="ml-1 text-[9px] tabular-nums text-muted-foreground">{activeExtensions.length}</span>
            )}
          </TabsTrigger>
        </TabsList>
      </div>
      <TabsContent value="activity" className="min-h-0 flex-1 overflow-hidden">
        <ActivityFeed />
      </TabsContent>
      <TabsContent value="service" className="min-h-0 flex-1 overflow-hidden">
        <ServicePanel />
      </TabsContent>
      <TabsContent value="console" className="min-h-0 flex-1 overflow-hidden">
        <ConsolePanel />
      </TabsContent>
      <TabsContent value="network" className="min-h-0 flex-1 overflow-hidden">
        <NetworkPanel />
      </TabsContent>
      <TabsContent value="storage" className="min-h-0 flex-1 overflow-hidden">
        <StoragePanel />
      </TabsContent>
      <TabsContent value="extensions" className="min-h-0 flex-1 overflow-hidden">
        <ExtensionsPanel />
      </TabsContent>
      <TabsContent value="chat" className="min-h-0 flex-1 overflow-hidden">
        <ChatPanel />
      </TabsContent>
    </Tabs>
  );

  if (isDesktop) {
    if (!hasSessions) {
      return (
        <AppShell activeSection={activeSection} onSectionChange={setActiveSection}>
          <ResizablePanelGroup
            orientation="horizontal"
            className="dashboard-panel-grid"
          >
            <ResizablePanel id="sessions" defaultSize="15%" minSize="10%" maxSize="30%">
              <div className="dashboard-pane dashboard-pane-left">
                <SessionTree />
              </div>
            </ResizablePanel>
            <ResizableHandle />
            <ResizablePanel id="empty" defaultSize="85%">
              <div className="dashboard-empty-state">
                <div className="dashboard-empty-card">
                  <div className="dashboard-empty-orb">
                    <Plus className="size-6" />
                  </div>
                  <div className="space-y-2">
                    <p className="text-xl font-black tracking-[-0.04em] text-foreground">
                      No active sessions
                    </p>
                    <p className="max-w-sm text-sm leading-6 text-muted-foreground">
                      Start a managed browser workspace to inspect pages, stream a headed session, and prepare the service control plane.
                    </p>
                  </div>
                  <Button
                    size="lg"
                    className="dashboard-primary-action"
                    onClick={() => setNewSessionDialog(true)}
                  >
                    <Plus className="size-4" />
                    New session
                  </Button>
                </div>
              </div>
            </ResizablePanel>
          </ResizablePanelGroup>
        </AppShell>
      );
    }

    return (
      <AppShell activeSection={activeSection} onSectionChange={setActiveSection}>
        <ResizablePanelGroup
          orientation="horizontal"
          className="dashboard-panel-grid"
        >
          <ResizablePanel id="sessions" defaultSize="15%" minSize="10%" maxSize="30%">
            <div className="dashboard-pane dashboard-pane-left">
              <SessionTree />
            </div>
          </ResizablePanel>
          <ResizableHandle />
          <ResizablePanel id="viewport" defaultSize="55%" minSize="30%">
            <div className="dashboard-pane dashboard-pane-viewport">
              {primaryPanel}
            </div>
          </ResizablePanel>
          <ResizableHandle />
          <ResizablePanel id="activity" defaultSize="30%" minSize="15%" maxSize="50%">
            <div className="dashboard-pane dashboard-pane-right">
              {sidePanel}
            </div>
          </ResizablePanel>
        </ResizablePanelGroup>
      </AppShell>
    );
  }

  return (
    <AppShell activeSection={activeSection} onSectionChange={setActiveSection}>
      <Tabs
        value={activeSection === "service" ? "service" : activeSection === "activity" ? "activity" : "viewport"}
        onValueChange={(value) => {
          if (value === "service" || value === "activity") {
            setActiveSection(value);
          } else {
            setActiveSection("overview");
          }
        }}
        className="min-h-0 flex-1"
      >
        <div className="shrink-0 px-3 pt-3">
          <TabsList className="w-full rounded-2xl bg-white/60 p-1 shadow-sm ring-1 ring-foreground/10 backdrop-blur-xl dark:bg-white/5">
            <TabsTrigger value="sessions">Sessions</TabsTrigger>
            <TabsTrigger value="viewport">Viewport</TabsTrigger>
            <TabsTrigger value="activity">Activity</TabsTrigger>
            <TabsTrigger value="service">Service</TabsTrigger>
          </TabsList>
        </div>
        <TabsContent value="sessions" className="min-h-0 overflow-hidden p-3">
          <SessionTree />
        </TabsContent>
        <TabsContent value="viewport" className="min-h-0 overflow-hidden p-3">
          <Viewport />
        </TabsContent>
        <TabsContent value="activity" className="min-h-0 overflow-hidden p-3">
          {sidePanel}
        </TabsContent>
        <TabsContent value="service" className="min-h-0 overflow-hidden p-3">
          <div className="dashboard-pane">
            <ServicePanel />
          </div>
        </TabsContent>
      </Tabs>
    </AppShell>
  );
}
