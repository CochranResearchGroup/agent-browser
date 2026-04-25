import { spawn } from 'node:child_process';
import { request } from 'node:http';
import { createConnection } from 'node:net';
import { mkdirSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

export const rootDir = new URL('..', import.meta.url).pathname;

export function createSmokeContext({ prefix, session, sessionPrefix, socketDir: customSocketDir, socketSubdir = 's' }) {
  const tempHome = mkdtempSync(join(tmpdir(), prefix));
  const realHome = process.env.HOME;
  const cargoHome = process.env.CARGO_HOME || (realHome ? join(realHome, '.cargo') : undefined);
  const rustupHome = process.env.RUSTUP_HOME || (realHome ? join(realHome, '.rustup') : undefined);
  const smokeSession = session ?? `${sessionPrefix}-${process.pid}`;
  const agentHome = join(tempHome, '.agent-browser');
  const socketDir = customSocketDir
    ? customSocketDir({ agentHome, tempHome })
    : join(tempHome, socketSubdir);

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

  return {
    agentHome,
    env,
    session: smokeSession,
    socketDir,
    tempHome,
    cleanupTempHome() {
      rmSync(tempHome, { recursive: true, force: true });
    },
  };
}

export function cargoArgs(args) {
  return ['run', '--quiet', '--manifest-path', 'cli/Cargo.toml', '--', ...args];
}

export function runCli(context, args, timeoutMs = 60000) {
  return new Promise((resolve, reject) => {
    const proc = spawn('cargo', cargoArgs(args), {
      cwd: rootDir,
      env: context.env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const procTimeout = setTimeout(() => {
      proc.kill('SIGTERM');
      reject(new Error(`agent-browser ${args.join(' ')} timed out`));
    }, timeoutMs);
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

export function parseJsonOutput(output, label) {
  const text = output.trim();
  try {
    return JSON.parse(text);
  } catch (err) {
    throw new Error(`Failed to parse ${label} JSON output: ${err.message}\n${output}`);
  }
}

export function httpJson(port, method, path, body) {
  return new Promise((resolve, reject) => {
    const rawBody = body === undefined ? undefined : JSON.stringify(body);
    const req = request(
      {
        host: '127.0.0.1',
        port,
        method,
        path,
        headers: rawBody
          ? {
              'content-type': 'application/json',
              'content-length': Buffer.byteLength(rawBody),
            }
          : undefined,
      },
      (res) => {
        let text = '';
        res.setEncoding('utf8');
        res.on('data', (chunk) => {
          text += chunk;
        });
        res.on('end', () => {
          try {
            const parsed = JSON.parse(text);
            if (!res.statusCode || res.statusCode < 200 || res.statusCode >= 300) {
              reject(new Error(`HTTP ${method} ${path} returned ${res.statusCode}: ${text}`));
              return;
            }
            resolve(parsed);
          } catch (err) {
            reject(new Error(`Failed to parse HTTP ${method} ${path}: ${err.message}\n${text}`));
          }
        });
      },
    );
    req.setTimeout(30000, () => {
      req.destroy(new Error(`HTTP ${method} ${path} timed out`));
    });
    req.on('error', reject);
    if (rawBody) req.write(rawBody);
    req.end();
  });
}

export function assert(condition, message) {
  if (!condition) throw new Error(message);
}

export function readResourceContents(response, label) {
  assert(response.success === true, `${label} read failed: ${JSON.stringify(response)}`);
  const contents = response.data?.contents;
  assert(contents && typeof contents === 'object', `${label} resource missing contents`);
  return contents;
}

export function daemonEndpoint(context) {
  if (process.platform === 'win32') {
    const port = Number(readFileSync(join(context.socketDir, `${context.session}.port`), 'utf8').trim());
    return { port, host: '127.0.0.1' };
  }
  return { path: join(context.socketDir, `${context.session}.sock`) };
}

export function sendRawCommand(context, command) {
  return new Promise((resolve, reject) => {
    const token = readFileSync(join(context.socketDir, `${context.session}.token`), 'utf8').trim();
    const socket = createConnection(daemonEndpoint(context));
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

export function createMcpStdioClient({ context, args, onFatal }) {
  const child = spawn('cargo', cargoArgs(args), {
    cwd: rootDir,
    env: context.env,
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  let stdout = '';
  let stderr = '';
  let nextId = 1;
  const pending = new Map();

  function fatal(message) {
    for (const { reject } of pending.values()) reject(new Error(message));
    pending.clear();
    void onFatal(message, stderr);
  }

  function handleLine(line) {
    let message;
    try {
      message = JSON.parse(line);
    } catch (err) {
      fatal(`MCP server emitted non-JSON stdout line: ${line}\n${err.message}`);
      return;
    }

    const pendingRequest = pending.get(message.id);
    if (!pendingRequest) {
      fatal(`Received unexpected MCP response id: ${message.id}`);
      return;
    }

    pending.delete(message.id);
    if (message.error) {
      pendingRequest.reject(
        new Error(`${pendingRequest.method} failed: ${JSON.stringify(message.error)}`),
      );
      return;
    }
    pendingRequest.resolve(message.result);
  }

  child.stdout.setEncoding('utf8');
  child.stdout.on('data', (chunk) => {
    stdout += chunk;
    let newline = stdout.indexOf('\n');
    while (newline >= 0) {
      const line = stdout.slice(0, newline).trim();
      stdout = stdout.slice(newline + 1);
      if (line) handleLine(line);
      newline = stdout.indexOf('\n');
    }
  });

  child.stderr.setEncoding('utf8');
  child.stderr.on('data', (chunk) => {
    stderr += chunk;
  });

  child.on('error', (err) => {
    fatal(`Failed to spawn MCP server: ${err.message}`);
  });

  child.on('exit', (code, signal) => {
    if (pending.size > 0) {
      fatal(`MCP server exited before all responses arrived: code=${code} signal=${signal}`);
    }
  });

  return {
    close() {
      child.stdin.end();
      child.kill('SIGTERM');
    },
    notify(method, params) {
      const notification = { jsonrpc: '2.0', method };
      if (params !== undefined) notification.params = params;
      child.stdin.write(`${JSON.stringify(notification)}\n`);
    },
    rejectPending(message) {
      for (const { reject } of pending.values()) reject(new Error(message));
      pending.clear();
    },
    send(method, params) {
      const id = nextId++;
      const request = { jsonrpc: '2.0', id, method };
      if (params !== undefined) request.params = params;
      const promise = new Promise((resolve, reject) => {
        pending.set(id, { resolve, reject, method });
      });
      child.stdin.write(`${JSON.stringify(request)}\n`);
      return promise;
    },
    stderr() {
      return stderr;
    },
  };
}

export async function closeSession(context) {
  try {
    await runCli(context, ['--json', '--session', context.session, 'close']);
  } catch {
    // The smoke may fail before the daemon starts; cleanup must stay best-effort.
  }
}
