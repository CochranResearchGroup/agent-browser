// @ts-check

/** Summarize an Auth-owned public observation without creating a request or
 * inferring delivery. The summary carries only opaque references and read-only
 * next operations; it never carries credentials or authority to perform input.
 * @param {Record<string, any>} observation
 * @returns {import('./login-progress.d.ts').LoginProgress}
 */
export function summarizeLoginProgress(observation) {
  const request = observation?.request ?? observation ?? {};
  const loginId = reference(observation?.loginId);
  const requestId = reference(request.requestId);
  const nextAction = loginId
    ? {operation:'inspect_credential_login',args:{loginId}}
    : requestId ? {operation:'inspect_credential_request',args:{requestId}} : null;
  let category='verification_incomplete';
  let waitingForUser=false;
  /** @type {'auth'|'remote-view'|'application'} */
  let component='auth';
  const evidenceReferences=[];
  const state=request.state ?? observation?.state;
  const delivery=observation?.approvalDelivery;
  if (['denied','revoked','cancelled'].includes(state)) category='authority_denied';
  else if (state==='expired') category='request_expired';
  else if (state==='unknown' || request.heldFor==='original_request_readback') category='input_outcome_unknown';
  else if (['receiving_target_unavailable','receiving_target_pending'].includes(request.heldFor ?? observation?.heldFor)) category='receiving_target_unavailable';
  else if (observation?.control?.owner==='operator') {category='control_metadata';component='remote-view';}
  else if (request.state==='pending') {
    if (['receiving_target_pending','receiving_target_unavailable'].includes(request.approvalSubmission)) category='receiving_target_unavailable';
    else if (request.approvalSubmission==='unknown') category='approval_submission_unknown';
    else if (delivery?.state==='delivered' && delivery.actionable===true && reference(delivery.evidenceRef) && requestId) {
      category='approval_delivered'; waitingForUser=true; evidenceReferences.push(delivery.evidenceRef);
    } else category=request.approvalSubmission==='accepted'?'approval_submitted':'approval_not_submitted';
  } else if (!requestId && observation?.state==='owner_review') category='credential_selection_required';
  return {
    schemaVersion:1, outcome:'incomplete', category,
    component, waitingForUser, evidenceReferences, nextAction,
    ...(loginId ? {loginId} : {}), ...(requestId ? {requestId} : {}),
  };
}

/** @param {unknown} value */
function reference(value) {
  return typeof value === 'string' && /^[A-Za-z0-9][A-Za-z0-9:._-]{0,199}$/.test(value)
    ? value : undefined;
}
