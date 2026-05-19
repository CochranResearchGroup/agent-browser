#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  profileAllocationFromLookupPayload,
  serviceProfileAllocationLookupUrl,
} from '../packages/dashboard/src/lib/service-profile-allocation.ts';

const servicePanel = readFileSync('packages/dashboard/src/components/service-panel.tsx', 'utf8');
const dashboardCss = readFileSync('packages/dashboard/src/app/globals.css', 'utf8');

assert.equal(
  serviceProfileAllocationLookupUrl('http://localhost:9223/api/service', 'journal-downloader'),
  'http://localhost:9223/api/service/profiles/journal-downloader/allocation',
);
assert.equal(
  serviceProfileAllocationLookupUrl('http://localhost:9223/api/service/', 'profile with space'),
  'http://localhost:9223/api/service/profiles/profile%20with%20space/allocation',
);
assert.throws(
  () => serviceProfileAllocationLookupUrl(' ', 'journal-downloader'),
  /service base URL/,
);
assert.throws(
  () => serviceProfileAllocationLookupUrl('http://localhost:9223/api/service', ' '),
  /profile ID/,
);

const fallback = {
  profileId: 'journal-downloader',
  leaseState: 'shared',
};
const fresh = {
  profileId: 'journal-downloader',
  leaseState: 'exclusive',
  recommendedAction: 'inspect_conflicts',
};

assert.deepEqual(
  profileAllocationFromLookupPayload(
    {
      success: true,
      data: {
        profileAllocation: fresh,
      },
    },
    fallback,
  ),
  fresh,
);
assert.deepEqual(
  profileAllocationFromLookupPayload(
    {
      success: true,
      data: {},
    },
    fallback,
  ),
  fallback,
);
assert.throws(
  () =>
    profileAllocationFromLookupPayload(
      {
        success: false,
        error: 'Profile allocation not found: journal-downloader',
      },
      fallback,
    ),
  /Profile allocation not found/,
);

assert.match(
  servicePanel,
  /browserBuild\?: string \| null;[\s\S]*accountIds\?: string\[\];[\s\S]*browserSummaries\?: ServiceProfileAllocationBrowserSummary\[\];/,
  'Dashboard profile allocation type must include browser build, login identities, and browser summaries from the service contract',
);

assert.match(
  servicePanel,
  /function profileAllocationTargetValues\(allocation: ServiceProfileAllocation\): string\[\][\s\S]*allocation\.targetReadiness[\s\S]*allocation\.targetServiceIds[\s\S]*allocation\.authenticatedServiceIds/,
  'Profile allocation rows and filters must derive target identities from service-owned readiness and target fields',
);

assert.match(
  servicePanel,
  /function profileAllocationLoginValues\(allocation: ServiceProfileAllocation\): string\[\][\s\S]*allocation\.targetReadiness[\s\S]*allocation\.accountIds/,
  'Profile allocation rows and filters must derive login identities from service-owned readiness and account fields',
);

assert.match(
  servicePanel,
  /const profileRoutingSummary = useMemo\(\(\) => \{[\s\S]*targets = new Set<string>\(\)[\s\S]*accounts = new Set<string>\(\)[\s\S]*authenticatedTargets[\s\S]*readinessAttention[\s\S]*explicitBrowserBuilds[\s\S]*profilesWithBrowsers/,
  'Profiles workspace must keep a compact identity and routing summary derived from service profile allocations',
);

assert.match(
  servicePanel,
  /const \[profileTargetFilter, setProfileTargetFilter\] = useState\("all"\);[\s\S]*const \[profileLoginFilter, setProfileLoginFilter\] = useState\("all"\);[\s\S]*const \[profileBrowserBuildFilter, setProfileBrowserBuildFilter\] = useState\("all"\);[\s\S]*const \[profileReadinessFilter, setProfileReadinessFilter\] = useState<ProfileReadinessFilter>\("all"\);/,
  'Profiles workspace must track target, login, browser-build, and readiness filters',
);

assert.match(
  servicePanel,
  /const profileTargetOptions = useMemo\([\s\S]*profileAllocations\.flatMap\(profileAllocationTargetValues\)[\s\S]*const profileLoginOptions = useMemo\([\s\S]*profileAllocations\.flatMap\(profileAllocationLoginValues\)[\s\S]*const profileBrowserBuildOptions = useMemo\([\s\S]*profileAllocations\.map\(\(allocation\) => allocation\.browserBuild\)/,
  'Profiles workspace must derive target, login, and browser-build filter options from profile allocations',
);

assert.match(
  servicePanel,
  /profileTargetFilter !== "all"[\s\S]*profileAllocationTargetValues\(allocation\)\.includes\(profileTargetFilter\)[\s\S]*profileLoginFilter !== "all"[\s\S]*profileAllocationLoginValues\(allocation\)\.includes\(profileLoginFilter\)[\s\S]*profileBrowserBuildFilter !== "all"[\s\S]*allocation\.browserBuild !== profileBrowserBuildFilter[\s\S]*profileReadinessFilter === "needs_attention"[\s\S]*profileReadinessFilter === "normal"/,
  'Profiles workspace must apply service-backed profile field filters before text search',
);

assert.match(
  servicePanel,
  /service-profile-routing-strip" aria-label="Profile identity and routing summary"[\s\S]*target identities[\s\S]*login identities[\s\S]*authenticated targets[\s\S]*profiles with browsers[\s\S]*pinned builds[\s\S]*readiness attention/,
  'Profiles workspace must render the identity and routing summary labels',
);

assert.match(
  servicePanel,
  /service-profile-field-filters" aria-label="Profile routing field filters"[\s\S]*All target identities[\s\S]*All login identities[\s\S]*All browser builds[\s\S]*Needs attention[\s\S]*No readiness attention/,
  'Profiles workspace must render target, login, browser-build, and readiness filters',
);

assert.match(
  servicePanel,
  /service-profile-route-grid[\s\S]*<strong>Target<\/strong>[\s\S]*<strong>Login<\/strong>[\s\S]*<strong>Browser build<\/strong>[\s\S]*<strong>Keyring<\/strong>/,
  'Profile allocation rows must show target, login, browser build, and keyring routing cells',
);

assert.match(
  servicePanel,
  /<EventDetailItem label="Browser build" value=\{allocation\.browserBuild\} \/>[\s\S]*<EventDetailItem label="Primary target" value=\{profileAllocationPrimaryTarget\(allocation\)\} \/>[\s\S]*<EventDetailItem label="Primary login" value=\{profileAllocationPrimaryLogin\(allocation\)\} \/>[\s\S]*<ProfileAllocationTokenSection title="Account identities" values=\{allocation\.accountIds\} \/>[\s\S]*<ProfileBrowserSummarySection rows=\{allocation\.browserSummaries\} \/>/,
  'Profile allocation detail must include browser-build, identity, account, and browser-summary routing evidence',
);

assert.match(
  dashboardCss,
  /\.service-profile-routing-strip[\s\S]*\.service-profile-field-filters[\s\S]*\.service-profile-route-grid[\s\S]*\.service-profile-route-cell[\s\S]*\.service-profile-route-detail[\s\S]*\.service-profile-attention-badge/,
  'Profiles routing UI must keep dedicated compact styling',
);

console.log('Dashboard profile allocation lookup smoke passed');
