/** Synthetic-only qualification of actual public hosted reconnect routes.
 * The caller owns one known task, Native allocation, cleanup and effects policy.
 * This helper never kills/restarts a browser or retries an uncertain dispatch.
 * Actual browser loss and desktop-restart cases remain provider-free evidence.
 */
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {performance} from 'node:perf_hooks';

export async function qualifyHostedBrowserReconnect({workflow,task,save,syntheticOnly}) {
 assert.equal(syntheticOnly,true,'This fixture is synthetic only');
 assert.equal(typeof workflow,'function');assert(task?.taskId);
 const receipt={passed:false,checks:[],warmStatusMs:[],actualBrowserLossExercised:false,actualDesktopRestartExercised:false};
 const identity=value=>Object.fromEntries(['taskId','browserId','profileId','tabId'].map(key=>[key,value[key]]));
 const status=()=>workflow('browser_reconnect',{taskId:task.taskId,operation:'status'});
 let current=await status();const original=identity(current.host),originalPolicy=current.reconnect.lossPolicy;
 let human=null,returnDispatched=false;
 const change=async(operation,extra={})=>{
  const result=await workflow('browser_reconnect',{taskId:task.taskId,operation,requestId:randomUUID(),expectedRevision:current.reconnect.revision,...extra});
  assert.deepEqual(identity(result.host),original,'Lifecycle operation must preserve all logical identities');current=result;return result;
 };
 try {
  const control=await workflow('browser_control',{taskId:task.taskId,operation:'status'});assert.equal(control.control.owner,'agent');
  human=await workflow('browser_control',{taskId:task.taskId,operation:'takeover',expectedEpoch:control.control.epoch,actor:'synthetic reconnect fixture',reason:'Verify disconnection preserves human takeover',pauseReads:false});
  assert.equal(human.control.owner,'human');
  await change('disconnect');assert.equal(current.host.controllerState.owner,'human');
  assert.equal(current.reconnect.retained,true,'Disconnect must retain this task, including existing leave-open intent');receipt.checks.push('disconnect-preserves-human-and-logical-identities');
  await change('reconnect');assert.equal(current.reconnect.state,'connected');assert.equal(current.reconnect.disconnectHold,false);assert.equal(current.host.controllerState.owner,'human');
  receipt.checks.push('fresh-exact-native-reconnect-preserves-human');
  returnDispatched=true;
  const returned=await workflow('browser_control',{taskId:task.taskId,operation:'return',expectedEpoch:human.control.epoch,actor:'synthetic reconnect fixture',reason:'Explicit return after exact same-task observation'});
  assert.equal(returned.control.owner,'agent');receipt.checks.push('explicit-return-observes-before-agent-resume');
  await change('configure',{lossPolicy:'reopen'});assert.equal(current.reconnect.lossPolicy,'reopen');
  await change('configure',{lossPolicy:'report_only'});assert.equal(current.reconnect.lossPolicy,'report_only');
  if(originalPolicy!=='report_only')await change('configure',{lossPolicy:originalPolicy});
  receipt.checks.push('app-loss-policy-selection-is-explicit-and-restored');
  const continued=await workflow('browser_continue',{taskId:task.taskId});assert.equal(continued.taskId,task.taskId);assert.equal(continued.browserId,original.browserId);
  receipt.checks.push('same-browser-continuation');
  for(let index=0;index<12;index++){
   const start=performance.now();const observed=await status();receipt.warmStatusMs.push(performance.now()-start);
   assert.deepEqual(identity(observed.host),original);assert.equal(observed.reconnect.pending,false);assert.equal(observed.reconnect.automaticReplay,false);
  }
  const sorted=[...receipt.warmStatusMs].sort((a,b)=>a-b);receipt.warmStatusP95Ms=sorted[Math.ceil(sorted.length*.95)-1];
  receipt.warmStatusTargetMs=250;receipt.warmStatusTargetMet=receipt.warmStatusP95Ms<250;
  receipt.final=current.reconnect;receipt.passed=true;
  await save('hosted-reconnect-qualification.json',receipt);return receipt;
 } catch(error) {
  // One deliberate cleanup return is allowed only after a known successful take.
  // A return with an unknown outcome is never dispatched again.
  if(human&&!returnDispatched){returnDispatched=true;try{await workflow('browser_control',{taskId:task.taskId,operation:'return',expectedEpoch:human.control.epoch,actor:'synthetic reconnect fixture',reason:'Stop fixture after a known successful takeover'});receipt.cleanupReturn='completed';}catch{receipt.cleanupReturn='failed-or-unknown-no-replay';}}
  receipt.failureCode=String(error?.code??error?.name??'Error');await save('hosted-reconnect-qualification.json',receipt);throw error;
 }
}
