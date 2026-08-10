"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { DASHBOARD_WORKSPACE_SELECTION_EVENT } from "@/lib/workspace-url-selection";
import type { WorkspaceViewPreferenceScope } from "@/lib/workspace-view-projection";

const STORAGE_KEY = "agent-browser.workspace-view-stream-preferences.v1";

type StoredWorkspaceViewPreferences = Record<string, string>;

export type WorkspaceViewPreferenceSnapshot = {
  revision: number;
  selectedProvider: string | null;
  byBrowserId: WorkspaceViewPreferenceScope["byBrowserId"];
};

export type WorkspaceViewPreferenceController = {
  snapshot: WorkspaceViewPreferenceSnapshot;
  write: (browserId: string, streamKey: string) => void;
};

function readStoredPreferences(): StoredWorkspaceViewPreferences {
  if (typeof window === "undefined") return {};
  try {
    const parsed = JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? "{}");
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
    return Object.fromEntries(Object.entries(parsed).filter(
      (entry): entry is [string, string] => typeof entry[1] === "string" && entry[1].trim().length > 0,
    ));
  } catch {
    return {};
  }
}

function readSelectedProvider(): string | null {
  if (typeof window === "undefined") return null;
  return new URLSearchParams(window.location.search).get("view-provider")?.trim().toLowerCase() || null;
}

/** Owns the single dashboard-local stream preference snapshot. */
export function useWorkspaceViewPreferences(): WorkspaceViewPreferenceController {
  const [stored, setStored] = useState<StoredWorkspaceViewPreferences>(readStoredPreferences);
  const [selectedProvider, setSelectedProvider] = useState(readSelectedProvider);
  const [revision, setRevision] = useState(0);

  useEffect(() => {
    const refresh = () => {
      setStored(readStoredPreferences());
      setSelectedProvider(readSelectedProvider());
      setRevision((current) => current + 1);
    };
    const onStorage = (event: StorageEvent) => {
      if (!event.key || event.key === STORAGE_KEY) refresh();
    };
    window.addEventListener("storage", onStorage);
    window.addEventListener("popstate", refresh);
    window.addEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, refresh);
    return () => {
      window.removeEventListener("storage", onStorage);
      window.removeEventListener("popstate", refresh);
      window.removeEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, refresh);
    };
  }, []);

  const write = useCallback((browserId: string, streamKey: string) => {
    setStored((current) => {
      const next = { ...current, [browserId]: streamKey };
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
      return next;
    });
    setRevision((current) => current + 1);
  }, []);

  const byBrowserId = useMemo(() => Object.fromEntries(
    Object.entries(stored).map(([browserId, streamKey]) => [browserId, { streamKey }]),
  ), [stored]);

  return {
    snapshot: { revision, selectedProvider, byBrowserId },
    write,
  };
}
