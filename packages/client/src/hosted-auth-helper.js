// @ts-check
import {realpath,lstat} from 'node:fs/promises';
import {homedir} from 'node:os';
import {join,isAbsolute} from 'node:path';
import {pathToFileURL} from 'node:url';

/** Admit producer code within the owner-only installation trust boundary before
 * executing its verifier. Same-user unrestricted processes are not sandboxed.
 * @param {string} path @param {boolean} directory */
async function ownedInstallationArtifact(path,directory){
  const stat=await lstat(path);
  if(stat.uid!==process.getuid()||stat.mode&0o022||(directory?!stat.isDirectory():!stat.isFile()))throw Object.assign(Error('Auth SDK installation is unsafe; rebuild the owner-controlled candidate'),{code:'auth_sdk_invalid'});
}

/**
 * Connect through the installed producer's supported SDK. Bearer discovery,
 * connection rotation and native caller attestation remain producer-owned.
 * Auth publishes the verified client at its data-directory client/current selector.
 * This Node-only adapter never reads or returns connection-file contents.
 * @param {{installationRoot?:string,discoveryFile?:string}} [options]
 */
export async function connectHostedAuthHelper({installationRoot=join(process.env.XDG_DATA_HOME||join(homedir(),'.local/share'),'auth-receipts/client/current'),discoveryFile}={}) {
  const callerTaskId=process.env.CODEX_THREAD_ID;
  if(!callerTaskId)throw Object.assign(Error('A current Codex caller is required'),{code:'auth_live_task_required'});
  let client,installation,followCredentialLogin;
  try {
    if(!isAbsolute(installationRoot))throw Error('Absolute installation required');
    const app=await realpath(installationRoot);
    await ownedInstallationArtifact(app,true);
    await ownedInstallationArtifact(join(app,'scripts'),true);
    await ownedInstallationArtifact(join(app,'src'),true);
    await ownedInstallationArtifact(join(app,'scripts/verify-installation-candidate.mjs'),false);
    const {verifyCandidate}=await import(pathToFileURL(join(app,'scripts/verify-installation-candidate.mjs')).href);
    const verified=await verifyCandidate(app);
    if(verified.product!=='auth-receipts'||verified.contractVersion!==1)throw Object.assign(Error('Install a compatible standalone Auth Receipts client SDK'),{code:'auth_sdk_incompatible'});
    const sdk=await import(pathToFileURL(join(app,'src/auth-client.mjs')).href);
    const {createHostedAuthClient}=sdk;followCredentialLogin=sdk.followCredentialLogin;
    if(typeof createHostedAuthClient!=='function')throw Object.assign(Error('Installed Auth SDK lacks the supported client export'),{code:'auth_sdk_incompatible'});
    client=createHostedAuthClient({discoveryFile,threadId:()=>callerTaskId});
    installation={product:verified.product,contractVersion:verified.contractVersion,identity:verified.identity,sourceRevision:verified.sourceRevision};
  } catch(error) {
    if(['auth_sdk_incompatible','auth_sdk_invalid'].includes(error?.code))throw error;
    throw Object.assign(Error('Standalone Auth Receipts client SDK is unavailable; install its verified client/current candidate before retrying'),{code:'auth_unavailable'});
  }
  const migration=()=>Object.assign(Error('Installed owner-authority login is unavailable'),{code:'auth_authority_migration_required'});
  const requireOwner=async()=>{
    if(typeof followCredentialLogin!=='function')throw migration();
    let capabilities;try{capabilities=await client.call('auth_capabilities',{});}catch(error){if(error.code==='auth_unknown_operation')throw migration();throw error;}
    if(capabilities?.contractVersion!==1||capabilities.credentialOperations!==true||capabilities.sharedOwnerAuthority!==true||capabilities.legacyCredentialEffects!==false)throw migration();
  };
  const invalid=()=>Object.assign(Error('Invalid owner login request'),{code:'auth_invalid_request'});
  const bound=(request,requestId)=>{if(!request||request.requestId!==requestId||request.mode!=='private'||request.operation!=='login')throw invalid();return request;};
  return {
    callerTaskId,installation,
    /** Request only private login under the existing owner policy. @param {{idempotencyKey:string,sourceId:string,recordId:string,origin:string,targetId:string,purpose:string,expiresAt:number}} options */
    async requestLogin(options){
      if(!options||Object.keys(options).some(key=>!['idempotencyKey','sourceId','recordId','origin','targetId','purpose','expiresAt'].includes(key)))throw invalid();
      await requireOwner();return client.call('request_credential',{...options,operation:'login',mode:'private'});
    },
    /** Poll the original approval and submit private use only before an attempt exists. @param {{requestId:string}} options */
    async advanceLogin(options){
      if(!options||Object.keys(options).length!==1||typeof options.requestId!=='string'||!options.requestId)throw invalid();
      await requireOwner();const {requestId}=options;
      let request=bound(await client.call('inspect_credential_request',{requestId}),requestId);
      if(request.state==='pending')request=bound(await client.call('receive_credential_approval',{requestId}),requestId);
      if(request.state==='approved'&&!request.attemptId)request=bound(await client.call('use_credential',{requestId}),requestId);
      return request;
    },
    /** @param {string} operation @param {Record<string,unknown>} args */
    helper:(operation,args)=>client.call(operation,args),
    /**
     * Follow an existing owner-authority private attempt through MFA and identity
     * readback. Never fall back to legacy authentication or replace the client.
     * @param {{requestId:string,intervalMs?:number,maxObservations?:number,signal?:AbortSignal,onUpdate?:(summary:Record<string,unknown>)=>void|Promise<void>}} options
     */
    async followLogin(options){
      if(!options||Object.keys(options).some(key=>!['requestId','intervalMs','maxObservations','signal','onUpdate'].includes(key)))throw Object.assign(Error('Invalid login follow options'),{code:'auth_invalid_request'});
      await requireOwner();
      return followCredentialLogin({...options,client});
    },
  };
}
