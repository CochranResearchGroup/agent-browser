"use client";

import { atom } from "jotai";
import { useEffect } from "react";
import { useAtomCallback } from "jotai/utils";
import { useCallback } from "react";
import { CHAT_API_URL, CHAT_STATUS_API_URL, MODELS_API_URL } from "@/lib/dashboard-api";

function getChatStatusUrl(): string {
  return CHAT_STATUS_API_URL;
}

export function getChatApiUrl(): string {
  return CHAT_API_URL;
}

export function getModelsApiUrl(): string {
  return MODELS_API_URL;
}

export interface ModelInfo {
  id: string;
  name?: string;
  owned_by?: string;
  context_window?: number;
}

export const chatEnabledAtom = atom(false);
export const chatModelAtom = atom<string | undefined>(undefined);
export const availableModelsAtom = atom<ModelInfo[]>([]);

export function useChatStatusSync() {
  const fetchStatus = useAtomCallback(
    useCallback(async (_get, set) => {
      try {
        const resp = await fetch(getChatStatusUrl());
        if (resp.ok) {
          const data = await resp.json();
          set(chatEnabledAtom, !!data.enabled);
          if (data.model) set(chatModelAtom, data.model);
        }
      } catch {
        set(chatEnabledAtom, false);
      }
      try {
        const resp = await fetch(getModelsApiUrl());
        if (resp.ok) {
          const data = await resp.json();
          if (Array.isArray(data?.data)) {
            const models: ModelInfo[] = data.data.map((m: Record<string, unknown>) => ({
              id: m.id as string,
              name: (m.name as string) || undefined,
              owned_by: (m.owned_by as string) || undefined,
              context_window: typeof m.context_window === "number" ? m.context_window : undefined,
            }));
            models.sort((a, b) => a.id.localeCompare(b.id));
            set(availableModelsAtom, models);
          }
        }
      } catch {
        // models fetch failed, leave empty
      }
    }, []),
  );

  useEffect(() => {
    fetchStatus();
  }, [fetchStatus]);
}
