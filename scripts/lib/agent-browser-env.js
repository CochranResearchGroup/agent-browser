import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

export function loadEnvFile(envPath, environment = process.env) {
  if (!existsSync(envPath)) return;
  const text = readFileSync(envPath, 'utf8');
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const separatorIndex = trimmed.indexOf('=');
    if (separatorIndex <= 0) continue;
    const key = trimmed.slice(0, separatorIndex).trim();
    let value = trimmed.slice(separatorIndex + 1).trim();
    if ((value.startsWith('"') && value.endsWith('"')) || (value.startsWith("'") && value.endsWith("'"))) {
      value = value.slice(1, -1);
    }
    if (!Object.hasOwn(environment, key)) environment[key] = value.replace(/\\"/g, '"');
  }
}

export function loadAgentBrowserEnv(environment = process.env) {
  const agentHome = environment.AGENT_BROWSER_HOME || join(environment.HOME || '', '.agent-browser');
  loadEnvFile(join(agentHome, '.env'), environment);
  loadEnvFile(
    environment.AGENT_BROWSER_GUACAMOLE_SECRET_FILE || join(agentHome, 'secrets', 'guacamole.env'),
    environment,
  );
}
