#!/usr/bin/env node
/** Default MCP advertisement is qualified through the installed stdio seam. */
import assert from 'node:assert/strict';
import { createMcpStdioClient, createSmokeContext, runCli } from './smoke-utils.js';
assert(process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD, 'Pin the candidate executable');
const context = createSmokeContext({ prefix: 'ab-catalog-', sessionPrefix: 'catalog' });
const client = createMcpStdioClient({ context, args: ['mcp', 'serve'], onFatal: () => {} });
try {
  const initialized = await client.send('initialize', {
    protocolVersion: '2025-06-18',
    capabilities: {},
    clientInfo: { name: 'workflow-catalog-fixture', version: '0' },
  });
  client.notify('notifications/initialized');
  const help = await runCli(context, ['mcp', '--help']);
  assert(Buffer.byteLength(help.stdout) <= 4000, 'MCP help stays bounded');
  assert(help.stdout.includes('browser_discover'));
  assert(!help.stdout.includes('service_access_plan'));
  const { tools } = await client.send('tools/list');
  for (const operation of ['native_capture','native_input','profile_duplicate']) {
    const discovery=await client.send('tools/call',{name:'browser_discover',arguments:{category:'browser',operation}});
    const metadata=JSON.parse(discovery.content[0].text).data;
    assert.equal(metadata.operation,operation);
    assert.equal(metadata.execution.tool,'browser_command');
    assert(metadata.inputSchema?.properties || metadata.arguments?.properties || metadata.schema?.properties,'operation exposes argument metadata');
  }

  const modeDiscovery = await client.send('tools/call', {
    name: 'browser_discover', arguments: { category: 'browser', operation: 'mode' },
  });
  const modeMetadata = JSON.parse(modeDiscovery.content[0].text).data;
  assert.equal(modeMetadata.execution.action, 'mode');
  assert.deepEqual(modeMetadata.inputSchema.properties, {});

  const bytes = Buffer.byteLength(JSON.stringify(tools));
  console.log(JSON.stringify({ toolCount: tools.length, schemaBytes: bytes }));
  assert.deepEqual(
    tools.map((tool) => tool.name).sort(),
    [
      'browser_control',
      'browser_reconnect',
      'browser_policy',
      'browser_artifact',
      'browser_open',
      'browser_navigate',
      'browser_snapshot',
      'browser_click',
      'browser_fill',
      'browser_screenshot',
      'browser_tab',
      'browser_handoff',
      'browser_continue',
      'browser_close',
      'browser_command',
      'browser_discover',
    ].sort(),
    'the default catalog contains only the complete workflow surface'
  );
  assert(bytes <= 15000, 'ordinary selection must not load the service architecture');
  for (const tool of tools.filter(
    (tool) => !['browser_open', 'browser_discover'].includes(tool.name)
  )) {
    if (tool.name === 'browser_command') {
      assert(tool.inputSchema.required.includes('action'));
      assert(tool.inputSchema.anyOf.some(branch => branch.required?.includes('taskId')));
      assert(tool.inputSchema.anyOf.some(branch => branch.properties?.action?.const === 'profile_duplicate'));
    } else assert(tool.inputSchema.required.includes('taskId'), `${tool.name} addresses a task`);
    for (const property of ['profileCapability', 'sessionId', 'routeId', 'revision', 'provider']) {
      assert(
        !Object.hasOwn(tool.inputSchema.properties, property),
        `${tool.name} hides owner plumbing`
      );
    }
  }
  await assert.rejects(
    client.send('tools/call', {
      name: 'service_profile_lease_renew',
      arguments: {},
    }),
    /Unknown MCP tool/,
    'retired public operations are removed, not hidden'
  );
  await assert.rejects(
    client.send('tools/call', {
      name: 'browser_command',
      arguments: { taskId: 'absent-task', action: 'service_viewer_lease_request', params: {} },
    }),
    /action .* is not supported/,
    'advanced dispatch cannot reintroduce retired authority'
  );
  const discovery = await client.send('tools/call', {
    name: 'browser_discover',
    arguments: { category: 'browser', operation: 'press' },
  });
  assert(
    JSON.stringify(discovery).includes('key'),
    'selected advanced operation includes argument metadata'
  );
  assert(
    Buffer.byteLength(JSON.stringify(discovery)) <= 15000,
    'discovery loads one bounded operation'
  );
  assert(
    !initialized.instructions.includes('service_access_plan'),
    'default guidance uses workflow tools'
  );
  await assert.rejects(
    client.send('tools/call', {
      name: 'browser_command',
      arguments: {
        taskId: 'absent-task',
        action: 'press',
        params: { sessionName: 'foreign', key: 'Enter' },
      },
    }),
    /reserved/,
    'page params cannot retarget another task'
  );
  assert(
    Buffer.byteLength(JSON.stringify(initialized)) <= 1500,
    'initialize guidance stays bounded'
  );
  const guide = await client.send('resources/read', { uri: 'agent-browser://operating-guide' });
  assert(Buffer.byteLength(JSON.stringify(guide)) <= 4000, 'workflow guide stays bounded');
  await assert.rejects(
    client.send('tools/call', {
      name: 'browser_command',
      arguments: {
        taskId: 'absent-task',
        action: 'press',
        params: { key: 'Enter', executablePath: '/foreign' },
      },
    }),
    /not a page argument/,
    'advanced params cannot select a different browser executable'
  );
  for (const name of [
    'service_request',
    'service_access_plan',
    'service_status',
    'service_profile_lease_rejoin',
    'service_profile_lease_release',
    'service_profile_lease_reconcile',
    'service_profile_acquire',
    'service_profile_recovery_plan',
    'service_profile_recovery_apply',
    'service_profile_repair_plan',
    'service_profile_repair_apply',
    'service_profile_reset_plan',
    'service_profile_reset_apply',
    'browser_tabs',
    'browser_tab_new',
    'browser_tab_switch',
    'browser_tab_close',
  ]) {
    await assert.rejects(
      client.send('tools/call', { name, arguments: {} }),
      /Unknown MCP tool/,
      `${name} cannot bypass the workflow catalog`
    );
  }
  const contracts = await client.send('resources/read', { uri: 'agent-browser://contracts' });
  const metadata = JSON.parse(contracts.contents[0].text);
  assert.equal(
    metadata.contracts.serviceRequest.mcp.available,
    false,
    'contract metadata does not advertise an unavailable generic MCP request'
  );
  assert.deepEqual([...metadata.mcpCatalog.tools].sort(), tools.map((tool) => tool.name).sort());
  for (const category of ['browser', 'diagnostics', 'administration']) {
    const listed = await client.send('tools/call', {
      name: 'browser_discover',
      arguments: { category },
    });
    const metadata = JSON.parse(listed.content.find((block) => block.type === 'text').text).data;
    if (category === 'administration')
      assert(
        metadata.operations.includes('service_monitor_upsert'),
        'explicit discovery preserves supported administrative metadata'
      );
    if (category === 'diagnostics')
      assert(
        metadata.operations.includes('service_access_plan'),
        'explicit discovery preserves supported diagnostic metadata'
      );
    for (const operation of metadata.operations) {
      const selected = await client
        .send('tools/call', { name: 'browser_discover', arguments: { category, operation } })
        .catch((error) => {
          throw new Error(`${category}/${operation}: ${error.message}`);
        });
      assert(
        Buffer.byteLength(JSON.stringify(selected)) <= 15000,
        `${operation} metadata stays bounded`
      );
      assert(
        JSON.parse(selected.content.find((block) => block.type === 'text').text).data.inputSchema,
        `${operation} has argument metadata`
      );
    }
  }
  console.log(
    JSON.stringify({
      helpBytes: Buffer.byteLength(help.stdout),
      initializeBytes: Buffer.byteLength(JSON.stringify(initialized)),
      guideBytes: Buffer.byteLength(JSON.stringify(guide)),
    })
  );
  console.log('Default workflow MCP catalog and retired dispatch passed');
} finally {
  client.close();
  context.cleanupTempHome();
}
