/** Public workflow extension for an already allocated synthetic hosted fixture.
 * Importing has no effects. workflow returns unwrapped successful data and throws
 * typed terminal refusals; the caller retains timeout/transport uncertainty and
 * owns processes, assignment, artifact directory and cleanup. Never replay effects.
 */
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {readFile,realpath,stat} from 'node:fs/promises';
import {isAbsolute,join} from 'node:path';

export async function qualifyHostedBrowserPolicy({workflow,task,peerTask,artifactRoot,save}){
 assert(task?.taskId&&peerTask?.taskId&&task.taskId!==peerTask.taskId);
 assert.equal(task.browserId,peerTask.browserId,'Fixture must explicitly share the same app-owned browser');
 assert.notEqual(task.tabId,peerTask.tabId,'Shared tasks retain separate logical tab identities');
 assert(isAbsolute(artifactRoot));assert.equal(await realpath(artifactRoot),artifactRoot);
 assert((await stat(artifactRoot)).isDirectory());
 const receipt={version:1,taskId:task.taskId,peerTaskId:peerTask.taskId,checks:[],publication:'not_requested',cleanup:'caller_owned'};
 const record=async(name,extra={})=>{receipt.checks.push({name,...extra});await save('policy-acceptance.json',receipt);};
 const call=(name,args)=>workflow(name,args);
 const page=async id=>(await call('browser_command',{taskId:id,action:'evaluate',params:{script:'({marker:document.body.textContent.includes("ATTACHMENT SYNTHETIC MARKER"),value:document.querySelector("#control")?.value,url:location.href})'}})).result;
 const before=await page(task.taskId);assert.equal(before.marker,true,'Only the prepared synthetic page may be modified');assert.equal(typeof before.value,'string');
 const peerBefore=await page(peerTask.taskId);assert.equal(peerBefore.url,before.url);
 async function refused(name,invoke,code){
  let failure;try{const result=await invoke();if(result?.success===false)failure=result.error;}catch(error){failure=error;}
  const text=[failure?.code,failure?.message].filter(Boolean).join(':');
  assert(text.includes(code),`${name} must return ${code}; unclassified/transport failure cannot qualify refusal`);
  await record(name,{code});
 }
 const current=await call('browser_policy',{taskId:task.taskId,operation:'status'});assert(Number.isSafeInteger(current.revision));
 const configured=await call('browser_policy',{taskId:task.taskId,operation:'set_task',expectedRevision:current.revision,actor:'synthetic-operator',reason:'Qualify denied fill with authorized local evidence',policy:{version:1,deniedActions:['fill'],artifactDestinations:{evidence:artifactRoot}}});
 assert(configured.revision>current.revision);await record('task-policy-configured');
 await refused('public-fill-denied',()=>call('browser_fill',{taskId:task.taskId,selector:'#control',value:'MUST NOT APPEAR'}),'browser_policy_action_denied');
 assert.equal((await page(task.taskId)).value,before.value,'Refused fill must have no page effect');
 const peerValue='peer-'+randomUUID();await call('browser_fill',{taskId:peerTask.taskId,selector:'#control',value:peerValue});assert.equal((await page(peerTask.taskId)).value,peerValue);await record('independent-peer-task-unrestricted');
 const requestId='override-fill-'+randomUUID();
 const granted=await call('browser_policy',{taskId:task.taskId,operation:'override',expectedRevision:configured.revision,actor:'synthetic-operator',reason:'Authorize one exact synthetic fill',overrideAction:'fill',requestId});assert.equal(typeof granted.overrideId,'string');
 await refused('override-cannot-retarget-request',()=>call('browser_command',{taskId:task.taskId,action:'fill',requestId:'different-'+randomUUID(),policyOverrideId:granted.overrideId,params:{selector:'#control',value:'WRONG REQUEST'}}),'browser_policy_action_denied');
 assert.equal((await page(task.taskId)).value,peerValue);
 const overrideValue='override-'+randomUUID();await call('browser_command',{taskId:task.taskId,action:'fill',requestId,policyOverrideId:granted.overrideId,params:{selector:'#control',value:overrideValue}});assert.equal((await page(task.taskId)).value,overrideValue);await record('exact-request-override-permits-fill');
 const name='synthetic-'+randomUUID()+'.png';
 const artifact=await call('browser_artifact',{taskId:task.taskId,kind:'screenshot',handle:'evidence',name,requestId:'capture-'+randomUUID()});
 assert.equal(artifact.publication,'not_requested');assert.equal(artifact.handle,'evidence');assert.equal(artifact.name,name);assert.equal(artifact.path,join(artifactRoot,name));assert.equal(typeof artifact.artifactId,'string');
 const bytes=await readFile(artifact.path);assert(bytes.length>8);assert.deepEqual([...bytes.subarray(0,8)],[137,80,78,71,13,10,26,10]);await record('authorized-local-artifact',{bytes:bytes.length,publication:artifact.publication});
 await refused('artifact-traversal-refused',()=>call('browser_artifact',{taskId:task.taskId,kind:'screenshot',handle:'evidence',name:'../outside.png',requestId:'traversal-'+randomUUID()}),'browser_artifact_name_invalid');
 const status=await call('browser_policy',{taskId:task.taskId,operation:'status'});
 await call('browser_policy',{taskId:task.taskId,operation:'clear_task',expectedRevision:status.revision,actor:'synthetic-operator',reason:'Restore fixture app defaults'});
 await call('browser_fill',{taskId:task.taskId,selector:'#control',value:before.value});assert.equal((await page(task.taskId)).value,before.value);await record('clear-task-restores-unrestricted-fill');
 await refused('broader-close-refuses-app-borrowed-browser',()=>call('browser_close',{taskId:task.taskId,scope:'browser',actor:'synthetic-operator',reason:'Verify borrowed process cannot be terminated'}),'browser_lifecycle_borrowed_process_preserved');
 assert.deepEqual(await page(task.taskId),before,'Refused browser close preserves original page and identity');assert.equal((await page(peerTask.taskId)).url,peerBefore.url);await record('original-and-peer-survive-broader-close-refusal');
 receipt.passed=true;await save('policy-acceptance.json',receipt);return receipt;
}
