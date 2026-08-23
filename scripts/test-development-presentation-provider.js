#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  DEVELOPMENT_PRESENTATION_PROVIDER_SCHEMA,
  developmentPresentationProviderDescriptor,
  developmentPresentationProviderManifest,
  developmentAgentSkillStatus,
  doctorDevelopmentPresentationProvider,
  synchronizeDevelopmentAgentSkill,
  validateDevelopmentPresentationProviderIsolation,
} from './lib/development-presentation-provider.js';

const fixture = mkdtempSync(join(tmpdir(), 'agent-browser-dev-provider-'));
const userHome = join(fixture, 'user');
const env = { ...process.env, AGENT_BROWSER_DEV_USER_HOME: userHome };

try {
  const descriptor = developmentPresentationProviderDescriptor(env);
  assert.equal(descriptor.environment, 'development');
  assert.equal(descriptor.warmSlots, 4);
  assert.equal(descriptor.hardMaxSlots, 6);
  assert.equal(descriptor.routes.length, 6);
  assert.equal(new Set(descriptor.routes.map((route) => route.routeId)).size, 6);
  assert.equal(new Set(descriptor.routes.map((route) => route.user)).size, 6);
  assert.equal(new Set(descriptor.routes.map((route) => route.connectionId)).size, 6);
  assert.equal(new Set(descriptor.routes.map((route) => route.display)).size, 6);
  assert.equal(descriptor.ports.guacamole, 8093);
  assert.equal(descriptor.ports.guacd, 4823);
  assert.equal(descriptor.ports.postgres, 55433);
  assert.match(descriptor.root, /agent-browser-dev\/presentation-provider$/);
  assert.match(descriptor.skill.target, /agent-browser-dev\/home\/\.codex\/skills\/agent-browser$/);
  assert.doesNotMatch(JSON.stringify(descriptor.routes), /route-a|route-b/i);
  assert.doesNotThrow(() => validateDevelopmentPresentationProviderIsolation(descriptor));
  assert.equal(developmentAgentSkillStatus({ env }).state, 'unconfigured');
  const skill = synchronizeDevelopmentAgentSkill({ env });
  assert.equal(skill.environment, 'development');
  assert.equal(developmentAgentSkillStatus({ env }).state, 'current');
  assert.notEqual(descriptor.skill.target, join(userHome, '.codex', 'skills', 'agent-browser'));

  for (const [field, value] of [
    ['AGENT_BROWSER_DEV_GUACAMOLE_PORT', '8092'],
    ['AGENT_BROWSER_DEV_GUACD_PORT', '4822'],
    ['AGENT_BROWSER_DEV_POSTGRES_PORT', '5432'],
  ]) {
    assert.throws(
      () => validateDevelopmentPresentationProviderIsolation(
        developmentPresentationProviderDescriptor({ ...env, [field]: value }),
      ),
      /collides with production/i,
    );
  }
  assert.throws(
    () => validateDevelopmentPresentationProviderIsolation(
      developmentPresentationProviderDescriptor({
        ...env,
        AGENT_BROWSER_DEV_PRESENTATION_ROOT: join(userHome, '.agent-browser', 'presentation-provider'),
      }),
    ),
    /overlaps production/i,
  );
  assert.throws(
    () => validateDevelopmentPresentationProviderIsolation(
      developmentPresentationProviderDescriptor({
        ...env,
        AGENT_BROWSER_DEV_GUACD_PORT: '3490',
      }),
    ),
    /duplicate ports/i,
  );

  const optional = doctorDevelopmentPresentationProvider({ env });
  assert.equal(optional.success, true);
  assert.equal(optional.status.state, 'unconfigured');
  assert.equal(optional.status.ready, false);
  assert.equal(optional.status.blocking, false);
  const required = doctorDevelopmentPresentationProvider({
    env: { ...env, AGENT_BROWSER_DEV_PRESENTATION_PROVIDER_REQUIRED: '1' },
  });
  assert.equal(required.success, false);
  assert.equal(required.status.blocking, true);

  mkdirSync(descriptor.root, { recursive: true });
  writeFileSync(
    descriptor.manifest,
    `${JSON.stringify(developmentPresentationProviderManifest(descriptor), null, 2)}\n`,
  );
  const configured = doctorDevelopmentPresentationProvider({ env });
  assert.equal(configured.success, true);
  assert.equal(configured.status.state, 'configured');
  assert.equal(configured.status.ready, true);
  assert.equal(configured.status.manifest.schemaVersion, DEVELOPMENT_PRESENTATION_PROVIDER_SCHEMA);

  const drifted = developmentPresentationProviderManifest(descriptor);
  drifted.ports.guacamole = 8092;
  writeFileSync(descriptor.manifest, `${JSON.stringify(drifted, null, 2)}\n`);
  const drift = doctorDevelopmentPresentationProvider({ env });
  assert.equal(drift.success, false);
  assert.equal(drift.status.state, 'drifted');
  assert.equal(drift.status.ready, false);

  console.log('Development presentation provider fixture passed');
} finally {
  rmSync(fixture, { recursive: true, force: true });
}
