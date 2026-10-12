#!/usr/bin/env node
/** Default MCP advertisement is qualified through the installed stdio seam. */
import assert from 'node:assert/strict';
import { createMcpStdioClient, createSmokeContext } from './smoke-utils.js';
assert(process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD, 'Pin the candidate executable');
const context = createSmokeContext({ prefix: 'ab-catalog-', sessionPrefix: 'catalog' });
const client = createMcpStdioClient({ context, args: ['mcp', 'serve'], onFatal: () => {} });
try {
  await client.send('initialize', { protocolVersion: '2025-06-18', capabilities: {},
    clientInfo: { name: 'workflow-catalog-fixture', version: '0' } });
  client.notify('notifications/initialized');
  const { tools } = await client.send('tools/list');
  const bytes = Buffer.byteLength(JSON.stringify(tools));
  console.log(JSON.stringify({ toolCount: tools.length, schemaBytes: bytes }));
  assert.deepEqual(tools.map(tool => tool.name).sort(), [
    'browser_open', 'browser_navigate', 'browser_snapshot', 'browser_click',
    'browser_fill', 'browser_screenshot', 'browser_tab', 'browser_handoff',
    'browser_continue', 'browser_close', 'browser_command', 'browser_discover',
  ].sort(), 'the default catalog contains only the complete workflow surface');
  assert(bytes <= 15000, 'ordinary selection must not load the service architecture');
  for (const tool of tools.filter(tool => !['browser_open', 'browser_discover'].includes(tool.name))) {
    assert(tool.inputSchema.required.includes('taskId'), `${tool.name} addresses a task`);
    for (const property of ['profileCapability', 'sessionId', 'routeId', 'revision', 'provider']) {
      assert(!Object.hasOwn(tool.inputSchema.properties, property), `${tool.name} hides owner plumbing`);
    }
  }
  await assert.rejects(client.send('tools/call', {
    name: 'service_profile_lease_renew', arguments: {},
  }), /Unknown MCP tool/, 'retired public operations are removed, not hidden');
  await assert.rejects(client.send('tools/call', {
    name: 'browser_command', arguments: {taskId: 'absent-task', action: 'service_viewer_lease_request', params: {}},
  }), /action .* is not supported/, 'advanced dispatch cannot reintroduce retired authority');
  const discovery = await client.send('tools/call', {
    name: 'browser_discover', arguments: {category: 'browser', operation: 'press'},
  });
  assert(JSON.stringify(discovery).includes('key'), 'selected advanced operation includes argument metadata');
  assert(Buffer.byteLength(JSON.stringify(discovery)) <= 15000, 'discovery loads one bounded operation');
  const initialized = await client.send('initialize', {protocolVersion: '2025-06-18'});
  assert(!initialized.instructions.includes('service_access_plan'), 'default guidance uses workflow tools');
  await assert.rejects(client.send('tools/call', {name: 'browser_command',
    arguments: {taskId: 'absent-task', action: 'press', params: {sessionName: 'foreign', key: 'Enter'}}}),
    /reserved/, 'page params cannot retarget another task');
  console.log('Default workflow MCP catalog and retired dispatch passed');
} finally {
  client.close();
  context.cleanupTempHome();
}
