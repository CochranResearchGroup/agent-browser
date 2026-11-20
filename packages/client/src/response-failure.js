import { clientRecoveryFor } from './client-recovery.js';
// @ts-check
/** True for an explicit application refusal, including a nested owner result.
 * Absence of a success flag alone is not a refusal.
 * @param {any} response
 */
export function isResponseFailure(response) {
  return response?.success === false || response?.ok === false ||
    response?.data?.success === false || response?.data?.ok === false;
}

/** Preserve the owner's recovery object and occurrence identity without
 * interpreting uncertainty as permission to retry. No observation or input is
 * performed by this adapter.
 * @param {any} response @param {string} fallback @param {Record<string, any>} [context]
 */
export function responseFailureError(response, fallback, context={}) {
  const failure = response?.failure ?? response?.data?.failure ?? response?.error?.data?.failure ?? null;
  const candidate = failure?.code ?? response?.data?.code ?? response?.error?.code ?? response?.code;
  const code = typeof candidate === 'string' && /^[a-z][a-z0-9_]{0,127}$/.test(candidate) ? candidate : fallback;
  return Object.assign(new Error(code), {
    code, failure,
    clientRecovery: {...clientRecoveryFor(context.localRecoveryCode??fallback,context),ownerRecoveryAvailable:failure!==null},
    occurrenceId: failure?.occurrenceId ?? response?.data?.occurrenceId ?? response?.occurrenceId ?? null,
    requestId: response?.id ?? response?.error?.data?.requestId ?? null,
  });
}
