#!/usr/bin/env node

import { createConnection } from 'node:net';
import { spawn } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const rootDir = new URL('..', import.meta.url).pathname;
const tempHome = mkdtempSync(join(tmpdir(), 'ab-sp-'));
const realHome = process.env.HOME;
const cargoHome = process.env.CARGO_HOME || (realHome ? join(realHome, '.cargo') : undefined);
const rustupHome = process.env.RUSTUP_HOME || (realHome ? join(realHome, '.rustup') : undefined);
const session = `sp-${process.pid}`;
const runtimeProfile = `smoke-${process.pid}`;
const serviceName = 'RuntimeProfileSmoke';
const agentName = 'smoke-agent';
const taskName = 'profileSessionResourceSmoke';
const agentHome = join(tempHome, '.agent-browser');
const socketDir = join(tempHome, 's');

mkdirSync(socketDir, { recursive: true });

const env = {
  ...process.env,
  HOME: tempHome,
  AGENT_BROWSER_HOME: agentHome,
  AGENT_BROWSER_SOCKET_DIR: socketDir,
  AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS: '0',
  ...(cargoHome ? { CARGO_HOME: cargoHome } : {}),
  ...(rustupHome ? { RUSTUP_HOME: rustupHome } : {}),
};

const timeout = setTimeout(() => {
  fail('Timed out waiting for service profile MCP smoke to complete');
}, 90000);

function cargoArgs(args) {
  return ['run', '--quiet', '--manifest-path', 'cli/Cargo.toml', '--', ...args];
}

function runCli(args) {
  return new Promise((resolve, reject) => {
    const proc = spawnCargo(args);
    const procTimeout = setTimeout(() => {
      proc.kill('SIGTERM');
      reject(new Error(`agent-browser ${args.join(' ')} timed out`));
    }, 60000);
    let out = '';
    let err = '';
    proc.stdout.setEncoding('utf8');
    proc.stderr.setEncoding('utf8');
    proc.stdout.on('data', (chunk) => {
      out += chunk;
    });
    proc.stderr.on('data', (chunk) => {
      err += chunk;
    });
    proc.on('error', (err) => {
      clearTimeout(procTimeout);
      reject(err);
    });
    proc.on('exit', (code, signal) => {
      clearTimeout(procTimeout);
      if (code === 0) {
        resolve({ stdout: out, stderr: err });
      } else {
        reject(
          new Error(
            `agent-browser ${args.join(' ')} failed: code=${code} signal=${signal}\n${out}${err}`,
          ),
        );
      }
    });
  });
}

function spawnCargo(args) {
  return spawn('cargo', cargoArgs(args), {
    cwd: rootDir,
    env,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function parseJsonOutput(output, label) {
  const text = output.trim();
  try {
    return JSON.parse(text);
  } catch (err) {
    throw new Error(`Failed to parse ${label} JSON output: ${err.message}\n${output}`);
  }
}

function readResourceContents(response, label) {
  assert(response.success === true, `${label} read failed: ${JSON.stringify(response)}`);
  const contents = response.data?.contents;
  assert(contents && typeof contents === 'object', `${label} resource missing contents`);
  return contents;
}

function daemonEndpoint() {
  if (process.platform === 'win32') {
    const port = Number(readFileSync(join(socketDir, `${session}.port`), 'utf8').trim());
    return { port, host: '127.0.0.1' };
  }
  return { path: join(socketDir, `${session}.sock`) };
}

function sendRawCommand(command) {
  return new Promise((resolve, reject) => {
    const token = readFileSync(join(socketDir, `${session}.token`), 'utf8').trim();
    const socket = createConnection(daemonEndpoint());
    let response = '';
    const procTimeout = setTimeout(() => {
      socket.destroy();
      reject(new Error(`raw daemon command ${command.action} timed out`));
    }, 30000);
    socket.setEncoding('utf8');
    socket.on('connect', () => {
      socket.write(`${JSON.stringify({ ...command, _agentBrowserAuthToken: token })}\n`);
    });
    socket.on('data', (chunk) => {
      response += chunk;
      if (response.includes('\n')) {
        clearTimeout(procTimeout);
        socket.end();
        resolve(JSON.parse(response.trim()));
      }
    });
    socket.on('error', (err) => {
      clearTimeout(procTimeout);
      reject(err);
    });
  });
}

async function cleanup() {
  clearTimeout(timeout);
  try {
    await runCli(['--json', '--session', session, 'close']);
  } catch {
    // The smoke may fail before the daemon starts; cleanup must stay best-effort.
  }
  rmSync(tempHome, { recursive: true, force: true });
}

async function fail(message) {
  await cleanup();
  console.error(message);
  process.exit(1);
}

try {
  const pageHtml = [
    '<!doctype html>',
    '<html>',
    '<head><title>Service Profile MCP Smoke</title></head>',
    '<body><h1 id="ready">Service Profile MCP Smoke</h1></body>',
    '</html>',
  ].join('');
  const pageUrl = `data:text/html;charset=utf-8,${encodeURIComponent(pageHtml)}`;

  const openResult = await runCli([
    '--json',
    '--session',
    session,
    '--runtime-profile',
    runtimeProfile,
    '--args',
    '--no-sandbox',
    'open',
    pageUrl,
  ]);
  const opened = parseJsonOutput(openResult.stdout, 'open');
  assert(opened.success === true, `Open command failed: ${openResult.stdout}${openResult.stderr}`);

  const launchResult = await sendRawCommand({
    id: 'service-profile-smoke-launch',
    action: 'launch',
    headless: true,
    runtimeProfile,
    args: ['--no-sandbox'],
    serviceName,
    agentName,
    taskName,
  });
  assert(
    launchResult.success === true,
    `Metadata launch command failed: ${JSON.stringify(launchResult)}`,
  );

  const profilesResourceResult = await runCli([
    '--json',
    'mcp',
    'read',
    'agent-browser://profiles',
  ]);
  const profilesResource = readResourceContents(
    parseJsonOutput(profilesResourceResult.stdout, 'mcp profiles resource'),
    'profiles',
  );
  const profile = profilesResource.profiles?.find((profile) => profile.id === runtimeProfile);
  assert(
    profile,
    `MCP profiles resource did not include runtime profile ${runtimeProfile}: ${JSON.stringify(
      profilesResource,
    )}`,
  );
  assert(profile.name === runtimeProfile, `Profile name was ${profile.name}`);
  assert(profile.persistent === true, 'Profile was not marked persistent');
  assert(profile.allocation === 'per_service', `Profile allocation was ${profile.allocation}`);
  assert(profile.keyring === 'basic_password_store', `Profile keyring was ${profile.keyring}`);
  assert(
    profile.sharedServiceIds?.includes(serviceName),
    `Profile sharedServiceIds missing ${serviceName}: ${JSON.stringify(profile)}`,
  );
  assert(
    typeof profile.userDataDir === 'string' && profile.userDataDir.includes(runtimeProfile),
    `Profile userDataDir did not include runtime profile name: ${JSON.stringify(profile)}`,
  );

  const sessionsResourceResult = await runCli([
    '--json',
    'mcp',
    'read',
    'agent-browser://sessions',
  ]);
  const sessionsResource = readResourceContents(
    parseJsonOutput(sessionsResourceResult.stdout, 'mcp sessions resource'),
    'sessions',
  );
  const persistedSession = sessionsResource.sessions?.find((item) => item.id === session);
  assert(
    persistedSession,
    `MCP sessions resource did not include active session ${session}: ${JSON.stringify(
      sessionsResource,
    )}`,
  );
  assert(
    persistedSession.serviceName === serviceName,
    `Session serviceName was ${persistedSession.serviceName}`,
  );
  assert(persistedSession.agentName === agentName, `Session agentName was ${persistedSession.agentName}`);
  assert(persistedSession.taskName === taskName, `Session taskName was ${persistedSession.taskName}`);
  assert(
    persistedSession.profileId === runtimeProfile,
    `Session profileId was ${persistedSession.profileId}`,
  );
  assert(persistedSession.lease === 'exclusive', `Session lease was ${persistedSession.lease}`);
  assert(
    persistedSession.cleanup === 'close_browser',
    `Session cleanup was ${persistedSession.cleanup}`,
  );
  assert(
    persistedSession.browserIds?.includes(`session:${session}`),
    `Session browserIds missing active browser: ${JSON.stringify(persistedSession)}`,
  );

  const eventsResourceResult = await runCli([
    '--json',
    'mcp',
    'read',
    'agent-browser://events',
  ]);
  const eventsResource = readResourceContents(
    parseJsonOutput(eventsResourceResult.stdout, 'mcp events resource'),
    'events',
  );
  const launchEvent = eventsResource.events?.find(
    (event) =>
      event.kind === 'browser_launch_recorded' &&
      event.sessionId === session &&
      event.profileId === runtimeProfile &&
      event.serviceName === serviceName &&
      event.agentName === agentName &&
      event.taskName === taskName,
  );
  assert(
    launchEvent,
    `MCP events resource missing launch event context: ${JSON.stringify(eventsResource)}`,
  );
  assert(launchEvent.serviceName === serviceName, `Event serviceName was ${launchEvent.serviceName}`);
  assert(launchEvent.agentName === agentName, `Event agentName was ${launchEvent.agentName}`);
  assert(launchEvent.taskName === taskName, `Event taskName was ${launchEvent.taskName}`);
  assert(
    launchEvent.browserId === `session:${session}`,
    `Event browserId was ${launchEvent.browserId}`,
  );

  await cleanup();
  console.log('Service profile MCP smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
