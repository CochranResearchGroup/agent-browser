import test from 'node:test';
import assert from 'node:assert/strict';
import {appProcessOwned,privateProfileReferenced,retainViewerProcess,viewerCleanupCensus} from './hosted-process-ownership.mjs';
const root={pid:100,start:'20',uid:42,session:100,processGroup:100};
const child={pid:101,start:'21',uid:42,session:100,processGroup:100,parentPid:100};
test('missing marker child qualifies by private kernel session',()=>assert(appProcessOwned(child,root)));
test('retained exact child qualifies after reparent and session change',()=>assert(appProcessOwned({...child,parentPid:1,session:9},root,new Map([[101,child]]))));
test('PID reuse fails closed',()=>assert.throws(()=>appProcessOwned({...child,start:'22'},root,new Map([[101,child]])),/reused/));
test('foreign UID, SID, group or older process does not qualify',()=>{for(const change of [{uid:1},{session:99},{processGroup:99},{start:'19'}])assert.equal(appProcessOwned({...child,...change},root),false);});
test('profile references survive flattened Chrome argv',()=>{assert(privateProfileReferenced('chrome --user-data-dir=/private/profile --flag','/private/profile'));assert(privateProfileReferenced('chrome\0--user-data-dir=/private/profile\0','/private/profile'));assert.equal(privateProfileReferenced('chrome --user-data-dir=/other','/private/profile'),false);});

import {sameProcessInstance,retainedProcessOwned} from './hosted-process-ownership.mjs';
test('private-home marker-proven crashpad remains retained after env marker loss',()=>{
 const prior={...child,executable:'/private/chrome_crashpad_handler'};
 const current={...prior,parentPid:1,session:88,processGroup:88};
 const retained=new Map([[current.pid,prior]]);
 assert(retainedProcessOwned(current,retained));
 assert.equal(retainedProcessOwned({...current,start:'22'},retained),false);
 assert.throws(()=>retainedProcessOwned({...current,executable:null},retained),/unknown/);
 assert.throws(()=>retainedProcessOwned({...current,uid:43},retained),/unknown/);
 assert.equal(sameProcessInstance({...current,executable:'/changed'},prior),false);
});

const viewerRoot={...root,executable:'/reviewed/chrome'};
function viewerFixture(){
 const observed=new Map();let rootReads=0;
 return {observed,get rootReads(){return rootReads;},args:{root:viewerRoot,observed,
  readRoot:async()=>{rootReads++;return {...viewerRoot};},readKernel:async()=>({...child})}};
}
test('new sandbox child needs exact launched viewer root and a fresh kernel identity, without environ/exe access',async()=>{
 const f=viewerFixture();assert(await retainViewerProcess(child,f.args));
 assert.equal(f.rootReads,1);assert.deepEqual(f.observed.get(child.pid),child);
 assert.equal(f.observed.get(child.pid).executable,undefined);
 const reparented={...child,parentPid:1,session:9,processGroup:9};
 assert(await retainViewerProcess(reparented,{...f.args,readRoot:async()=>{throw Error('root exited');},readKernel:async()=>reparented}));
});
test('foreign UID, SID, group and older generation never acquire viewer custody',async()=>{
 for(const change of [{uid:43},{session:99},{processGroup:99},{start:'19'}]){
  const f=viewerFixture();assert.equal(await retainViewerProcess({...child,...change},f.args),false);
  assert.equal(f.observed.size,0);assert.equal(f.rootReads,0);
 }
});
test('reused or changed viewer root, retained child generation and UID fail closed',async()=>{
 for(const change of [{start:'22'},{uid:43},{executable:'/foreign/browser'}]){
  const f=viewerFixture();await assert.rejects(retainViewerProcess(child,{...f.args,readRoot:async()=>({...viewerRoot,...change})}),/root_identity_unknown/);
  assert.equal(f.observed.size,0);
 }
 const f=viewerFixture();await retainViewerProcess(child,f.args);
 await assert.rejects(retainViewerProcess({...child,start:'22'},f.args),/pid_reused/);
 await assert.rejects(retainViewerProcess({...child,uid:43},f.args),/identity_unknown/);
 await assert.rejects(retainViewerProcess(child,{...f.args,readKernel:async()=>({...child,start:'22'})}),/identity_changed/);
});
test('kernel-only viewer cleanup reports live restricted identity unresolved, not absent or signalable',async()=>{
 const f=viewerFixture();await retainViewerProcess(child,f.args);
 const restricted=await viewerCleanupCensus(f.observed,async()=>{throw Object.assign(Error('sandbox denied'),{code:'EACCES'});});
 assert.deepEqual(restricted.owned,[]);assert.equal(restricted.unresolved[0].pid,child.pid);
 assert.deepEqual(await viewerCleanupCensus(f.observed,async()=>null),{owned:[],unresolved:[]});
 for(const change of [{uid:43},{start:'22'},{executable:undefined}]){
  const result=await viewerCleanupCensus(f.observed,async()=>({...child,executable:'/reviewed/chrome',...change}));
  assert.equal(result.owned.length,0);assert.equal(result.unresolved.length,1);
 }
});
