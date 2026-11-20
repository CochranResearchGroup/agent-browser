import {dashboardLocalFailure} from './response-failure.ts';

/** The existing browser menus' inspection only. No mutation choice is accepted. */
export function inspectDashboardBrowserOperation(scope:Record<string,unknown>,fetcher:typeof fetch=globalThis.fetch):Promise<Response> {
  const fields=['action','profileId','sourceProfileId','sessionId','sessionName','browserId','operationId','kind','schemaVersion','executablePath'];
  if(!scope||typeof scope!=='object'||Array.isArray(scope)||Object.keys(scope).some(key=>!fields.includes(key))||!['browser_session_build_select','browser_session_mode_convert','browser_session_profile_duplicate'].includes(String(scope.action))) {
    throw dashboardLocalFailure(new TypeError('Use the original browser operation inspection action and scope; mutation choices are not accepted.'),'dashboard_browser_inspection_scope_invalid','packages/dashboard/src/lib/browser-operation-readback.ts','inspectDashboardBrowserOperation');
  }
  return fetcher('/api/command',{method:'POST',headers:{'Content-Type':'application/json'},signal:AbortSignal.timeout(15000),body:JSON.stringify({...scope,choice:'inspect'})});
}
