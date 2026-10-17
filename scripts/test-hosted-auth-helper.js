import assert from 'node:assert/strict';
import {mkdtemp,rm,realpath} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {connectHostedAuthHelper} from '../packages/client/src/hosted-auth-helper.js';

// Real installed producer/broker public seam, isolated host/native boundary.
const installationRoot=await realpath(join(process.env.HOME,'.local/share/codex-bots/current'));
const {createBotService}=await import(pathToFileURL(join(installationRoot,'src/service.mjs')));
const root=await mkdtemp(join(tmpdir(),'p245-hosted-client-'));
const before=process.env.CODEX_THREAD_ID,callerTaskId='733ad34f-2a54-43b6-8175-cac24d5151f4';
process.env.CODEX_THREAD_ID=callerTaskId;
const target={provider:'agent-browser',taskId:'receiving-work',profileId:'profile',browserId:'browser',tabId:'tab'};
const scope={person:'operator',tenant:'owned',account:'dummy',origin:'https://fixture.invalid',source:'opaque-source',capabilities:['login']};
let service,sourceReads=0;
try {
 service=await createBotService({bots:[],stateDir:join(root,'state'),native:{async status(thread){return {connected:thread===callerTaskId,turnId:'live-turn'};}},
  authConfig:{root:join(root,'auth'),approvers:['operator'],accounts:[{...scope,callers:['codex'],approvers:['operator']}],store:{async read(){sourceReads++;throw Error('No source read permitted');}},
   external:{async verifyReceiving(e){return e.callerTaskId===callerTaskId&&JSON.stringify(e.receivingTarget)===JSON.stringify(target);},async createReceiver(){throw Error('No private receiving permitted');}}}});
 const connected=await connectHostedAuthHelper({installationRoot,discoveryFile:service.authExternalDiscovery});
 assert.equal(connected.callerTaskId,callerTaskId);assert.equal(connected.installation.identity.length,64);
 const request=await connected.helper('request_login',{...scope,expiresAt:Date.now()+60000,contractVersion:2,receivingTarget:target});
 assert.equal(request.taskId,callerTaskId);assert.notEqual(request.taskId,target.taskId);assert.deepEqual(request.receivingTarget,target);assert.equal(request.state,'pending');
 await service.close();service=undefined;
 const missing=await connectHostedAuthHelper({installationRoot,discoveryFile:join(root,'absent.json')});
 await assert.rejects(missing.helper('inspect_request',{requestId:request.requestId}),{code:'auth_unavailable'});
 assert.equal(sourceReads,0);
 console.log('Installed producer SDK discovery and distinct native caller/receiving request passed without exposing bearer');
} finally {
 await service?.close();await rm(root,{recursive:true,force:true});
 if(before===undefined)delete process.env.CODEX_THREAD_ID;else process.env.CODEX_THREAD_ID=before;
}
