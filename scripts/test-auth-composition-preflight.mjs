import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import test from 'node:test';
import {mkdtemp,mkdir,writeFile,rm} from 'node:fs/promises';
import {join} from 'node:path';
import {tmpdir} from 'node:os';

const command=fileURLToPath(new URL('./dev/run-owner-auth-source.mjs',import.meta.url));
const inputs=['AUTH_RECEIPTS_SOURCE','AUTH_RECEIPTS_SDK_SOURCE','RECEIPTS_SOURCE'];
const invoke=(args=[],overrides={})=>{const env={...process.env,...overrides};for(const key of inputs)if(!Object.hasOwn(overrides,key))delete env[key];return spawnSync(process.execPath,[command,...args],{env,encoding:'utf8'});};

test('missing explicit roots are reported together before composition setup',()=>{
 const run=invoke();
 assert.equal(run.status,1);
 const result=JSON.parse(run.stderr);
 assert.equal(result.code,'auth_composition_preflight_failed');
 assert.deepEqual(result.problems.map(p=>p.input).sort(),[...inputs].sort());
 assert.ok(result.problems.every(p=>p.reason==='missing_root'));
 assert.equal(run.stdout,'');
});

async function sources(){
 const root=await mkdtemp(join(tmpdir(),'auth-preflight-test-'));
 const service=join(root,'private-service'),sdk=join(root,'private-sdk'),parent=join(root,'private-parent');
 const layouts=[
  [service,['src/standalone.mjs','src/credential-service.mjs','src/receipts-approval.mjs','src/keepass-backend.mjs','src/credential-sources.mjs','src/native-browser-receiver.mjs']],
  [sdk,['src/client-sdk.mjs','src/client-sdk-verification.mjs','src/auth-client.mjs','src/auth-transport.mjs']],
  [parent,['apps/receipts-interoperation/src/request-hub.mjs','apps/receipts-interoperation/src/notifications.mjs']],
 ];
 for(const [path,files]of layouts){
  await mkdir(path,{recursive:true});await writeFile(join(path,'package.json'),JSON.stringify({name:'synthetic-only',dependencies:{}}));
  for(const file of files){await mkdir(join(path,file,'..'),{recursive:true});await writeFile(join(path,file),"throw Error('PREFLIGHT_MUST_NOT_IMPORT_SOURCE');\n");}
  for(const args of [['init','-q'],['add','.'],['-c','user.name=Synthetic','-c','user.email=synthetic@example.test','commit','-qm','synthetic reviewed source']])assert.equal(spawnSync('git',args,{cwd:path}).status,0);
 }
 return {root,service,sdk,parent,env:{AUTH_RECEIPTS_SOURCE:service,AUTH_RECEIPTS_SDK_SOURCE:sdk,RECEIPTS_SOURCE:parent},close:()=>rm(root,{recursive:true,force:true})};
}

test('invalid roots and missing source entrypoints are reported without private path disclosure',async()=>{
 const f=await sources();try{
  await rm(join(f.service,'src/credential-service.mjs'));
  const run=invoke(['--preflight-only'],{...f.env,AUTH_RECEIPTS_SDK_SOURCE:join(f.root,'absent-private-sdk')});
  assert.equal(run.status,1);const result=JSON.parse(run.stderr);
  assert.deepEqual(result.problems.map(p=>p.input).sort(),['AUTH_RECEIPTS_SDK_SOURCE','AUTH_RECEIPTS_SOURCE']);
  assert.equal(JSON.stringify(result).includes(f.root),false);assert.equal(run.stdout,'');
 }finally{await f.close();}
});

test('preflight identifies distinct reviewed roots and actual bytes without importing source',async()=>{
 const f=await sources();try{
  const run=invoke(['--preflight-only'],f.env);assert.equal(run.status,0,run.stderr);
  const result=JSON.parse(run.stdout);assert.equal(result.schemaVersion,'agent-browser.auth-composition-preflight.v1');
  assert.deepEqual(Object.keys(result.sources),['service','sdk','parent']);
  for(const value of Object.values(result.sources)){assert.match(value.revision,/^[a-f0-9]{40}$/);assert.equal(value.dirty,false);assert.match(value.contentDigest,/^[a-f0-9]{64}$/);assert.match(value.dependencyDigest,/^[a-f0-9]{64}$/);}
  assert.equal(run.stdout.includes(f.root),false);
  await writeFile(join(f.service,'src/credential-service.mjs'),'// changed reviewed source\n');
  const changed=JSON.parse(invoke(['--preflight-only'],f.env).stdout);
  assert.equal(changed.sources.service.dirty,true);assert.notEqual(changed.sources.service.contentDigest,result.sources.service.contentDigest);
  assert.deepEqual(changed.sources.sdk,result.sources.sdk);
 }finally{await f.close();}
});

test('content identity covers installed dependencies while non-Git SDK remains explicit',async()=>{
 const f=await sources();try{
  await rm(join(f.sdk,'.git'),{recursive:true});
  await mkdir(join(f.service,'node_modules','synthetic-package'),{recursive:true});
  await writeFile(join(f.service,'node_modules','synthetic-package','index.js'),'// dependency v1\n');
  const first=JSON.parse(invoke(['--preflight-only'],f.env).stdout);
  assert.equal(first.sources.sdk.revision,null);assert.equal(first.sources.sdk.dirty,null);
  await writeFile(join(f.service,'node_modules','synthetic-package','index.js'),'// dependency v2\n');
  const next=JSON.parse(invoke(['--preflight-only'],f.env).stdout);
  assert.notEqual(next.sources.service.dependencyDigest,first.sources.service.dependencyDigest);
  assert.equal(next.sources.service.contentDigest,first.sources.service.contentDigest);
  assert.deepEqual(next.sources.sdk,first.sources.sdk);
 }finally{await f.close();}
});

test('invalid arguments and help do not require sources or import candidate code',()=>{
 const help=invoke(['--help']);assert.equal(help.status,0);assert.match(help.stdout,/--preflight-only/);
 const bad=invoke(['--source','private-root']);assert.equal(bad.status,1);assert.deepEqual(JSON.parse(bad.stderr),{code:'auth_composition_arguments_invalid'});
});
