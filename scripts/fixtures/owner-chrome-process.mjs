import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {glob} from 'node:fs/promises';
import {join,resolve} from 'node:path';
import {pathToFileURL} from 'node:url';

// A fresh public SDK client reconnects to the original durable Auth request.
export async function resumeOwnerLoginInFreshProcess(f){
 const server=createServer(async(req,res)=>{
  const method=req.url==='/verify'?'verifyLogin':req.url==='/validate'?'validatePlan':'prepareLogin';
  res.setHeader('content-type','application/json');res.end(JSON.stringify(await f.browser[method](f.options)));
 });
 await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
 try{
  const files=[];for await(const file of glob('**/external-discovery.json',{cwd:f.root}))files.push(file);assert.equal(files.length,1);
  const helper=pathToFileURL(resolve('packages/client/src/hosted-auth-helper.js')).href;
  const playbook=pathToFileURL(resolve('packages/client/src/owner-login-playbook.js')).href;
  const base=`http://127.0.0.1:${server.address().port}`;
  const code=`import {readFile,writeFile} from 'node:fs/promises';import {join} from 'node:path';import {connectHostedAuthHelper} from ${JSON.stringify(helper)};import {createOwnerLoginPlaybook} from ${JSON.stringify(playbook)};
  const auth=await connectHostedAuthHelper({installationRoot:process.env.FIXTURE_SDK,discoveryFile:process.env.FIXTURE_DISCOVERY});
  const checkpoints={load:async key=>{try{return JSON.parse(await readFile(join(process.env.FIXTURE_STORE,encodeURIComponent(key)+'.json'),'utf8'));}catch(e){if(e.code==='ENOENT')return null;throw e;}},save:async(key,value)=>writeFile(join(process.env.FIXTURE_STORE,encodeURIComponent(key)+'.json'),JSON.stringify(value),{mode:0o600})};
  const browser=Object.fromEntries([['verifyLogin','verify'],['prepareLogin','prepare'],['validatePlan','validate']].map(([method,route])=>[method,async()=>{const r=await fetch(process.env.FIXTURE_BROWSER+'/'+route);return r.json();}]));
  console.log(JSON.stringify({pid:process.pid,result:await createOwnerLoginPlaybook({browser,auth,checkpoints}).advance(JSON.parse(process.env.FIXTURE_OPTIONS))}));`;
  const child=await promisify(execFile)(process.execPath,['--input-type=module','-e',code],{env:{...process.env,CODEX_THREAD_ID:'synthetic-caller',FIXTURE_SDK:join(f.root,'sdk'),FIXTURE_DISCOVERY:join(f.root,files[0]),FIXTURE_STORE:join(f.root,'checkpoints'),FIXTURE_BROWSER:base,FIXTURE_OPTIONS:JSON.stringify(f.options)},timeout:10000});
  const observed=JSON.parse(child.stdout.trim());assert.notEqual(observed.pid,process.pid,'Recovery must run in a fresh OS process');return observed.result;
 }finally{await new Promise(resolve=>server.close(resolve));}
}

