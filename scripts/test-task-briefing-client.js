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

// Owner refusals survive an HTTP-success transport, without a second request.
const failure = {code:'browser_task_operation_outcome_uncertain', occurrenceId:'failure-exact-760', effectState:'effect_uncertain', retryDisposition:'inspect_before_retry', recommendedAction:'Inspect the original operation', reuseAllowed:false};
let refusedCalls = 0;
await assert.rejects(getTaskBriefing({baseUrl:'https://example.invalid', taskId:'task-exact', fetch:async()=>{refusedCalls++;return new Response(JSON.stringify({success:true,data:{ok:false,failure}}));}}), error=>error.code===failure.code && error.failure.occurrenceId===failure.occurrenceId && error.failure.effectState==='effect_uncertain');
assert.equal(refusedCalls,1);
