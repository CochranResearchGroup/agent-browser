import assert from 'node:assert/strict';
import {mkdtemp,mkdir,writeFile,access,rm,chmod} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {connectHostedAuthHelper} from '../packages/client/src/hosted-auth-helper.js';

// Provider-free installation boundary. The opt-in standalone bootstrap and owner
// compositions separately qualify actual Auth SDK/broker/authority source.
const root=await mkdtemp(join(tmpdir(),'auth-client-installation-'));
const previous=process.env.CODEX_THREAD_ID;process.env.CODEX_THREAD_ID='synthetic-caller';
try{
 for(const scenario of ['verified','wrong-producer','unsafe','unsafe-verifier','missing-export']){
  const candidate=join(root,scenario),marker=join(candidate,'imported');
  await mkdir(join(candidate,'scripts'),{recursive:true});await mkdir(join(candidate,'src'));
  const metadata={product:scenario==='wrong-producer'?'codex-bots':'auth-receipts',contractVersion:1,identity:'synthetic-identity',sourceRevision:'synthetic-source'};
  await writeFile(join(candidate,'scripts/verify-installation-candidate.mjs'),scenario==='unsafe'?`export async function verifyCandidate(){throw Object.assign(Error('invalid'),{code:'auth_sdk_invalid'});}`:`export async function verifyCandidate(){return ${JSON.stringify(metadata)};}`);
  await writeFile(join(candidate,'src/auth-client.mjs'),`import {writeFile} from 'node:fs/promises';await writeFile(${JSON.stringify(marker)},'imported');${scenario==='missing-export'?'':`export function createHostedAuthClient({threadId}){return {async call(operation){if(operation==='denied')throw Object.assign(Error('denied'),{code:'auth_not_authorized'});return {operation,caller:threadId()};}};}`}`);
  if(scenario==='unsafe-verifier')await chmod(join(candidate,'scripts/verify-installation-candidate.mjs'),0o666);
  if(scenario==='verified'){
   const client=await connectHostedAuthHelper({installationRoot:candidate});assert.equal(client.installation.product,'auth-receipts');
   assert.deepEqual(await client.helper('inspect',{}),{operation:'inspect',caller:'synthetic-caller'});
   await assert.rejects(client.helper('denied',{}),{code:'auth_not_authorized'});
  }else{
   await assert.rejects(connectHostedAuthHelper({installationRoot:candidate}),{code:['unsafe','unsafe-verifier'].includes(scenario)?'auth_sdk_invalid':'auth_sdk_incompatible'});
   if(scenario!=='missing-export')await assert.rejects(access(marker),{code:'ENOENT'},'Refused producer cannot execute SDK');
  }
 }
 console.log('Standalone SDK loader verifies producer before SDK import and preserves real authorization errors');
}finally{if(previous===undefined)delete process.env.CODEX_THREAD_ID;else process.env.CODEX_THREAD_ID=previous;await rm(root,{recursive:true,force:true});}
