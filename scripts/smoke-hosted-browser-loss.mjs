/** Injectable loss acceptance for a separately created synthetic managed task.
 * No effect runs at import. The caller owns fresh allocation, exact termination,
 * actual kernel/provider observations and cleanup. No baseline browser is selected.
 */
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {isAbsolute} from 'node:path';

const logical=task=>[task.taskId,task.browserId,task.profileId,task.tabId];
const desktopKey=desktop=>({desktopId:desktop.desktopId,generation:desktop.generation});
function identity(proof){
 assert(proof?.freshOwned===true&&proof.custody==='managed','Only a fresh isolated managed browser may be terminated');
 assert(Number.isInteger(proof.pid)&&proof.pid>0);assert(Number.isInteger(proof.uid)&&proof.uid>=0);
 assert(typeof proof.startToken==='string'&&proof.startToken.length>0);
 assert(isAbsolute(proof.executablePath)&&isAbsolute(proof.profilePath));
 return {pid:proof.pid,startToken:proof.startToken,uid:proof.uid,executablePath:proof.executablePath,profilePath:proof.profilePath};
}

/** workflow resolves unwrapped public data; unknown/rejected mutations throw.
 * observeProcess({task,expected}) returns exact kernel identity plus alive/pidAbsent.
 * terminateOwnedBrowser(identity) must target only that identity and return its
 * pid/startToken plus originalGenerationAbsent:true. Independent readback is required.
 */
export async function qualifyHostedBrowserLoss({workflow,task,desktop,observeProcess,terminateOwnedBrowser,observeDesktop,observePeers,save,confirmSyntheticPage,syntheticOnly=false}){
 assert.equal(syntheticOnly,true);assert(logical(task).every(value=>typeof value==='string'&&value));
 assert(typeof desktop?.desktopId==='string'&&Number.isSafeInteger(desktop.generation)&&desktop.generation>0);
 const receipt={version:1,taskId:task.taskId,browserId:task.browserId,profileId:task.profileId,tabId:task.tabId,checks:[],desktopRestart:{state:'not_exercised',reason:'Native consumer exposes no exact desktop-restart capability'},physicalRelocation:'not_exercised',cleanup:'caller_owned'};
 const record=async(name,detail={})=>{receipt.checks.push({name,...detail});await save('loss-acceptance.json',receipt);};
 const call=(name,args)=>workflow(name,args);
 const inspect=async()=>{
  const proof=await observeDesktop(desktop);assert.equal(proof.state,'active');assert.deepEqual(desktopKey(proof),desktopKey(desktop));return proof;
 };
 await inspect();const peersBefore=await observePeers();
 const before=await observeProcess({task,expected:null});const original=identity(before);assert.equal(before.alive,true);
 const read=async script=>(await call('browser_command',{taskId:task.taskId,action:'evaluate',params:{script}})).result;
 if(confirmSyntheticPage)assert.equal(await confirmSyntheticPage({task,workflow}),true);
 else assert.equal(await read('document.body.textContent.includes("ATTACHMENT SYNTHETIC MARKER")'),true,'Refuse to modify an unprepared page');
 const marker='lossFixture'+randomUUID().replaceAll('-','');
 assert.equal(await read(`globalThis.${marker}=1`),1);
 await record('fresh-owned-synthetic-browser-qualified',{original});
 const status=await call('browser_reconnect',{taskId:task.taskId,operation:'status'});let current=status.reconnect;assert(Number.isSafeInteger(current.revision));
 const mutate=async(operation,extra={})=>{
  const result=await call('browser_reconnect',{taskId:task.taskId,operation,requestId:'loss-'+operation+'-'+randomUUID(),expectedRevision:current.revision,...extra});
  assert.deepEqual(logical(result.host),logical(task),'Retain exact logical task/browser/profile/tab');
  assert(result.reconnect.revision>current.revision);assert.equal(result.reconnect.automaticReplay,false);current=result.reconnect;return result;
 };
 await mutate('configure',{lossPolicy:'report_only'});assert.equal(current.lossPolicy,'report_only');
 const terminated=await terminateOwnedBrowser(original);
 assert.equal(terminated.pid,original.pid);assert.equal(terminated.startToken,original.startToken);assert.equal(terminated.originalGenerationAbsent,true);
 const gone=await observeProcess({task,expected:original});assert.equal(gone.pid,original.pid);assert.equal(gone.startToken,original.startToken);assert.equal(gone.alive,false);assert.equal(gone.pidAbsent,true,'PID reuse or inaccessible process is not positive absence');
 await record('exact-original-process-death-independently-observed');
 const lost=await mutate('reconnect');assert.equal(current.state,'browser_lost');assert.equal(current.lossPolicy,'report_only');assert.deepEqual(desktopKey(lost.host.desktop),desktopKey(desktop));
 await inspect();assert.deepEqual(await observePeers(),peersBefore);await record('report-only-loss-does-not-replace-or-move-browser');
 await mutate('configure',{lossPolicy:'reopen'});assert.equal(current.lossPolicy,'reopen');
 const planned=await mutate('rebind',{desktop:desktopKey(desktop)});assert.equal(current.state,'browser_lost');assert.deepEqual(desktopKey(current.recoveryDesktop),desktopKey(desktop));assert.deepEqual(desktopKey(planned.host.desktop),desktopKey(desktop));
 await record('explicit-ready-recovery-destination-is-intent-only');
 const reopened=await mutate('reopen');assert.equal(current.state,'connected');assert.equal(current.pending,false);assert.equal(current.recoveryDesktop,null);assert.deepEqual(desktopKey(reopened.host.desktop),desktopKey(desktop));
 const after=await observeProcess({task,expected:null});const replacement=identity(after);assert.equal(after.alive,true);assert.notEqual(replacement.startToken,original.startToken);assert.equal(replacement.uid,original.uid);assert.equal(replacement.executablePath,original.executablePath);assert.equal(replacement.profilePath,original.profilePath);
 await inspect();
 const page=await read(`({url:location.href,oldMarker:typeof globalThis.${marker},body:document.body?.textContent??''})`);
 assert.equal(page.url,'about:blank');assert.equal(page.oldMarker,'undefined');assert.equal(page.body,'','Replacement starts blank without replaying the old page/history');
 assert.deepEqual(await observePeers(),peersBefore);await record('explicit-reopen-retains-logical-profile-with-new-process-and-blank-page',{replacement});
 receipt.passed=true;await save('loss-acceptance.json',receipt);return receipt;
}
