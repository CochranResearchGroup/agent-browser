import { randomUUID } from 'node:crypto';
import { mkdirSync, renameSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { developmentPresentationProviderDescriptor } from './development-presentation-provider.js';
import { writeProviderAuthority } from './development-presentation-provider-deployment.js';

export function scaleOutDevelopmentPresentation({
  env = process.env,
  effects,
} = {}) {
  if (!effects) throw new Error('Development presentation scale-out requires an effect adapter');
  const descriptor = developmentPresentationProviderDescriptor(env);
  const productionBefore = effects.snapshotProduction();
  const before = effects.observe(descriptor);
  const beforeRoutes = readyRoutes(descriptor, before);
  const admission = effects.pressureAdmission(descriptor, before);
  const admittedMaximum = Number(admission?.admittedMaximum);
  if (!Number.isInteger(admittedMaximum) || admittedMaximum < descriptor.warmSlots) {
    throw new Error('Development presentation pressure admission is invalid');
  }
  const effectiveMaximum = Math.min(descriptor.hardMaxSlots, admittedMaximum);
  if (beforeRoutes.length >= effectiveMaximum) {
    return lifecycleDecision(descriptor, {
      state: 'deferred',
      reason: beforeRoutes.length >= descriptor.hardMaxSlots
        ? 'configured_hard_maximum'
        : 'pressure_admission',
      beforeSlots: beforeRoutes.length,
      afterSlots: beforeRoutes.length,
      pressureAdmission: admission,
      productionUnchanged: true,
    });
  }
  const route = descriptor.routes.find((candidate) =>
    !beforeRoutes.some((ready) => ready.routeId === candidate.routeId));
  if (!route) throw new Error('Development presentation inventory has no absent route to provision');

  let after;
  let afterRoutes;
  let display;
  try {
    effects.provisionRoute(route, descriptor);
    const afterProvision = effects.observe(descriptor);
    display = afterProvision.displays.find((candidate) =>
      candidate.displayReservationId === route.displayReservationId && candidate.ready === true);
    if (!display?.displayName || display.user !== route.user) {
      throw new Error(`Development presentation route did not become ready: ${route.routeId}`);
    }
    effects.grantDisplayAccess(display, descriptor);
    after = effects.observe(descriptor);
    afterRoutes = readyRoutes(descriptor, after);
    if (afterRoutes.length !== beforeRoutes.length + 1 ||
        !afterRoutes.some((candidate) => candidate.routeId === route.routeId)) {
      throw new Error(`Development presentation scale-out did not add exactly one route: ${route.routeId}`);
    }
  } catch (error) {
    after = effects.observe(descriptor);
    afterRoutes = readyRoutes(descriptor, after);
    writeProviderAuthority(descriptor, after);
    effects.assertProductionUnchanged(productionBefore, effects.snapshotProduction());
    return lifecycleDecision(descriptor, {
      state: 'quarantined',
      reason: 'provision_failed',
      error: error instanceof Error ? error.message : String(error),
      routeId: route.routeId,
      slotId: route.slotId,
      displayReservationId: route.displayReservationId,
      beforeSlots: beforeRoutes.length,
      afterSlots: afterRoutes.length,
      pressureAdmission: admission,
      cleanupObligation: cleanupObligation(route, 'provision_failed'),
      productionUnchanged: true,
    });
  }
  writeProviderAuthority(descriptor, after);
  effects.assertProductionUnchanged(productionBefore, effects.snapshotProduction());
  return lifecycleDecision(descriptor, {
    state: 'provisioned',
    routeId: route.routeId,
    slotId: route.slotId,
    displayReservationId: route.displayReservationId,
    displayName: display.displayName,
    beforeSlots: beforeRoutes.length,
    afterSlots: afterRoutes.length,
    pressureAdmission: admission,
    productionUnchanged: true,
  });
}

export function scaleInDevelopmentPresentation({
  env = process.env,
  effects,
} = {}) {
  if (!effects) throw new Error('Development presentation scale-in requires an effect adapter');
  const descriptor = developmentPresentationProviderDescriptor(env);
  const productionBefore = effects.snapshotProduction();
  const before = effects.observe(descriptor);
  const beforeRoutes = readyRoutes(descriptor, before);
  if (beforeRoutes.length <= descriptor.warmSlots) {
    return lifecycleDecision(descriptor, {
      state: 'deferred',
      reason: 'warm_minimum',
      beforeSlots: beforeRoutes.length,
      afterSlots: beforeRoutes.length,
      productionUnchanged: true,
    });
  }
  const route = beforeRoutes
    .filter((candidate) => candidate.ordinal > descriptor.warmSlots)
    .sort((left, right) => right.ordinal - left.ordinal)[0];
  if (!route) throw new Error('Development presentation elastic route identity is missing');
  const cooldown = effects.cooldownStatus
    ? effects.cooldownStatus(route, descriptor)
    : { ready: true, elapsedMs: null, requiredMs: null };
  if (cooldown.ready !== true) {
    return lifecycleDecision(descriptor, {
      state: 'deferred',
      reason: 'cooldown_not_elapsed',
      routeId: route.routeId,
      beforeSlots: beforeRoutes.length,
      afterSlots: beforeRoutes.length,
      cooldown,
      productionUnchanged: true,
    });
  }
  const references = effects.referenceCheck(route, descriptor);
  if (!references || references.routeId !== route.routeId) {
    throw new Error(`Development presentation reference receipt mismatched: ${route.routeId}`);
  }
  if ((references.ambiguities || []).length > 0) {
    return lifecycleDecision(descriptor, {
      state: 'quarantined',
      reason: 'reference_ambiguity',
      routeId: route.routeId,
      beforeSlots: beforeRoutes.length,
      afterSlots: beforeRoutes.length,
      references,
      productionUnchanged: true,
    });
  }
  if ((references.blockers || []).length > 0) {
    return lifecycleDecision(descriptor, {
      state: 'deferred',
      reason: 'referenced',
      routeId: route.routeId,
      beforeSlots: beforeRoutes.length,
      afterSlots: beforeRoutes.length,
      references,
      productionUnchanged: true,
    });
  }

  let after;
  let afterRoutes;
  try {
    effects.reclaimRoute(route, descriptor);
    after = effects.observe(descriptor);
    afterRoutes = readyRoutes(descriptor, after);
    if (afterRoutes.length !== beforeRoutes.length - 1 ||
        afterRoutes.some((candidate) => candidate.routeId === route.routeId)) {
      throw new Error(`Development presentation scale-in did not reclaim exactly one route: ${route.routeId}`);
    }
  } catch (error) {
    after = effects.observe(descriptor);
    afterRoutes = readyRoutes(descriptor, after);
    writeProviderAuthority(descriptor, after);
    effects.assertProductionUnchanged(productionBefore, effects.snapshotProduction());
    return lifecycleDecision(descriptor, {
      state: 'quarantined',
      reason: 'reclaim_failed',
      error: error instanceof Error ? error.message : String(error),
      routeId: route.routeId,
      slotId: route.slotId,
      displayReservationId: route.displayReservationId,
      beforeSlots: beforeRoutes.length,
      afterSlots: afterRoutes.length,
      cooldown,
      references,
      cleanupObligation: cleanupObligation(route, 'reclaim_failed'),
      productionUnchanged: true,
    });
  }
  writeProviderAuthority(descriptor, after);
  effects.assertProductionUnchanged(productionBefore, effects.snapshotProduction());
  return lifecycleDecision(descriptor, {
    state: 'reclaimed',
    routeId: route.routeId,
    slotId: route.slotId,
    displayReservationId: route.displayReservationId,
    beforeSlots: beforeRoutes.length,
    afterSlots: afterRoutes.length,
    cooldown,
    references,
    productionUnchanged: true,
  });
}

function readyRoutes(descriptor, observation) {
  const displays = new Map(
    (observation?.displays || [])
      .filter((display) => display.ready === true && display.displayName)
      .map((display) => [display.displayReservationId, display]),
  );
  return descriptor.routes
    .filter((route) => displays.has(route.displayReservationId))
    .map((route) => ({ ...route, display: displays.get(route.displayReservationId) }));
}

function lifecycleDecision(descriptor, decision) {
  const receipt = writeLifecycleReceipt(descriptor, decision);
  return {
    success: decision.state !== 'quarantined',
    environment: 'development',
    ...decision,
    receipt,
  };
}

function writeLifecycleReceipt(descriptor, decision) {
  mkdirSync(descriptor.receiptsDir, { recursive: true, mode: 0o700 });
  const id = `${Date.now()}-${process.pid}-${randomUUID()}`;
  const path = join(descriptor.receiptsDir, `lifecycle-${id}.json`);
  const temporary = `${path}.next`;
  writeFileSync(temporary, `${JSON.stringify({
    schemaVersion: 'agent-browser.development-presentation-lifecycle-receipt.v1',
    environment: 'development',
    observedAt: new Date().toISOString(),
    ...decision,
  }, null, 2)}\n`, { mode: 0o600 });
  renameSync(temporary, path);
  return path;
}

function cleanupObligation(route, reason) {
  return {
    id: `cleanup:${route.routeId}`,
    reason,
    routeId: route.routeId,
    slotId: route.slotId,
    displayReservationId: route.displayReservationId,
    routeUser: route.user,
    viewerSession: route.viewerSession,
    viewerProfile: route.viewerProfile,
  };
}
