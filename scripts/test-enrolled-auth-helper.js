import assert from 'node:assert/strict';
import {mkdtemp,mkdir,writeFile,rm,chmod,access} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
import * as helpers from '../packages/client/src/hosted-auth-helper.js';

const root=await mkdtemp(join(tmpdir(),'enrolled-auth-helper-'));
const previous=process.env.CODEX_THREAD_ID;delete process.env.CODEX_THREAD_ID;
const operationContext={operationId:'d03cd7bc-95af-4601-a171-de46a78ea3f1',recoveryId:'932b97f6-2836-4d59-89d4-8db425a67c4d',binding:{taskId:'task',browserId:'browser',profileId:'profile',tabId:'tab',origin:'https://example.test',controlGeneration:'generation-1'}};
try{
 assert.equal(typeof helpers.connectEnrolledAuthHelper,'function','An unattended consumer needs an explicit enrolled helper, not a Codex caller');
 const candidate=join(root,'verified'),discoveryFile=join(root,'consumer-discovery.json');await mkdir(join(candidate,'scripts'),{recursive:true});await mkdir(join(candidate,'src'));
 await writeFile(join(candidate,'scripts/verify-installation-candidate.mjs'),`export async function verifyCandidate(){return{product:'auth-receipts',contractVersion:1,identity:'synthetic-enrolled-sdk',sourceRevision:'synthetic-source'};}`);
 await writeFile(join(candidate,'src/auth-client.mjs'),`
 export const creations=[],calls=[];let owner=true,capabilityError;export function setOwner(value){owner=value;}export function setCapabilityError(value){capabilityError=value;}
 export async function createEnrolledAuthClient(options){creations.push(structuredClone(options));if(options.discoveryFile.endsWith('wrong.json'))throw Object.assign(Error('wrong discovery'),{code:'auth_unavailable'});if(options.operationContext.operationId==='invalid')throw Object.assign(Error('bad context'),{code:'auth_invalid_request'});return{callerTaskId:'consumer:fixture:operation',async call(operation,args){calls.push({operation,args});if(operation==='auth_capabilities'){if(capabilityError)throw Object.assign(Error('policy denied'),{code:capabilityError});return{contractVersion:1,credentialOperations:true,sharedOwnerAuthority:owner,legacyCredentialEffects:!owner};}return{requestId:args.requestId??'fixture-request',operation:'login',mode:'private',state:'unknown',attemptId:'retained-attempt'};}};}
 export function createHostedAuthClient(){throw Error('Codex fallback is forbidden');}
 export async function followCredentialLogin({client,requestId}){return{followState:'completed',request:await client.call('inspect_credential_request',{requestId})};}
 `);
 const connected=await helpers.connectEnrolledAuthHelper({installationRoot:candidate,discoveryFile,operationContext});
 assert.equal(connected.callerTaskId,'consumer:fixture:operation');assert.equal(connected.installation.identity,'synthetic-enrolled-sdk');
 const sdk=await import(pathToFileURL(join(candidate,'src/auth-client.mjs')).href);assert.deepEqual(sdk.creations,[{discoveryFile,operationContext}]);
 const request=await connected.advanceLogin({requestId:'original-private-request'});assert.equal(request.attemptId,'retained-attempt');assert.deepEqual(sdk.calls.map(c=>c.operation),['auth_capabilities','inspect_credential_request']);
 assert.equal((await connected.followLogin({requestId:'original-private-request'})).request.requestId,'original-private-request');
 // Metadata/helper policy stays with Auth; private effects require current owner capabilities.
 const requestOptions={idempotencyKey:'login-once',sourceId:'fixture-source',recordId:'fixture-record',origin:'https://example.test',targetId:'task',purpose:'Synthetic enrolled helper qualification',expiresAt:1800000000000};
 await connected.requestLogin(requestOptions);assert.deepEqual(sdk.calls.at(-1),{operation:'request_credential',args:{...requestOptions,operation:'login',mode:'private'}});
 const before=sdk.calls.length;sdk.setOwner(false);await assert.rejects(connected.requestLogin(requestOptions),{code:'auth_authority_migration_required'});await assert.rejects(connected.advanceLogin({requestId:'original-private-request'}),{code:'auth_authority_migration_required'});await assert.rejects(connected.followLogin({requestId:'original-private-request'}),{code:'auth_authority_migration_required'});assert.deepEqual(sdk.calls.slice(before).map(c=>c.operation),['auth_capabilities','auth_capabilities','auth_capabilities']);
 sdk.setOwner(true);sdk.setCapabilityError('auth_not_authorized');await assert.rejects(connected.advanceLogin({requestId:'original-private-request'}),{code:'auth_not_authorized'});sdk.setCapabilityError('auth_unknown_operation');await assert.rejects(connected.followLogin({requestId:'original-private-request'}),{code:'auth_authority_migration_required'});sdk.setCapabilityError(undefined);
 await assert.rejects(connected.followLogin({requestId:'original-private-request',client:{call(){throw Error('Caller replacement');}}}),{code:'auth_invalid_request'});
 // Ambient Codex identity cannot select a different client or rescue invalid enrolled discovery/context.
 process.env.CODEX_THREAD_ID='not-an-enrolled-caller';
 await assert.rejects(helpers.connectEnrolledAuthHelper({installationRoot:candidate,discoveryFile:join(root,'wrong.json'),operationContext}),{code:'auth_unavailable'});
 await assert.rejects(helpers.connectEnrolledAuthHelper({installationRoot:candidate,discoveryFile,operationContext:{...operationContext,operationId:'invalid'}}),{code:'auth_invalid_request'});
 await assert.rejects(helpers.connectEnrolledAuthHelper({installationRoot:candidate,operationContext}),{code:'auth_invalid_request'});
 for(const scenario of ['wrong-producer','unsafe-verifier','missing-export','missing-follow','incompatible-client']){
  const installationRoot=join(root,scenario),marker=join(installationRoot,'imported');await mkdir(join(installationRoot,'scripts'),{recursive:true});await mkdir(join(installationRoot,'src'));
  await writeFile(join(installationRoot,'scripts/verify-installation-candidate.mjs'),`export async function verifyCandidate(){return{product:'${scenario==='wrong-producer'?'other-product':'auth-receipts'}',contractVersion:1,identity:'synthetic',sourceRevision:'synthetic'};}`);
  await writeFile(join(installationRoot,'src/auth-client.mjs'),`import {writeFile} from 'node:fs/promises';await writeFile(${JSON.stringify(marker)},'imported');export function createHostedAuthClient(){throw Error('Hosted fallback forbidden');}${scenario==='missing-export'?'':`export async function createEnrolledAuthClient(){return${scenario==='incompatible-client'?"{}":"{callerTaskId:'consumer:fixture',async call(){return{contractVersion:1,credentialOperations:true,sharedOwnerAuthority:true,legacyCredentialEffects:false};}}"};}`}${scenario==='missing-follow'?'':`export async function followCredentialLogin(){return{followState:'completed'};}`}`);
  if(scenario==='unsafe-verifier')await chmod(join(installationRoot,'scripts/verify-installation-candidate.mjs'),0o666);
  if(scenario==='missing-follow'){const incomplete=await helpers.connectEnrolledAuthHelper({installationRoot,discoveryFile,operationContext});await assert.rejects(incomplete.requestLogin(requestOptions),{code:'auth_authority_migration_required'});}
  else{await assert.rejects(helpers.connectEnrolledAuthHelper({installationRoot,discoveryFile,operationContext}),{code:scenario==='unsafe-verifier'?'auth_sdk_invalid':'auth_sdk_incompatible'});if(['wrong-producer','unsafe-verifier'].includes(scenario))await assert.rejects(access(marker),{code:'ENOENT'});}
 }
 console.log('Enrolled helper: exact no-Codex SDK factory, verified installation/refusals, owner policy and unknown-attempt non-replay pass');
}finally{if(previous===undefined)delete process.env.CODEX_THREAD_ID;else process.env.CODEX_THREAD_ID=previous;await rm(root,{recursive:true,force:true});}
