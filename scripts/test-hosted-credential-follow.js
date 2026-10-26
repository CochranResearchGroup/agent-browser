import assert from 'node:assert/strict';
import {mkdtemp,mkdir,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {connectHostedAuthHelper} from '../packages/client/src/hosted-auth-helper.js';

// Synthetic installed SDK boundary. Real owner broker/native/Slack acceptance
// remains in Auth; this contract catches adapter fallback and client replacement.
const root=await mkdtemp(join(tmpdir(),'hosted-credential-follow-'));
const previous=process.env.CODEX_THREAD_ID;process.env.CODEX_THREAD_ID='synthetic-caller';
try{
 await mkdir(join(root,'scripts'));await mkdir(join(root,'src'));
 await writeFile(join(root,'scripts/verify-installation-candidate.mjs'),`export async function verifyCandidate(){return {product:'auth-receipts',contractVersion:1,identity:'synthetic-candidate',sourceRevision:'synthetic-source'};}`);
 await writeFile(join(root,'src/auth-client.mjs'),`
 export let owner=true,errorCode;export function setOwner(value){owner=value;}export function setError(value){errorCode=value;}export const operations=[];
 export function createHostedAuthClient(){return {async call(operation,args){operations.push(operation);if(errorCode)throw Object.assign(Error('Synthetic capability failure'),{code:errorCode});if(operation==='auth_capabilities')return {contractVersion:1,credentialOperations:true,sharedOwnerAuthority:owner,legacyCredentialEffects:!owner};return {requestId:args.requestId,operation};}};}
 export async function followCredentialLogin({client,requestId}){return {followState:'completed',request:await client.call('inspect_credential_request',{requestId})};}
 `);
 const connected=await connectHostedAuthHelper({installationRoot:root});
 const result=await connected.followLogin({requestId:'original-request',intervalMs:1,maxObservations:2});
 assert.equal(result.request.requestId,'original-request');assert.equal(result.followState,'completed');
 await assert.rejects(connected.followLogin({requestId:'original-request',client:{call(){throw Error('Client override');}}}),{code:'auth_invalid_request'});
 const sdk=await import(pathToFileURL(join(root,'src/auth-client.mjs')));sdk.setOwner(false);const beforeCalls=sdk.operations.length;
 await assert.rejects(connected.followLogin({requestId:'original-request'}),{code:'auth_authority_migration_required'});assert.deepEqual(sdk.operations.slice(beforeCalls),['auth_capabilities']);
 sdk.setOwner(true);sdk.setError('auth_not_authorized');await assert.rejects(connected.followLogin({requestId:'original-request'}),{code:'auth_not_authorized'});sdk.setError('auth_unknown_operation');await assert.rejects(connected.followLogin({requestId:'original-request'}),{code:'auth_authority_migration_required'});
 const legacy=join(root,'legacy');await mkdir(join(legacy,'scripts'),{recursive:true});await mkdir(join(legacy,'src'));
 await writeFile(join(legacy,'scripts/verify-installation-candidate.mjs'),`export async function verifyCandidate(){return {product:'auth-receipts',contractVersion:1,identity:'synthetic-old',sourceRevision:'synthetic-old'};}`);
 await writeFile(join(legacy,'src/auth-client.mjs'),`export function createHostedAuthClient(){return {async call(){throw Error('No legacy effect permitted');}};}`);
 const old=await connectHostedAuthHelper({installationRoot:legacy});await assert.rejects(old.followLogin({requestId:'original-request'}),{code:'auth_authority_migration_required'});
 await assert.rejects(old.requestLogin({idempotencyKey:'original-login',sourceId:'synthetic',recordId:'synthetic:1',origin:'https://example.test',targetId:'browser',purpose:'Synthetic',expiresAt:1000}),{code:'auth_authority_migration_required'});
 await assert.rejects(old.advanceLogin({requestId:'original-request'}),{code:'auth_authority_migration_required'});
 console.log('Hosted credential follow delegates to supported SDK and refuses caller transport override');
}finally{
 if(previous===undefined)delete process.env.CODEX_THREAD_ID;else process.env.CODEX_THREAD_ID=previous;
 await rm(root,{recursive:true,force:true});
}
