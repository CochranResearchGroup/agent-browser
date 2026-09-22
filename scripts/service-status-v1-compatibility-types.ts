import type { ServiceStatusResponse } from '../packages/client/src/service-observability.generated.js';

const resourcePressure = {
  state: 'clear',
  totalProcessCount: 0,
  correlatedProcessCount: 0,
  candidateCount: 0,
  protectedCount: 0,
  observedCount: 0,
  observedUnownedAgentBrowserProcessCount: 0,
  candidateRssBytes: 0,
  totalRssBytes: 0,
  reasons: [],
};

export const oldV1ServiceStatusResponse = {
  service_state: {},
  profileAllocations: [],
  browserSessionAuthority: {
    schemaVersion: 1,
    summary: {
      modeledBrowserCount: 0,
      viableBrowserCount: 0,
      attentionBrowserCount: 0,
      nonViableBrowserCount: 0,
    },
    resourcePressure,
    browserVerdicts: [],
  },
} satisfies ServiceStatusResponse;

export const currentV1ServiceStatusResponse = {
  service_state: {},
  profileAllocations: [],
  presentationKeeper: {
    schemaVersion: 'agent-browser.presentation-keeper-status.v1',
    supervisor: { state: 'failed', code: 'route_keeper_supervisor_terminated' },
    hostGeneration: 3,
    state: 'unavailable',
    readyRouteCount: 0,
    configuredSlotCount: 6,
    minimumReady: 1,
    warmTarget: 4,
    minimumSatisfied: false,
    warmTargetSatisfied: false,
    unavailableReason: 'route_keeper_supervisor_terminated',
    allocation: {
      state: 'over_target',
      browserCount: 7,
      occupiedDisplayCount: 3,
      maximumDisplays: 2,
      maximumBrowsersPerDisplay: 4,
    },
  },
  browserSessionAuthority: {
    schemaVersion: 1,
    availability: 'unknown',
    summary: {
      modeledBrowserCount: 0,
      viableBrowserCount: 0,
      attentionBrowserCount: 0,
      nonViableBrowserCount: 0,
      unknownBrowserCount: 0,
    },
    resourcePressure,
    browserVerdicts: [],
  },
  statusProjection: {
    schemaVersion: 1,
    authority: {
      source: 'reconciled_service_state',
      projectedAt: '2026-08-10T12:00:00.000Z',
    },
    observations: {
      state: 'unavailable',
      source: 'unavailable_status_observation_adapter',
      sourceHostId: null,
      observedAt: null,
      validUntil: null,
      maxAgeMs: 5000,
      manualBrowsersState: 'unavailable',
      browserProcessState: 'unavailable',
      errors: [],
      viewStreams: [],
    },
  },
} satisfies ServiceStatusResponse;
