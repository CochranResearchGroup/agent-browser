// @ts-check
import {realpath} from 'node:fs/promises';
import {homedir} from 'node:os';
import {join,isAbsolute} from 'node:path';
import {pathToFileURL} from 'node:url';

/**
 * Connect through the installed producer's supported SDK. Bearer discovery,
 * connection rotation and native caller attestation remain producer-owned.
 * This Node-only adapter never reads or returns connection-file contents.
 * @param {{installationRoot?:string,discoveryFile?:string}} [options]
 */
export async function connectHostedAuthHelper({installationRoot=join(homedir(),'.local/share/codex-bots/current'),discoveryFile}={}) {
  const callerTaskId=process.env.CODEX_THREAD_ID;
  if(!callerTaskId)throw Object.assign(Error('A current Codex caller is required'),{code:'auth_live_task_required'});
  let client,installation;
  try {
    if(!isAbsolute(installationRoot))throw Error('Absolute installation required');
    const app=await realpath(installationRoot);
    const {verifyCandidate}=await import(pathToFileURL(join(app,'scripts/verify-installation-candidate.mjs')).href);
    const verified=await verifyCandidate(app);
    const {createHostedAuthClient}=await import(pathToFileURL(join(app,'src/auth-client.mjs')).href);
    client=createHostedAuthClient({discoveryFile,threadId:()=>callerTaskId});
    installation={identity:verified.identity,sourceRevision:verified.sourceRevision};
  } catch {
    throw Object.assign(Error('Installed hosted auth helper is unavailable'),{code:'auth_unavailable'});
  }
  return {
    callerTaskId,installation,
    /** @param {string} operation @param {Record<string,unknown>} args */
    helper:(operation,args)=>client.call(operation,args),
  };
}
