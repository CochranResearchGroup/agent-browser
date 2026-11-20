import {withClientErrorRecovery} from './client-recovery.js';
import { callClientOwner } from './client-recovery.js';
// @ts-check
import {realpath,lstat} from 'node:fs/promises';
import {homedir} from 'node:os';
import {join,isAbsolute} from 'node:path';
import {pathToFileURL} from 'node:url';
import {summarizeLoginProgress} from './login-progress.js';
import {createLoginRecovery} from './login-recovery.js';
import {createLoginVerifier} from './login-verification.js';
import {createLoginDiscovery} from './login-discovery.js';
import {createLoginContinuation} from './login-continuation.js';
export {createLoginDiscovery} from './login-discovery.js';
export {summarizeLoginProgress} from './login-progress.js';

/** Admit producer code within the owner-only installation trust boundary before
 * executing its verifier. Same-user unrestricted processes are not sandboxed.
 * @param {string} path @param {boolean} directory */
async function ownedInstallationArtifact(path,directory){
  const stat=await lstat(path);
  if(stat.uid!==process.getuid()||stat.mode&0o022||(directory?!stat.isDirectory():!stat.isFile()))throw withClientErrorRecovery(Object.assign(Error('Auth SDK installation is unsafe; rebuild the owner-controlled candidate'),{code:'auth_sdk_invalid'}),{source:"packages/client/src/hosted-auth-helper.js",owningFunction:"ownedInstallationArtifact"},'auth_sdk_invalid');
}

const defaultInstallationRoot=()=>join(process.env.XDG_DATA_HOME||join(homedir(),'.local/share'),'auth-receipts/client/current');

/** Shared owner-only installation admission; private discovery stays in Auth's SDK.
 * @param {string} installationRoot */
async function installedAuthSdk(installationRoot){
  if(!isAbsolute(installationRoot))throw Error('Absolute installation required');
  const app=await realpath(installationRoot);
  await ownedInstallationArtifact(app,true);
  await ownedInstallationArtifact(join(app,'scripts'),true);
  await ownedInstallationArtifact(join(app,'src'),true);
  await ownedInstallationArtifact(join(app,'scripts/verify-installation-candidate.mjs'),false);
  const {verifyCandidate}=await import(pathToFileURL(join(app,'scripts/verify-installation-candidate.mjs')).href);
  const verified=await verifyCandidate(app);
  if(verified.product!=='auth-receipts'||verified.contractVersion!==1)throw withClientErrorRecovery(Object.assign(Error('Install a compatible standalone Auth Receipts client SDK'),{code:'auth_sdk_incompatible'}),{source:"packages/client/src/hosted-auth-helper.js",owningFunction:"installedAuthSdk"},'auth_sdk_incompatible');
  const sdk=await import(pathToFileURL(join(app,'src/auth-client.mjs')).href);
  const installation={product:verified.product,contractVersion:verified.contractVersion,identity:verified.identity,sourceRevision:verified.sourceRevision};
  return{sdk,installation};
}
function installationFailure(error){
  // A verified producer's typed context/policy refusal is not proof that its
  // SDK installation is missing. Preserve its complete owner recovery object.
  if(typeof error?.code==='string'&&/^auth_[a-z0-9_]{1,122}$/.test(error.code))throw error;
  throw withClientErrorRecovery(Object.assign(Error('Standalone Auth Receipts client SDK is unavailable; install its verified client/current candidate before retrying'),{code:'auth_unavailable'}),{source:"packages/client/src/hosted-auth-helper.js",owningFunction:"installationFailure"},'auth_unavailable');
}

/** Connect for the genuine current Codex caller; enrolled consumers use the
 * distinct entry point below. Connection contents never enter this adapter.
 * @param {{installationRoot?:string,discoveryFile?:string}} [options] */
export async function connectHostedAuthHelper({installationRoot=defaultInstallationRoot(),discoveryFile}={}){
  const callerTaskId=process.env.CODEX_THREAD_ID;
  if(!callerTaskId)throw withClientErrorRecovery(Object.assign(Error('A current Codex caller is required'),{code:'auth_live_task_required'}),{source:"packages/client/src/hosted-auth-helper.js",owningFunction:"connectHostedAuthHelper"},'auth_live_task_required');
  try{
    const {sdk,installation}=await installedAuthSdk(installationRoot);
    if(typeof sdk.createHostedAuthClient!=='function')throw withClientErrorRecovery(Object.assign(Error('Installed Auth SDK lacks the supported client export'),{code:'auth_sdk_incompatible'}),{source:"packages/client/src/hosted-auth-helper.js",owningFunction:"connectHostedAuthHelper"},'auth_sdk_incompatible');
    const client=sdk.createHostedAuthClient({discoveryFile,threadId:()=>callerTaskId});
    return ownerAuthHelper(client,installation,callerTaskId,sdk.followCredentialLogin,sdk.discoverLoginChannels);
  }catch(error){return installationFailure(error);}
}

/** @typedef {{operationId:string,recoveryId:string,binding:{taskId:string,browserId:string,profileId:string,tabId:string,origin:string,controlGeneration:string}}} AuthOperationContext */
/** Connect an explicitly enrolled unattended consumer through the verified Auth
 * producer factory. Auth owns discovery/identity/context validation and private
 * transport. No CODEX_THREAD_ID lookup or hosted/discovery fallback occurs here.
 * @param {{installationRoot?:string,discoveryFile:string,operationContext:AuthOperationContext}} options */
export async function connectEnrolledAuthHelper(options){
  if(!options||Object.keys(options).some(key=>!['installationRoot','discoveryFile','operationContext'].includes(key))||typeof options.discoveryFile!=='string'||!isAbsolute(options.discoveryFile)||!options.operationContext||typeof options.operationContext!=='object'||Array.isArray(options.operationContext))throw withClientErrorRecovery(Object.assign(Error('Invalid enrolled Auth helper request'),{code:'auth_invalid_request'}),{source:"packages/client/src/hosted-auth-helper.js",owningFunction:"connectEnrolledAuthHelper"},'auth_invalid_request');
  const {installationRoot=defaultInstallationRoot(),discoveryFile,operationContext}=options;
  try{
    const {sdk,installation}=await installedAuthSdk(installationRoot);
    if(typeof sdk.createEnrolledAuthClient!=='function')throw withClientErrorRecovery(Object.assign(Error('Installed Auth SDK lacks enrolled consumer support'),{code:'auth_sdk_incompatible'}),{source:"packages/client/src/hosted-auth-helper.js",owningFunction:"connectEnrolledAuthHelper"},'auth_sdk_incompatible');
    const client=await sdk.createEnrolledAuthClient({discoveryFile,operationContext});
    if(typeof client?.call!=='function'||typeof client.callerTaskId!=='string'||!client.callerTaskId)throw withClientErrorRecovery(Object.assign(Error('Installed Auth SDK returned an incompatible enrolled client'),{code:'auth_sdk_incompatible'}),{source:"packages/client/src/hosted-auth-helper.js",owningFunction:"connectEnrolledAuthHelper"},'auth_sdk_incompatible');
    return ownerAuthHelper(client,installation,client.callerTaskId,sdk.followCredentialLogin,sdk.discoverLoginChannels);
  }catch(error){return installationFailure(error);}
}

/** Keep private login policy and uncertain-attempt handling identical for both
 * genuine Codex callers and explicitly enrolled service consumers. */
function ownerAuthHelper(client,installation,callerTaskId,followCredentialLogin,discoverLoginChannels){
  const migration=()=>withClientErrorRecovery(Object.assign(Error('Installed owner-authority login is unavailable'),{code:'auth_authority_migration_required'}),{source:"packages/client/src/hosted-auth-helper.js",owningFunction:"migration"},'auth_authority_migration_required');
  const requireOwner=async()=>{
    if(typeof followCredentialLogin!=='function')throw migration();
    let capabilities;try{capabilities=await callClientOwner(client,'auth_capabilities',{},{source:"packages/client/src/hosted-auth-helper.js",owningFunction:"ownerAuthHelper"});}catch(error){if(error.code==='auth_unknown_operation')throw migration();throw error;}
    if(capabilities?.contractVersion!==1||capabilities.credentialOperations!==true||capabilities.sharedOwnerAuthority!==true||capabilities.legacyCredentialEffects!==false)throw migration();
  };
  const invalid=()=>withClientErrorRecovery(Object.assign(Error('Invalid owner login request'),{code:'auth_invalid_request'}),{source:"packages/client/src/hosted-auth-helper.js",owningFunction:"invalid"},'auth_invalid_request');
  const bound=(request,requestId)=>{if(!request||request.requestId!==requestId||request.mode!=='private'||request.operation!=='login')throw invalid();return request;};
  return {
    callerTaskId,installation,
    /** Existing owner recovery operations; no private credential use or replay. */
    loginRecovery:createLoginRecovery({client}),
    /** Read source readiness through Auth; extension state is separate and
     * an unavailable permission view never establishes absent permission.
     * @param {import('./login-discovery.d.ts').LoginDiscoveryIntent} intent */
    async discoverLoginReadiness(intent){return createLoginDiscovery({client}).inspect(intent);},
    /** Resume only an existing private continuation namespace. Update and Wake
     * callbacks report observations; neither creates credential authority.
     * @param {{episodeId:string,store:import('./login-continuation.d.ts').LoginContinuationOptions['store'],verifyAccess:import('./login-verification.d.ts').LoginVerifierOptions['verifyAccess'],readWake?:import('./login-continuation.d.ts').LoginContinuationOptions['readWake'],onUpdate?:import('./login-continuation.d.ts').LoginContinuationOptions['onUpdate']}} options */
    async resumeLoginContinuation({episodeId,store,verifyAccess,readWake,onUpdate}){
      const verifier=createLoginVerifier({client,verifyAccess,callerTaskId});
      return createLoginContinuation({store,callerTaskId,inspectRequest:args=>callClientOwner(client,'inspect_credential_request',args,{source:"packages/client/src/hosted-auth-helper.js",owningFunction:"ownerAuthHelper.resumeLoginContinuation"}),verify:input=>verifier.verify(input),readWake,onUpdate}).resume({episodeId});
    },
    /** Verify receiving-application access independently of owner completion.
     * @param {import('./login-verification.d.ts').LoginVerificationInput & {verifyAccess:import('./login-verification.d.ts').LoginVerifierOptions['verifyAccess']}} options */
    async verifyLoginAccess({verifyAccess,...input}){
      return createLoginVerifier({client,verifyAccess,callerTaskId}).verify(input);
    },
    /** Inspect the original Auth login/request and return bounded progress.
     * This read never polls an approval or uses a credential.
     * @param {{loginId?:string,requestId?:string}} options */
    async inspectLoginProgress(options){
      const keys=Object.keys(options??{});
      if(keys.length!==1||!['loginId','requestId'].includes(keys[0])||typeof options[keys[0]]!=='string'||!options[keys[0]])throw invalid();
      const operation=keys[0]==='loginId'?'inspect_credential_login':'inspect_credential_request';
      const observation=await callClientOwner(client,operation,options,{source:"packages/client/src/hosted-auth-helper.js",owningFunction:"ownerAuthHelper.inspectLoginProgress"});
      if(observation?.[keys[0]]!==options[keys[0]])throw invalid();
      return summarizeLoginProgress(observation);
    },
    /** Read-only IM/Mail proposals through the verified Auth SDK. This neither
     * grants authority nor invokes Auth credential use or a private receiver.
     * @param {{tenant:string,smsSuffix?:string,emailHint?:string,im?:{callTool:(request:any)=>Promise<any>},mail?:{callTool:(request:any)=>Promise<any>}}} options */
    async discoverLoginChannels(options){
      if(typeof discoverLoginChannels!=='function')throw withClientErrorRecovery(Object.assign(Error('Installed Auth SDK lacks channel discovery; use auth-receipts://guides/login-orchestration and repair the verified SDK installation'),{code:'auth_channel_discovery_unavailable'}),{source:"packages/client/src/hosted-auth-helper.js",owningFunction:"ownerAuthHelper.discoverLoginChannels"},'auth_channel_discovery_unavailable');
      return discoverLoginChannels(options);
    },
    /** Request only private login under the existing owner policy. @param {{idempotencyKey:string,sourceId:string,recordId:string,origin:string,targetId:string,purpose:string,expiresAt:number}} options */
    async requestLogin(options){
      if(!options||Object.keys(options).some(key=>!['idempotencyKey','sourceId','recordId','origin','targetId','purpose','expiresAt'].includes(key)))throw invalid();
      await requireOwner();return callClientOwner(client,'request_credential',{...options,operation:'login',mode:'private'},{source:"packages/client/src/hosted-auth-helper.js",owningFunction:"ownerAuthHelper.requestLogin"});
    },
    /** Poll the original approval and submit private use only before an attempt exists. @param {{requestId:string}} options */
    async advanceLogin(options){
      if(!options||Object.keys(options).length!==1||typeof options.requestId!=='string'||!options.requestId)throw invalid();
      await requireOwner();const {requestId}=options;
      let request=bound(await callClientOwner(client,'inspect_credential_request',{requestId},{source:"packages/client/src/hosted-auth-helper.js",owningFunction:"ownerAuthHelper.advanceLogin"}),requestId);
      if(request.state==='pending')request=bound(await callClientOwner(client,'receive_credential_approval',{requestId},{source:"packages/client/src/hosted-auth-helper.js",owningFunction:"ownerAuthHelper.advanceLogin"}),requestId);
      if(request.state==='approved'&&!request.attemptId)request=bound(await callClientOwner(client,'use_credential',{requestId},{source:"packages/client/src/hosted-auth-helper.js",owningFunction:"ownerAuthHelper.advanceLogin"}),requestId);
      return request;
    },
    /** @param {string} operation @param {Record<string,unknown>} args */
    helper:(operation,args)=>callClientOwner(client,operation,args,{source:"packages/client/src/hosted-auth-helper.js",owningFunction:"ownerAuthHelper"}),
    /**
     * Follow an existing owner-authority private attempt through MFA and identity
     * readback. Never fall back to legacy authentication or replace the client.
     * @param {{requestId:string,intervalMs?:number,maxObservations?:number,signal?:AbortSignal,onUpdate?:(summary:Record<string,unknown>)=>void|Promise<void>}} options
     */
    async followLogin(options){
      if(!options||Object.keys(options).some(key=>!['requestId','intervalMs','maxObservations','signal','onUpdate'].includes(key)))throw withClientErrorRecovery(Object.assign(Error('Invalid login follow options'),{code:'auth_invalid_request'}),{source:"packages/client/src/hosted-auth-helper.js",owningFunction:"ownerAuthHelper.followLogin"},'auth_invalid_request');
      await requireOwner();
      return followCredentialLogin({...options,client});
    },
  };
}
