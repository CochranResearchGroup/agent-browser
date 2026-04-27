#!/usr/bin/env node

import { spawn } from 'node:child_process';

const smokeScripts = [
  ['shutdown health', 'scripts/smoke-service-shutdown-health.js'],
  ['recovery HTTP', 'scripts/smoke-service-recovery-http.js'],
  ['recovery MCP', 'scripts/smoke-service-recovery-mcp.js'],
  ['recovery override HTTP', 'scripts/smoke-service-recovery-override-http.js'],
  ['recovery override MCP', 'scripts/smoke-service-recovery-override-mcp.js'],
  ['incident HTTP/MCP parity', 'scripts/smoke-service-incident-parity.js'],
];

function runSmoke([label, script]) {
  return new Promise((resolve, reject) => {
    console.log(`Running service health smoke: ${label}`);
    const child = spawn(process.execPath, [script], {
      cwd: process.cwd(),
      env: process.env,
      stdio: 'inherit',
    });

    child.on('error', reject);
    child.on('exit', (code, signal) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`${label} failed${signal ? ` with signal ${signal}` : ` with exit code ${code}`}`));
    });
  });
}

for (const smoke of smokeScripts) {
  await runSmoke(smoke);
}

console.log('Service health live smoke gate passed');
