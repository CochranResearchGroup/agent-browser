const focusRequests = new Map<string, Promise<void>>();

/**
 * Preserve browser focus request order across viewport refreshes and mounts.
 * Completing a transport request does not prove native focus after a failure.
 */
export async function queueWorkspaceFocusRequest(
  browserKey: string,
  request: () => Promise<void>,
): Promise<void> {
  const previous = focusRequests.get(browserKey) ?? Promise.resolve();
  const current = previous.catch(() => {}).then(request);
  focusRequests.set(browserKey, current);
  try {
    await current;
  } finally {
    if (focusRequests.get(browserKey) === current) focusRequests.delete(browserKey);
  }
}
