// @ts-check
/** Read one exact retained task. No browser actions, fallback or retries.
 * @param {{baseUrl: string, taskId: string, headers?: Record<string,string>, signal?: AbortSignal, fetch?: typeof globalThis.fetch}} options
 * @returns {Promise<import('./task-briefing.js').TaskBriefing>}
 */
export async function getTaskBriefing({ baseUrl, taskId, headers, signal, fetch = globalThis.fetch }) {
  if (typeof taskId !== 'string' || taskId.trim() === '') throw new TypeError('taskId is required');
  const url = new URL(`/api/browser/tasks/${encodeURIComponent(taskId)}/briefing`, baseUrl);
  const response = await fetch(url, { method: 'GET', headers, signal, redirect: 'error' });
  if (!response.ok) throw new Error(`task briefing read failed: ${response.status}`);
  const payload = await response.json();
  if (payload?.success !== true || payload.data?.schemaVersion !== 'agent-browser.task-briefing.v1' || payload.data.taskId !== taskId) {
    throw new Error('task briefing identity or response mismatch');
  }
  return payload.data;
}

/**
 * Read an original operation without executing or retrying it.
 * @param {{baseUrl: string, taskId: string, requestId: string, headers?: Record<string,string>, signal?: AbortSignal, fetch?: typeof globalThis.fetch}} options
 * @returns {Promise<import('./task-briefing.js').TaskOperationResponse>}
 */
export async function getTaskOperation({ baseUrl, taskId, requestId, headers, signal, fetch = globalThis.fetch }) {
  if (typeof taskId !== 'string' || taskId.trim() === '' ||
      typeof requestId !== 'string' || requestId.trim() === '') throw new TypeError('taskId and requestId are required');
  const url = new URL(`/api/browser/tasks/${encodeURIComponent(taskId)}/operations/${encodeURIComponent(requestId)}`, baseUrl);
  const response = await fetch(url, { method: 'GET', headers, signal, redirect: 'error' });
  if (!response.ok) throw new Error(`task operation read failed: ${response.status}`);
  const payload = await response.json();
  if (payload?.success !== true || payload.data?.schemaVersion !== 'agent-browser.task-operation.v1' ||
      payload.data.operation?.taskId !== taskId || payload.data.operation.requestId !== requestId) {
    throw new Error('task operation identity or response mismatch');
  }
  return payload.data;
}
