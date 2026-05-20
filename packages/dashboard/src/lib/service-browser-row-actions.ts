export type BrowserRowActionTitleOptions = {
  available: boolean;
  supported: boolean;
};

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
