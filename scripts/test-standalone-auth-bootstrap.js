import assert from 'node:assert/strict';
import {mkdtemp,mkdir,symlink,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';

const source=process.env.AUTH_RECEIPTS_SOURCE;
assert.ok(source,'Set AUTH_RECEIPTS_SOURCE to reviewed Auth source');
const {publishClientSdk}=await import(pathToFileURL(join(source,'src/client-sdk.mjs')));
const {startStandalone}=await import(pathToFileURL(join(source,'src/standalone.mjs')));
const {openCredentialService}=await import(pathToFileURL(join(source,'src/credential-service.mjs')));
const root=await mkdtemp(join(tmpdir(),'browser-auth-bootstrap-'));let host;
try{
 const home=join(root,'home'),data=join(home,'.local/share/auth-receipts/client');await mkdir(data,{recursive:true});
 const candidate=join(root,'sdk');await publishClientSdk({destination:candidate});await symlink(candidate,join(data,'current'));
 host=await startStandalone({tenantOptions:{stateHome:join(root,'ipc')},auth:{principals:[{id:'caller',role:'caller'}],resolveCaller:async()=>({taskId:'synthetic-thread'}),credentialServiceFactory:context=>openCredentialService({...context,root:join(root,'authority'),now:Date.now,sources:[]})}});
 // Keep connection and discovery in the same private directory, as Auth requires.
 const {dirname}=await import('node:path');const file=join(dirname(host.connections[0]),'external-discovery.json');
 await writeFile(file,JSON.stringify({contractVersion:2,transport:'codex-bots-auth',receivingProvider:'agent-browser',connectionFile:host.connections[0]}),{mode:0o600});
 const module=pathToFileURL(resolve('packages/client/src/hosted-auth-helper.js')).href;
 const child=await promisify(execFile)(process.execPath,['--input-type=module','-e',`import {connectHostedAuthHelper} from ${JSON.stringify(module)};const x=await connectHostedAuthHelper();if(x.installation.product!=='auth-receipts')throw Error('Wrong producer');const c=await x.helper('auth_capabilities',{});if(c.sharedOwnerAuthority!==true)throw Error('Wrong authority');console.log('verified standalone');`],{env:{...process.env,HOME:home,XDG_DATA_HOME:join(home,'.local/share'),CODEX_THREAD_ID:'synthetic-thread',AUTH_RECEIPTS_DISCOVERY_FILE:file},encoding:'utf8'});
 assert.match(child.stdout,/verified standalone/);
 console.log('Default standalone Auth SDK discovery succeeds with Bots absent');
}finally{await host?.close();await rm(root,{recursive:true,force:true});}
