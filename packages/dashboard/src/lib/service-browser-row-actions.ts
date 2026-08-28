export type BrowserRowActionTitleOptions = {
  available: boolean;
  supported: boolean;
};

export type BrowserRowCloseRoute = {
  browserId: string;
  sessionName?: string;
  params: {
    browserId: string;
  };
};

/**
 * Build the broker route and daemon action parameter for one exact service browser.
 * Top-level route hints select the owning daemon; params.browserId remains the
 * service_browser_close target enforced by that daemon.
 */
export function browserRowCloseRoute(
  browserId: string,
  preferredSessionName?: string | null,
): BrowserRowCloseRoute {
  const stableBrowserId = browserId.trim();
  if (!stableBrowserId) {
    throw new Error("Browser row close route requires a stable browser ID.");
  }
  const stableSessionName = preferredSessionName?.trim()
    || (stableBrowserId.startsWith("session:")
      ? stableBrowserId.slice("session:".length).trim()
      : "");
  return {
    browserId: stableBrowserId,
    ...(stableSessionName ? { sessionName: stableSessionName } : {}),
    params: { browserId: stableBrowserId },
  };
}

export function browserRowCloseTitle({
  available,
  supported,
}: BrowserRowActionTitleOptions): string {
  if (available) return "Queue polite close for this service browser.";
  if (!supported) return "This service does not advertise row-scoped browser close support.";
  return "Only the active service browser can be closed from this row.";
}

export function browserRowRepairTitle({
  available,
  supported,
}: BrowserRowActionTitleOptions): string {
  if (available) return "Mark this degraded or faulted browser retryable.";
  if (!supported) return "This service does not advertise row-scoped browser repair support.";
  return "Repair is available for degraded or faulted browser records.";
}
