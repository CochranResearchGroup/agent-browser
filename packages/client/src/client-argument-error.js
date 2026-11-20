import { clientRecoveryFor } from './client-recovery.js';
// @ts-check
/** Local validation refused before this helper dispatched an owner request.
 * This constructor is used only by reviewed input/configuration checks. Owner,
 * transport, readiness and policy refusals must keep their own failure objects.
 * @param {string} message @param {string} source @param {string} owningFunction @param {string[]} [checkedFields]
 */
export function clientArgumentError(message, source, owningFunction, checkedFields = []) {
  return Object.assign(new TypeError(message), {
    code: 'client_argument_invalid',
    clientRecovery: clientRecoveryFor('client_argument_invalid',{source,owningFunction}),
    source, owningFunction,
    logging: { state: 'local_only', ownerOccurrenceId: null, reason: 'No owner request was dispatched by this validation. Use an existing caller observer if available; record only code, source and owningFunction, never arguments or credentials.' },
    recovery: {
      schemaVersion: 'agent-browser.client-argument-recovery.v1',
      requiredActor: 'caller',
      source, owningFunction, checkedFields,
      effectState: 'not_dispatched_by_this_validation',
      retryDisposition: 'correct_arguments_before_resubmitting',
      inspection: 'Read the named helper contract and the original TypeError message for the exact missing, invalid or contradictory field.',
      remediation: 'Correct that field using existing owner-approved configuration and scope. Preserve original task/request identifiers. Do not loosen permission policy or substitute an unrelated profile, task or endpoint.',
      verification: 'Revalidate the corrected arguments using the same pure request builder or client constructor where available. If this was an HTTP helper configuration check, verify the existing authenticated endpoint through getServiceStatus before resuming the same intended call.',
      resumeCondition: 'The named input contract is satisfied within the existing authorization. This validation dispatched no owner request; any earlier operation in the workflow still requires its original readback before replay.',
      maximumAutomaticAttempts: 1,
    },
  });
}
