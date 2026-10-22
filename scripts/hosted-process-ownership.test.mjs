import test from 'node:test';
import assert from 'node:assert/strict';
import {appProcessOwned,privateProfileReferenced} from './hosted-process-ownership.mjs';
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
