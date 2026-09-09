#!/usr/bin/env node

import { runP158LiveCampaignEntrypoint } from './lib/p158-live-campaign-entrypoint.js';

function argument(name) {
  const index = process.argv.indexOf(name);
  return index < 0 ? null : process.argv[index + 1];
}

const descriptorPath = argument('--descriptor');
const descriptorSha256 = argument('--descriptor-sha256');

try {
  const result = await runP158LiveCampaignEntrypoint({ descriptorPath, descriptorSha256 });
  process.stdout.write(`${JSON.stringify(result)}\n`);
} catch (error) {
  process.stderr.write(`${JSON.stringify({
    error: error?.code ?? 'campaign_entrypoint_failed',
    message: error?.message ?? String(error),
    terminalReceipt: error?.terminalReceipt ?? null,
  })}\n`);
  process.exitCode = 1;
}
