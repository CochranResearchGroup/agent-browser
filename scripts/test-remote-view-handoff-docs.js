#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const files = {
  agents: readFileSync('AGENTS.md', 'utf8'),
  readme: readFileSync('README.md', 'utf8'),
  skill: readFileSync('skills/agent-browser/SKILL.md', 'utf8'),
  guide: readFileSync('docs/src/app/remote-view/page.mdx', 'utf8'),
  navigation: readFileSync('docs/src/lib/docs-navigation.ts', 'utf8'),
};

for (const [name, contents] of Object.entries({
  agents: files.agents,
  readme: files.readme,
  skill: files.skill,
  guide: files.guide,
})) {
  assert.match(
    contents,
    /\/remote-view\/<handoff-id>/,
    `${name} must name the durable opaque remote-view handoff path`,
  );
}

for (const [name, contents] of Object.entries({
  agents: files.agents,
  readme: files.readme,
  skill: files.skill,
  guide: files.guide,
})) {
  assert.match(
    contents,
    /(?:Never|Do not|must not)[^\n]*(?:providerExternalUrl|raw Guacamole)/i,
    `${name} must forbid ordinary operator handoff through a raw provider URL`,
  );
}

assert.match(
  files.skill,
  /Require `operatorVisible\.state=ready`[\s\S]*Return, store, and reopen only `handoffUrl`/,
  'the agent skill must put readiness before durable-link handoff',
);
assert.match(
  files.guide,
  /requestServiceRemoteViewHandoff\([\s\S]*console\.log\(handoff\.handoffUrl\)/,
  'the guide must include a software-client example that returns only the handoff link',
);
assert.match(
  files.guide,
  /Reconnect without opening another browser[\s\S]*Open the same `handoffUrl` again/,
  'the guide must tell agents to reuse the durable handoff during provider churn',
);
assert.match(
  files.navigation,
  /RDP Remote View[\s\S]*href: "\/remote-view"/,
  'the remote-view handoff guide must be reachable from docs navigation',
);

console.log('remote-view handoff documentation checks passed');
