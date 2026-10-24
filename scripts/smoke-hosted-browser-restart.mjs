/** Actual Native restart oracle. Caller retains exclusive disposable fixture
 * custody and supplies the supported Native624 restart and independent evidence.
 * No effects at import; no automatic retry, navigation or fallback destination.
 */
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
const logical=task=>['taskId','browserId','profileId','tabId'].map(key=>task[key]);
const desktop=value=>({desktopId:value.desktopId,generation:value.generation});

export async function qualifyHostedBrowserRestart({workflow,task,restartNative,observeProcess,observePeers,observeReopenedNative,save,syntheticOnly=false}){
 assert.equal(syntheticOnly,true);
 const original=logical(task);assert(original.every(value=>typeof value==='string'&&value));
 const old=desktop(task.host.desktop),peers=await observePeers();
 const before=await observeProcess();assert.equal(before.alive,true);
 const receipt={version:1,passed:false,checks:[],physicalRelocation:'not_exercised',automaticReplay:false};
 const record=async(name,detail={})=>{receipt.checks.push({name,...detail});await save('restart-acceptance.json',receipt);};
 let current=await workflow('browser_reconnect',{taskId:task.taskId,operation:'status'});
 const change=async(operation,extra={})=>{
  const value=await workflow('browser_reconnect',{taskId:task.taskId,operation,expectedRevision:current.reconnect.revision,requestId:randomUUID(),...extra});
  assert.deepEqual(logical(value.host),original);assert(value.reconnect.revision>current.reconnect.revision);
  assert.equal(value.reconnect.automaticReplay,false);current=value;return value;
 };
 await change('configure',{lossPolicy:'report_only'});
 const transition=await restartNative(before);
 assert.equal(transition.state,'ready_for_explicit_rebind');
 assert.equal(transition.before.desktopId,transition.after.desktopId);
 assert.equal(transition.before.assignmentId,transition.after.assignmentId);
 assert.equal(transition.before.lifecycleGeneration,old.generation);
 assert(transition.after.viewingGeneration>transition.before.viewingGeneration);
 assert.equal(transition.launchBinding.readinessScope,'live_resource');
 assert.equal(transition.applicationLoss.observed,'lost');
 assert.equal(transition.websiteEffectsReplayed,false);
 const gone=await observeProcess(before);assert.equal(gone.pidAbsent,true);
 await record('native-generation-and-browser-retirement-observed',{transition});
 // Report-only reconciliation may observe desktop loss before browser loss.
 await change('reconnect');
 assert(['browser_lost','desktop_lost'].includes(current.reconnect.state));
 assert.deepEqual(desktop(current.host.desktop),old,'Observed generation must not silently replace host binding');
 // Public desktop generation is assignment lifecycle, not Native viewing.
 const next={desktopId:old.desktopId,generation:transition.after.lifecycleGeneration};
 await change('configure',{lossPolicy:'reopen'});
 await change('rebind',{desktop:next});
 assert.deepEqual(desktop(current.host.desktop),old,'Rebind records intent before explicit reopen');
 assert.deepEqual(desktop(current.reconnect.recoveryDesktop),next);
 await record('explicit-new-generation-is-recovery-intent-only');
 await change('reopen');
 assert.equal(current.reconnect.state,'connected');assert.equal(current.reconnect.pending,false);
 assert.deepEqual(desktop(current.host.desktop),next);assert.equal(current.reconnect.recoveryDesktop,null);
 const replacement=await observeProcess();assert.equal(replacement.alive,true);
 assert.notEqual(replacement.startToken,before.startToken);assert.equal(replacement.profilePath,before.profilePath);
 const page=(await workflow('browser_command',{taskId:task.taskId,action:'evaluate',params:{script:'({url:location.href,body:document.body?.textContent??""})'}})).result;
 assert.equal(page.url,'about:blank');assert.equal(page.body,'');
 const native=await observeReopenedNative(replacement);
 assert.equal(native.viewingGeneration,transition.after.viewingGeneration);
 assert.equal(native.desktopId,transition.after.desktopId);
 assert.equal(native.processId,replacement.pid);
 assert.equal(native.windowObserved,true);
 assert.deepEqual(await observePeers(),peers);
 await record('same-logical-task-profile-new-process-blank-page',{replacement,native});
 receipt.passed=true;await save('restart-acceptance.json',receipt);return receipt;
}
