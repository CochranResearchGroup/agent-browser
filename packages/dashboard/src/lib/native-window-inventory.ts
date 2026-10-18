export type NativeBrowserWindow = {
  key: string;
  desktopId: string;
  slot: number;
  generation: number;
  displayName: string;
  application: string | null;
  windowId: number;
  pid: number | null;
  title: string;
  class: string;
};

/** Native windows describe viewing availability, never managed tab custody. */
export function nativeBrowserWindows(value: unknown): { windows: NativeBrowserWindow[]; ageMs: number; observedAtMs: number; unverified: number } {
  const source = value as Record<string, unknown> | null;
  if (!source || source.schemaVersion !== 1 || source.source !== "remote-view-native-windows"
    || !Array.isArray(source.desktops) || typeof source.observationAgeMs !== "number"
    || !Number.isFinite(source.observationAgeMs) || source.observationAgeMs < 0
    || typeof source.observedAtMs !== "number" || !Number.isSafeInteger(source.observedAtMs) || source.observedAtMs < 0
    || typeof source.unverifiedDesktopCount !== "number" || !Number.isSafeInteger(source.unverifiedDesktopCount)
    || source.unverifiedDesktopCount < 0) throw new Error("native_inventory_invalid");
  const windows: NativeBrowserWindow[] = [];
  const seen = new Set<string>();
  for (const desktop of source.desktops) {
    if (!desktop || typeof desktop.desktopId !== "string" || !/^[a-f0-9-]{36}$/i.test(desktop.desktopId)
      || !Number.isSafeInteger(desktop.slot) || desktop.slot <= 0
      || !Number.isSafeInteger(desktop.generation) || desktop.generation <= 0
      || typeof desktop.displayName !== "string" || !Array.isArray(desktop.windows)) throw new Error("native_inventory_invalid");
    for (const window of desktop.windows) {
      if (!window || !Number.isSafeInteger(window.id) || window.id <= 0 || typeof window.title !== "string"
        || typeof window.class !== "string" || (window.pid !== null && (!Number.isSafeInteger(window.pid) || window.pid <= 0))) throw new Error("native_inventory_invalid");
      const key = `${desktop.desktopId}:${desktop.generation}:${window.id}`;
      if (seen.has(key)) throw new Error("native_inventory_duplicate");
      seen.add(key);
      windows.push({ key, desktopId: desktop.desktopId, slot: desktop.slot, generation: desktop.generation,
        displayName: desktop.displayName, application: typeof desktop.application === "string" ? desktop.application : null,
        windowId: window.id, pid: window.pid, title: window.title.trim() || window.class || "Browser window", class: window.class });
    }
  }
  return { windows, ageMs: source.observationAgeMs, observedAtMs: source.observedAtMs, unverified: source.unverifiedDesktopCount };
}
