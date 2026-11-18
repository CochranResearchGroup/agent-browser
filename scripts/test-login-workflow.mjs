import assert from 'node:assert/strict';
import {test} from 'node:test';
import {mkdtemp,mkdir,writeFile,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {connectHostedAuthHelper} from '../packages/client/src/hosted-auth-helper.js';

test('public helper with a synthetic SDK discovers, preserves uncertainty, resumes and verifies access',async()=>{
  const root=await mkdtemp(join(tmpdir(),'login-composition-'));
  const previous=process.env.CODEX_THREAD_ID;process.env.CODEX_THREAD_ID='synthetic-caller';
  try {
    await mkdir(join(root,'scripts'),{mode:0o700});await mkdir(join(root,'src'),{mode:0o700});
    const statePath=join(root,'synthetic-server.json'),callsPath=join(root,'calls.jsonl');
    const save=value=>writeFile(statePath,JSON.stringify(value),{mode:0o600});
    const binding={taskId:'browser-task',profileId:'profile',browserId:'browser',tabId:'tab',targetId:'physical-target',origin:'https://provider.example'};
    const request={requestId:'request-1',attemptId:'attempt-1',taskId:'synthetic-caller',clientId:'client-1',targetId:binding.taskId,origin:binding.origin,operation:'login',mode:'private',state:'unknown',running:false};
    const completion={requestId:'request-1',state:'completed',completion:{outcome:'succeeded',returnId:'completion-1',evidenceAuthority:'caller_observed',actor:{taskId:'synthetic-caller',clientId:'client-1'},observation:{applicationOrigin:'https://app.example',accountEvidence:'account-proof',evidenceRefs:['application-proof']}},receivingSettlement:{status:'settled',privacy:'retained'}};
    await save({request,completion});
    await writeFile(join(root,'scripts/verify-installation-candidate.mjs'),"export async function verifyCandidate(){return {product:'auth-receipts',contractVersion:1,identity:'synthetic-installation',sourceRevision:'synthetic-source'}}",{mode:0o600});
    await writeFile(join(root,'src/auth-client.mjs'),`
import {readFile,appendFile} from 'node:fs/promises';
export function createHostedAuthClient(){return {async call(operation,args){
 await appendFile(${JSON.stringify(callsPath)},JSON.stringify({operation,args})+'\\n',{mode:0o600});
 const state=JSON.parse(await readFile(${JSON.stringify(statePath)},'utf8'));
 if(operation==='inspect_credential_login')return {loginId:args.loginId,state:'owner_review'};
 if(operation==='auth_capabilities')return {contractVersion:1,credentialOperations:true,sharedOwnerAuthority:true,legacyCredentialEffects:false};
 if(operation==='discover_credentials'){const selected={sourceId:'lastpass',recordId:'record-1',origin:'https://provider.example',account:'configured@example.test'};return {state:'unique',sources:[{sourceId:'lastpass',capabilities:{}}],unavailable:[],selected,candidates:[selected]};}
 if(operation==='inspect_credential_request'||operation==='reconcile_credential_request')return state.request;
 if(operation==='inspect_authentication_completion')return state.completion;
 throw Error('Unexpected private effect');
}}}
`,{mode:0o600});
    const helper=await connectHostedAuthHelper({installationRoot:root});
    assert.equal((await helper.inspectLoginProgress({loginId:'login-1'})).waitingForUser,false);
    assert.equal((await helper.discoverLoginReadiness({origin:binding.origin,accountHint:'configured@example.test',extensionState:'signed_out'})).category,'credential_ready');
    let record={episodeId:'episode-1',taskId:binding.taskId,inputs:[{requestId:'native-original',state:'uncertain'}],loginContinuation:{schemaVersion:1,workflowId:'workflow-1',requestId:request.requestId,attemptId:request.attemptId,binding,requestedOutcome:{applicationOrigin:'https://app.example',expectedAccount:'configured@example.test',expectedOrganization:'Example Org'},unresolvedAttempts:[]}};
    const store={async withEpisode(_id,run){return run();},async load(){return structuredClone(record);},async save(value){record=structuredClone(value);}};
    let observed=false;const updates=[];
    const options={episodeId:'episode-1',store,verifyAccess:async()=>({...binding,state:observed?'verified':'unverified',accountMatched:observed,organizationMatched:observed,applicationOrigin:'https://app.example',evidenceRef:'application-proof'}),onUpdate:async update=>{updates.push(update);}};
    assert.equal((await helper.resumeLoginContinuation(options)).category,'input_outcome_unknown');
    const resumed=await connectHostedAuthHelper({installationRoot:root});
    assert.equal((await resumed.resumeLoginContinuation(options)).notification.emitted,false);
    observed=true;await save({request:{...request,state:'completed'},completion});
    const result=await resumed.resumeLoginContinuation(options);
    assert.equal(result.outcome,'verified');assert.equal(updates.length,2);
    assert.equal(record.inputs[0].state,'uncertain');
    assert.equal(result.unresolvedAttempts[0].attemptId,'attempt-1');
    const calls=(await readFile(callsPath,'utf8')).trim().split('\n').map(line=>JSON.parse(line));
    assert(calls.every(call=>!['use_credential','request_credential','prepare_credential_login'].includes(call.operation)));
    assert(!JSON.stringify(result).includes('configured@example.test'));
  } finally {
    if(previous===undefined)delete process.env.CODEX_THREAD_ID;else process.env.CODEX_THREAD_ID=previous;
    await rm(root,{recursive:true,force:true});
  }
});
