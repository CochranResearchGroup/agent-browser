const focusRequests = new Map<string, Promise<void>>();

/**
 * Preserve focus request order within the declared display or browser scope
 * across viewport refreshes and mounts.
 * Completing a transport request does not prove native focus after a failure.
 */
export async function queueWorkspaceFocusRequest(
  focusScopeKey: string,
  request: () => Promise<void>,
): Promise<void> {
  const previous = focusRequests.get(focusScopeKey) ?? Promise.resolve();
  const current = previous.catch(() => {}).then(request);
  focusRequests.set(focusScopeKey, current);
  try {
    await current;
  } finally {
    if (focusRequests.get(focusScopeKey) === current) focusRequests.delete(focusScopeKey);
  }
}
