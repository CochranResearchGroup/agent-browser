import { clientRecoveryFor } from './client-recovery.js';
// @ts-check
import { responseFailureError } from './response-failure.js';

/** Add local owner-readback recourse without changing the original Error class
 * or replacing an owner's typed failure/occurrence identity.
 * @template {Error} T
 * @param {T} error @param {string} code @param {unknown} response
 * @param {string} source @param {string} owningFunction @param {string} recipe
 */
export function clientOwnerFailure(error, code, response, source, owningFunction, recipe) {
  const owner = responseFailureError(response, code);
  return Object.assign(error, {
    code: owner.code, failure: owner.failure, occurrenceId: owner.occurrenceId,
    source, owningFunction,
    clientRecovery: {...clientRecoveryFor(code,{source,owningFunction}),ownerRecoveryAvailable:owner.failure!==null},
    logging: {state: owner.occurrenceId ? 'owner_occurrence_preserved' : 'local_only', ownerOccurrenceId: owner.occurrenceId},
    recovery: {
      schemaVersion: 'agent-browser.client-owner-recovery.v1', recipe,
      requiredActor: 'caller_with_existing_owner_scope',
      inspection: 'Read the exact retained owner task, original operation or access plan using existing identifiers and authenticated transport.',
      remediation: 'Follow the named recovery recipe. Refresh current owner evidence or use the supported scoped continuation; do not overwrite evidence, expand permission or replay an uncertain operation.',
      verification: 'Re-read the same original owner context and re-run the local readiness check. A local refusal is not proof that a previous effect did not occur.',
      resumeCondition: 'The exact original context is verified and the named readiness/policy requirement is satisfied. Unresolved original effects remain non-replayable.',
      maximumAutomaticAttempts: 1,
    },
  });
}
