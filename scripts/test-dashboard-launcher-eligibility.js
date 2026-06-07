#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  createLauncherServiceRequestFromAccessPlan,
  createLauncherSessionArgsFromAccessPlan,
  deriveLauncherEligibilityPreview,
  launcherAccessPlanPosture,
} from '../packages/dashboard/src/lib/launcher-eligibility.ts';

const registry = {
  generatedAt: '2026-05-23T12:00:00.000Z',
  browserHosts: [
    {
      id: 'local-host',
      name: 'Local headed host',
      reachable: true,
      lifecycleOwner: 'agent_browser',
      remoteViewSupport: true,
    },
  ],
  browserExecutables: [
    {
      id: 'stealth-exe',
      hostId: 'local-host',
      buildLabel: 'stealthcdp_chromium',
      executablePath: '/opt/chromium-stealth/chrome',
      fresh: true,
    },
    {
      id: 'stock-exe',
      hostId: 'local-host',
      buildLabel: 'stock_chrome',
      executablePath: '/usr/bin/google-chrome',
      fresh: true,
    },
  ],
  browserCapabilities: [
    {
      id: 'stealth-capability',
      hostId: 'local-host',
      executableId: 'stealth-exe',
      cdpSupported: true,
      cdpFreeLaunchSupported: false,
      streamingSupported: true,
    },
    {
      id: 'stock-capability',
      hostId: 'local-host',
      executableId: 'stock-exe',
      cdpSupported: true,
      cdpFreeLaunchSupported: false,
      streamingSupported: true,
    },
  ],
  profileCompatibility: [
    {
      id: 'ready-stealth',
      profileId: 'profile-ready',
      hostId: 'local-host',
      executableId: 'stealth-exe',
      compatible: true,
      requiresOperatorOverride: false,
      reason: 'same_browser_family',
    },
    {
      id: 'ready-stock',
      profileId: 'profile-ready',
      hostId: 'local-host',
      executableId: 'stock-exe',
      compatible: true,
      requiresOperatorOverride: false,
      reason: 'same_browser_family',
    },
    {
      id: 'manual-stealth',
      profileId: 'profile-manual',
      hostId: 'local-host',
      executableId: 'stealth-exe',
      compatible: true,
      requiresOperatorOverride: false,
      reason: 'same_browser_family',
    },
    {
      id: 'mismatch-stealth',
      profileId: 'profile-mismatch',
      hostId: 'local-host',
      executableId: 'stealth-exe',
      compatible: false,
      requiresOperatorOverride: false,
      reason: 'browser_family_mismatch',
      notes: 'Chrome-family profile evidence does not match this executable.',
    },
  ],
  browserPreferenceBindings: [],
  validationEvidence: [
    {
      id: 'stealth-launch-pass',
      hostId: 'local-host',
      executableId: 'stealth-exe',
      capabilityId: 'stealth-capability',
      kind: 'launch',
      state: 'passed',
      checkedAt: '2026-05-23T12:00:00.000Z',
      evidence: 'no-launch smoke passed',
    },
  ],
};

const profiles = [
  {
    id: 'profile-ready',
    name: 'Research profile',
    browserBuild: 'stealthcdp_chromium',
    targetServiceIds: ['acs'],
    accountIds: ['research@example.test'],
    targetReadiness: [
      {
        targetServiceId: 'acs',
        loginId: 'research',
        state: 'ready',
      },
    ],
  },
  {
    id: 'profile-manual',
    name: 'Manual profile',
    targetReadiness: [
      {
        targetServiceId: 'canva',
        loginId: 'design',
        state: 'needs_manual_seeding',
        manualSeedingRequired: true,
        recommendedAction: 'Open detached headed browser for manual login.',
      },
    ],
  },
  {
    id: 'profile-mismatch',
    name: 'Mismatch profile',
    targetReadiness: [
      {
        targetServiceId: 'legacy',
        state: 'ready',
      },
    ],
  },
];

const allocations = [
  {
    profileId: 'profile-ready',
    profileName: 'Research profile',
    serviceNames: ['JournalDownloader'],
    agentNames: ['codex'],
    taskNames: ['downloadArticle'],
    leaseState: 'available',
  },
  {
    profileId: 'profile-manual',
    profileName: 'Manual profile',
    leaseState: 'available',
  },
  {
    profileId: 'profile-mismatch',
    profileName: 'Mismatch profile',
    leaseState: 'available',
  },
];

const accessPlans = [
  {
    comboId: 'launch:profile-ready:registry:local-host:stealth-exe',
    profileId: 'profile-ready',
    browserBuild: 'stealthcdp_chromium',
    decision: {
      recommendedAction: 'queue_service_tab_request',
      launchPosture: {
        browserBuild: 'stealthcdp_chromium',
        viewStreamProvider: 'rdp_gateway',
        controlInputProvider: 'manual_attached_desktop',
        displayIsolation: 'private_virtual_display',
      },
      profileReuse: {
        recommendedAction: 'reuse_existing_browser',
        selectedProfileId: 'profile-ready',
        reusableBrowserId: 'browser-ready',
        reusableBrowserIds: ['browser-ready'],
        compatibleLiveBrowserCount: 1,
        sameProfileLiveBrowserIds: ['browser-ready'],
        activeLeaseSessionIds: [],
        activeLeaseCount: 0,
        duplicatePressure: false,
        profileLeasePolicy: 'wait',
        reasons: ['compatible_live_browser_available'],
      },
      serviceRequest: {
        available: true,
        route: '/api/service/request',
        helper: 'requestServiceTab',
        request: {
          action: 'tab_new',
          serviceName: 'JournalDownloader',
          agentName: 'codex',
          taskName: 'downloadArticle',
          targetServiceIds: ['acs'],
          accountIds: ['research@example.test'],
          browserBuild: 'stealthcdp_chromium',
          runtimeProfile: 'profile-ready',
          profileLeasePolicy: 'wait',
          params: {
            browserHost: 'remote_headed',
            viewStreamProvider: 'rdp_gateway',
            controlInputProvider: 'manual_attached_desktop',
            displayIsolation: 'private_virtual_display',
          },
        },
      },
    },
    browserCapabilityEvidence: {
      browserExecutables: [
        {
          id: 'stealth-exe',
          hostId: 'local-host',
          buildLabel: 'stealthcdp_chromium',
          executablePath: '/opt/chromium-stealth/chrome',
        },
      ],
    },
  },
  {
    comboId: 'launch:profile-ready:registry:local-host:stock-exe',
    profileId: 'profile-ready',
    browserBuild: 'stock_chrome',
    decision: {
      attention: {
        required: true,
        severity: 'warning',
        presentation: 'client_decides',
        message: 'Verify freshness before relying on authenticated automation.',
        reason: 'verify_or_seed_profile_before_authenticated_work',
      },
      recommendedAction: 'verify_or_seed_profile_before_authenticated_work',
      launchPosture: {
        browserBuild: 'stock_chrome',
        viewStreamProvider: 'cdp_screencast',
        controlInputProvider: 'cdp_input',
      },
      serviceRequest: {
        available: true,
        request: {
          action: 'tab_new',
          browserBuild: 'stock_chrome',
          runtimeProfile: 'profile-ready',
          profileLeasePolicy: 'wait',
        },
      },
    },
  },
  {
    comboId: 'launch:profile-manual:registry:local-host:stealth-exe',
    profileId: 'profile-manual',
    browserBuild: 'stealthcdp_chromium',
    decision: {
      recommendedAction: 'manual_seeding_required',
      serviceRequest: {
        request: {
          action: 'tab_new',
          blockedByManualAction: true,
          manualSeedingRequired: true,
        },
      },
    },
  },
];

function rowBy(preview, profileId, executableId) {
  const row = preview.rows.find((item) => item.profileId === profileId && item.executableId === executableId);
  assert.ok(row, `Missing ${profileId} ${executableId} launcher row`);
  return row;
}

const preview = deriveLauncherEligibilityPreview({
  profiles,
  allocations,
  browserCapabilityRegistry: registry,
  accessPlans,
  serviceRequestActions: ['tab_new', 'cdp_free_launch'],
});

const ready = rowBy(preview, 'profile-ready', 'stealth-exe');
assert.equal(ready.status, 'eligible');
assert.equal(ready.launchAction, 'tab_new');
assert.equal(ready.remoteView, 'controllable');
assert.equal(ready.browserHost, 'remote_headed');
assert.equal(ready.accessPlanFetched, true);
assert.match(ready.reason, /Reuse compatible live browser browser-ready/);
assert.equal(ready.serviceReason, 'reuse_existing_browser');

const leaseWaitPreview = deriveLauncherEligibilityPreview({
  profiles: [profiles[0]],
  allocations: [allocations[0]],
  browserCapabilityRegistry: registry,
  accessPlans: [{
    ...accessPlans[0],
    decision: {
      ...accessPlans[0].decision,
      profileReuse: {
        recommendedAction: 'wait_for_profile_lease',
        selectedProfileId: 'profile-ready',
        reusableBrowserId: null,
        reusableBrowserIds: [],
        compatibleLiveBrowserCount: 0,
        sameProfileLiveBrowserIds: ['browser-ready'],
        activeLeaseSessionIds: ['session-a'],
        activeLeaseCount: 1,
        duplicatePressure: false,
        profileLeasePolicy: 'wait',
        reasons: ['profile_lease_active'],
      },
    },
  }],
  serviceRequestActions: ['tab_new'],
});
const leaseWaitReady = rowBy(leaseWaitPreview, 'profile-ready', 'stealth-exe');
assert.equal(leaseWaitReady.status, 'eligible');
assert.match(leaseWaitReady.reason, /Queue through the profile lease held by session-a/);
assert.equal(leaseWaitReady.serviceReason, 'wait_for_profile_lease');

const noNativeRemoteRegistry = {
  ...registry,
  browserHosts: registry.browserHosts.map((host) => ({
    ...host,
    remoteViewSupport: false,
  })),
  browserCapabilities: registry.browserCapabilities.map((capability) => ({
    ...capability,
    streamingSupported: false,
  })),
};
const plannedRemotePreview = deriveLauncherEligibilityPreview({
  profiles: [profiles[0]],
  allocations: [allocations[0]],
  browserCapabilityRegistry: noNativeRemoteRegistry,
  accessPlans: [accessPlans[0]],
  serviceRequestActions: ['tab_new'],
});
const plannedRemoteReady = rowBy(plannedRemotePreview, 'profile-ready', 'stealth-exe');
assert.equal(plannedRemoteReady.status, 'eligible');
assert.equal(plannedRemoteReady.browserHost, 'remote_headed');
assert.equal(plannedRemoteReady.remoteView, 'controllable');

const remoteReadinessPreview = deriveLauncherEligibilityPreview({
  profiles: [profiles[0]],
  allocations: [allocations[0]],
  browserCapabilityRegistry: registry,
  accessPlans: [{
    ...accessPlans[0],
    readinessSummary: {
      remoteViewReadiness: {
        components: [
          {
            component: 'public_ingress',
            status: 'failed',
            evidence: 'gateway timeout',
            recovery: 'Inspect DNS, proxy, and dashboard route configuration for the public Guacamole path.',
          },
        ],
      },
    },
  }],
  serviceRequestActions: ['tab_new'],
});
const remoteReadinessBlocked = rowBy(remoteReadinessPreview, 'profile-ready', 'stealth-exe');
assert.equal(remoteReadinessBlocked.status, 'blocked');
assert.equal(remoteReadinessBlocked.reasonSource, 'readiness');
assert.match(remoteReadinessBlocked.reason, /public ingress: Inspect DNS/);

const missingValidation = rowBy(preview, 'profile-ready', 'stock-exe');
assert.equal(missingValidation.status, 'blocked');
assert.match(missingValidation.reason, /No passed browser capability validation evidence/);

const warningPreview = deriveLauncherEligibilityPreview({
  profiles: [profiles[0]],
  allocations: [allocations[0]],
  browserCapabilityRegistry: {
    ...registry,
    validationEvidence: [
      ...registry.validationEvidence,
      {
        id: 'stock-launch-pass',
        hostId: 'local-host',
        executableId: 'stock-exe',
        capabilityId: 'stock-capability',
        kind: 'launch',
        state: 'passed',
        checkedAt: '2026-05-23T12:00:00.000Z',
      },
    ],
  },
  accessPlans: [accessPlans[1]],
  serviceRequestActions: ['tab_new'],
});
const warningReady = rowBy(warningPreview, 'profile-ready', 'stock-exe');
assert.equal(warningReady.status, 'eligible');
assert.match(warningReady.reason, /Verify freshness/);

const manual = rowBy(preview, 'profile-manual', 'stealth-exe');
assert.equal(manual.status, 'needs-operator-action');
assert.match(manual.reason, /manual login/);

const mismatch = rowBy(preview, 'profile-mismatch', 'stealth-exe');
assert.equal(mismatch.status, 'blocked');
assert.match(mismatch.reason, /profile evidence|browser_family_mismatch/i);

const noAccessPlanPreview = deriveLauncherEligibilityPreview({
  profiles: [profiles[0]],
  allocations: [allocations[0]],
  browserCapabilityRegistry: registry,
  accessPlans: [],
  serviceRequestActions: ['tab_new'],
});
assert.notEqual(rowBy(noAccessPlanPreview, 'profile-ready', 'stealth-exe').status, 'eligible');

const noActionPreview = deriveLauncherEligibilityPreview({
  profiles: [profiles[0]],
  allocations: [allocations[0]],
  browserCapabilityRegistry: registry,
  accessPlans: [accessPlans[0]],
  serviceRequestActions: [],
});
assert.equal(rowBy(noActionPreview, 'profile-ready', 'stealth-exe').status, 'blocked');

assert.equal(preview.summary.runtimeProfiles, 3);
assert.equal(preview.summary.registryExecutables, 2);
assert.equal(preview.summary.eligible, 1);

const posture = launcherAccessPlanPosture(accessPlans[0]);
assert.deepEqual(posture, {
  action: 'tab_new',
  helper: 'requestServiceTab',
  route: '/api/service/request',
  profileLeasePolicy: 'wait',
  browserBuild: 'stealthcdp_chromium',
  url: '',
  displayIsolation: 'private_virtual_display',
  viewStreamProvider: 'rdp_gateway',
  controlInputProvider: 'manual_attached_desktop',
});

const request = createLauncherServiceRequestFromAccessPlan(accessPlans[0], {
  url: 'https://example.test/start',
  displayIsolation: 'shared_display',
  viewStreamProvider: 'novnc',
  controlInputProvider: 'vnc_input',
  jobTimeoutMs: 60000,
});
assert.deepEqual(request, {
  action: 'tab_new',
  serviceName: 'JournalDownloader',
  agentName: 'codex',
  taskName: 'downloadArticle',
  targetServiceIds: ['acs'],
  accountIds: ['research@example.test'],
  browserBuild: 'stealthcdp_chromium',
  runtimeProfile: 'profile-ready',
  profileLeasePolicy: 'wait',
  jobTimeoutMs: 60000,
  url: 'https://example.test/start',
  params: {
    browserHost: 'remote_headed',
    viewStreamProvider: 'novnc',
    controlInputProvider: 'vnc_input',
    displayIsolation: 'shared_display',
    url: 'https://example.test/start',
  },
});

const sessionArgs = createLauncherSessionArgsFromAccessPlan(accessPlans[0], {
  sessionName: 'workspace-1',
  url: 'https://example.test/start',
  displayIsolation: 'shared_display',
  viewStreamProvider: 'rdp_gateway',
  controlInputProvider: 'manual_attached_desktop',
  executableId: 'stealth-exe',
  browserHostId: 'local-host',
});
assert.deepEqual(sessionArgs, [
  '--session',
  'workspace-1',
  '--executable-path',
  '/opt/chromium-stealth/chrome',
  '--runtime-profile',
  'profile-ready',
  '--browser-host',
  'remote_headed',
  '--view-stream-provider',
  'rdp_gateway',
  '--control-input-provider',
  'manual_attached_desktop',
  '--display-isolation',
  'shared_display',
  '--headed',
  'open',
  'https://example.test/start',
]);

const rdpRequest = createLauncherServiceRequestFromAccessPlan(accessPlans[1], {
  viewStreamProvider: 'rdp_gateway',
});
assert.deepEqual(rdpRequest, {
  action: 'tab_new',
  browserBuild: 'stock_chrome',
  runtimeProfile: 'profile-ready',
  profileLeasePolicy: 'wait',
  params: {
    browserHost: 'remote_headed',
    headless: false,
    viewStreamProvider: 'rdp_gateway',
    displayIsolation: 'shared_display',
    controlInputProvider: 'manual_attached_desktop',
  },
});

const postureOnlyRdpRequest = createLauncherServiceRequestFromAccessPlan({
  decision: {
    launchPosture: {
      browserBuild: 'stealthcdp_chromium',
      viewStreamProvider: 'rdp_gateway',
      controlInputProvider: 'manual_attached_desktop',
      displayIsolation: 'shared_display',
    },
    serviceRequest: {
      available: true,
      request: {
        action: 'tab_new',
        browserBuild: 'stealthcdp_chromium',
        runtimeProfile: 'profile-ready',
        profileLeasePolicy: 'wait',
      },
    },
  },
}, {
  displayIsolation: 'service_default',
  viewStreamProvider: 'service_default',
  controlInputProvider: 'service_default',
});
assert.deepEqual(postureOnlyRdpRequest, {
  action: 'tab_new',
  browserBuild: 'stealthcdp_chromium',
  runtimeProfile: 'profile-ready',
  profileLeasePolicy: 'wait',
  params: {
    displayIsolation: 'shared_display',
    viewStreamProvider: 'rdp_gateway',
    browserHost: 'remote_headed',
    headless: false,
    controlInputProvider: 'manual_attached_desktop',
  },
});

assert.throws(
  () => createLauncherServiceRequestFromAccessPlan({ decision: {} }),
  /Missing access plan service request/,
);

assert.throws(
  () => createLauncherServiceRequestFromAccessPlan(accessPlans[2]),
  /manual profile seeding/,
);

console.log('Dashboard launcher eligibility smoke passed');
