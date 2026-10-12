#!/usr/bin/env node
/** Qualify discovery through the installed Codex client without a model turn. */
import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { cpSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const binary = process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD;
const skill = process.env.AGENT_BROWSER_SMOKE_INSTALLED_SKILL;
const chrome = process.env.AGENT_BROWSER_SMOKE_BROWSER_EXECUTABLE;
assert(
  binary && skill && chrome,
  'Pin the installed candidate, installed skill and reviewed browser'
);
assert(binary.startsWith('/') && skill.startsWith('/'), 'Use absolute installed paths');
const root = mkdtempSync(join(tmpdir(), 'ab-supported-client-'));
const skillTarget = join(root, '.agents', 'skills', 'agent-browser');
mkdirSync(resolve(skillTarget, '..'), { recursive: true });
cpSync(skill, skillTarget, { recursive: true });
const serverConfig = {
  command: binary,
  args: ['mcp', 'serve'],
  env: {
    AGENT_BROWSER_HOME: join(root, 'browser'),
    AGENT_BROWSER_SOCKET_DIR: join(root, 'socket'),
    AGENT_BROWSER_RUNTIME_HOST: '1',
    AGENT_BROWSER_EXECUTABLE_PATH: chrome,
    XDG_RUNTIME_DIR: join(root, 'runtime'),
    XDG_CONFIG_HOME: join(root, 'config'),
    XDG_DATA_HOME: join(root, 'data'),
  },
};
mkdirSync(serverConfig.env.XDG_RUNTIME_DIR, { mode: 0o700 });
function toml(value) {
  if (Array.isArray(value)) return `[${value.map(toml).join(',')}]`;
  if (value && typeof value === 'object')
    return `{${Object.entries(value)
      .map(([key, v]) => `${key}=${toml(v)}`)
      .join(',')}}`;
  return JSON.stringify(value);
}
const config = `mcp_servers=${toml({ p238_browser: serverConfig })}`;
const child = spawn('codex', ['app-server', '-c', config], {
  cwd: root,
  stdio: ['pipe', 'pipe', 'pipe'],
});
let buffer = '';
let id = 0;
let stderr = '';
const pending = new Map();
child.stderr.on('data', (chunk) => {
  stderr = (stderr + chunk).slice(-4000);
});
child.on('exit', (code) => {
  for (const request of pending.values()) {
    clearTimeout(request.timer);
    request.reject(new Error(`Client exited ${code}: ${stderr}`));
  }
  pending.clear();
});
child.stdout.on('data', (chunk) => {
  buffer += chunk;
  let end;
  while ((end = buffer.indexOf('\n')) >= 0) {
    const line = buffer.slice(0, end);
    buffer = buffer.slice(end + 1);
    let message;
    try {
      message = JSON.parse(line);
    } catch {
      continue;
    }
    const request = pending.get(message.id);
    if (!request) continue;
    pending.delete(message.id);
    clearTimeout(request.timer);
    if (message.error) request.reject(new Error(JSON.stringify(message.error)));
    else request.resolve(message.result);
  }
});
function send(method, params) {
  const nextId = ++id;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      pending.delete(nextId);
      reject(new Error(`Client timeout: ${method}`));
    }, 45000);
    pending.set(nextId, { resolve, reject, timer });
    child.stdin.write(JSON.stringify({ id: nextId, method, params }) + '\n');
  });
}
let threadId;
const tasks = [];
async function call(tool, args) {
  const result = await send('mcpServer/tool/call', {
    threadId,
    server: 'p238_browser',
    tool,
    arguments: args,
  });
  assert(!result.isError, JSON.stringify(result));
  const payload = JSON.parse(result.content.find((block) => block.type === 'text').text);
  assert.equal(payload.success, true, JSON.stringify(payload));
  return payload.data;
}
function ownedProcesses() {
  return readdirSync('/proc')
    .filter((pid) => /^\d+$/.test(pid) && Number(pid) !== process.pid)
    .filter((pid) => {
      try {
        return (
          readFileSync(`/proc/${pid}/environ`, 'utf8')
            .split('\0')
            .includes(`AGENT_BROWSER_HOME=${serverConfig.env.AGENT_BROWSER_HOME}`) ||
          readFileSync(`/proc/${pid}/cmdline`, 'utf8')
            .replaceAll('\0', ' ')
            .includes(serverConfig.env.AGENT_BROWSER_HOME + '/')
        );
      } catch {
        return false;
      }
    })
    .map(Number);
}
try {
  await send('initialize', {
    clientInfo: { name: 'agent-browser-supported-client-smoke', version: '1' },
    capabilities: { experimentalApi: true },
  });
  child.stdin.write(JSON.stringify({ method: 'initialized' }) + '\n');
  const configRead = await send('config/read', { cwd: root, includeLayers: false });
  assert.equal(configRead.config.mcp_servers.p238_browser.command, binary);
  // Selecting this server avoids connecting unrelated user-scoped servers.
  const result = await send('mcpServerStatus/list', {
    serverName: 'p238_browser',
    detail: 'full',
  });
  assert.equal(result.data.length, 1);
  const server = result.data[0];
  assert.equal(server.name, 'p238_browser');
  assert.equal(server.toolsError, null);
  assert(server.resources.some((resource) => resource.uri === 'agent-browser://operating-guide'));
  assert(server.resources.some((resource) => resource.uri === 'agent-browser://contracts'));
  const guide = await send('mcpServer/resource/read', {
    server: 'p238_browser',
    uri: 'agent-browser://operating-guide',
  });
  assert(Buffer.byteLength(JSON.stringify(guide)) <= 4000, 'Client operating guide stays bounded');
  assert(JSON.stringify(guide).includes('browser_open'));
  const names = Object.keys(server.tools).sort();
  assert.deepEqual(
    names,
    [
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
    ].sort()
  );
  const schemaBytes = Buffer.byteLength(JSON.stringify(Object.values(server.tools)));
  assert(schemaBytes <= 15000, 'Actual client inventory stays bounded');
  const skills = await send('skills/list', { cwds: [root], forceReload: true });
  const encodedSkills = JSON.stringify(skills);
  assert(encodedSkills.includes(skillTarget), 'Client discovers the installed workflow skill');
  const skillBytes = Buffer.byteLength(readFileSync(join(skillTarget, 'SKILL.md')));
  assert(skillBytes <= 10000, 'Ordinary guidance stays bounded');
  const isolatedServers = Object.fromEntries(
    Object.keys(configRead.config.mcp_servers).map((name) => [name, { enabled: false }])
  );
  isolatedServers.p238_browser = { ...serverConfig, enabled: true };
  const thread = await send('thread/start', {
    cwd: root,
    ephemeral: true,
    approvalPolicy: 'never',
    config: { mcp_servers: isolatedServers },
  });
  threadId = thread.thread.id;
  const page = 'data:text/html,<title>Client workflow</title><input id="draft">';
  const alice = await call('browser_open', {
    profile: 'client-shared',
    profileMode: 'persistent',
    taskKey: 'alice',
    url: page,
  });
  tasks.push(alice);
  const bob = await call('browser_open', {
    profile: 'client-shared',
    profileMode: 'persistent',
    taskKey: 'bob',
    url: page,
  });
  tasks.push(bob);
  assert.equal(alice.browserId, bob.browserId);
  assert.notEqual(alice.tabId, bob.tabId);
  await call('browser_fill', { taskId: alice.taskId, selector: '#draft', value: 'client edit' });
  const replay = await call('browser_open', {
    profile: 'client-shared',
    profileMode: 'persistent',
    taskKey: 'alice',
    url: 'about:blank',
  });
  assert.equal(replay.tabId, alice.tabId);
  const continued = await call('browser_continue', { taskId: alice.taskId });
  assert.equal(continued.tabId, alice.tabId);
  const value = await call('browser_command', {
    taskId: alice.taskId,
    action: 'inputvalue',
    params: { selector: '#draft' },
  });
  assert(JSON.stringify(value).includes('client edit'));
  await call('browser_close', { taskId: alice.taskId });
  tasks.splice(tasks.indexOf(alice), 1);
  const peer = await call('browser_continue', { taskId: bob.taskId });
  assert.equal(peer.tabId, bob.tabId);
  await call('browser_close', { taskId: bob.taskId });
  tasks.splice(tasks.indexOf(bob), 1);
  console.log(
    JSON.stringify({
      client: execFileSync('codex', ['--version'], { encoding: 'utf8' }).trim(),
      binary,
      toolCount: names.length,
      resourceCount: server.resources.length,
      schemaBytes,
      skillBytes,
      skillDiscovered: true,
      modelTurns: 0,
      workflow: 'same-profile peers, edit-preserving replay, continuation and scoped close',
    })
  );
} finally {
  for (const task of tasks.reverse()) {
    try {
      await call('browser_close', { taskId: task.taskId });
    } catch (error) {
      console.error(`Owned task cleanup: ${error.message}`);
    }
  }
  child.stdin.end();
  child.kill('SIGTERM');
  await new Promise((resolve) => {
    if (child.exitCode !== null || child.signalCode !== null) return resolve();
    child.once('exit', resolve);
    setTimeout(() => {
      child.kill('SIGKILL');
      resolve();
    }, 3000).unref();
  });
  for (const pid of ownedProcesses()) {
    try {
      if (ownedProcesses().includes(pid)) process.kill(pid, 'SIGTERM');
    } catch {
      /* Already exited. */
    }
  }
  const deadline = Date.now() + 5000;
  while (ownedProcesses().length && Date.now() < deadline)
    await new Promise((resolve) => setTimeout(resolve, 50));
  assert.equal(ownedProcesses().length, 0, `Retained own cleanup obligation at ${root}`);
  rmSync(root, { recursive: true, force: true });
  console.log('Supported client fixture census: zero owned residue');
}
