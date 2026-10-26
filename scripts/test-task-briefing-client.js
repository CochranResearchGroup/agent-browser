import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { getTaskBriefing } from '../packages/client/src/task-briefing.js';
const calls = [];
const server = createServer((req, res) => {
  calls.push({ method: req.method, url: req.url });
  res.setHeader('content-type', 'application/json');
  res.end(JSON.stringify({ success: true, data: { schemaVersion: 'agent-browser.task-briefing.v1', taskId: 'task / exact', contextEvidence: 'retained_not_live' } }));
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
try {
  const baseUrl = `http://127.0.0.1:${server.address().port}`;
  const result = await getTaskBriefing({ baseUrl, taskId: 'task / exact' });
  assert.equal(result.taskId, 'task / exact');
  assert.deepEqual(calls, [{ method: 'GET', url: '/api/browser/tasks/task%20%2F%20exact/briefing' }]);
  await assert.rejects(getTaskBriefing({ baseUrl, taskId: 'wrong' }), /identity/);
  assert.equal(calls.length, 2, 'failed identity never retries');
  await assert.rejects(getTaskBriefing({ baseUrl, taskId: '' }), /taskId/);
  assert.equal(calls.length, 2, 'invalid identity has no request');
  console.log('Task briefing client public HTTP protocol cases passed');
} finally { await new Promise(resolve => server.close(resolve)); }
