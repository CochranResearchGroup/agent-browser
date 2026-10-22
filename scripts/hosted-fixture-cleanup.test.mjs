import test from 'node:test';
import assert from 'node:assert/strict';
import {cleanupExactOwnedProcesses} from './hosted-fixture-cleanup.mjs';
import {retainedProcessOwned} from './hosted-process-ownership.mjs';

const processRow=(pid=100,extra={})=>({pid,start:'20',uid:42,executable:'/private/candidate',parentPid:1,session:pid,processGroup:pid,...extra});
function fixture(rows=[processRow()],options={}) {
 let tick=0;const live=new Map(rows.map(row=>[row.pid,{...row}]));const signals=[],records=[];
 const known=new Map(rows.map(row=>[row.pid,{...row}]));
 return {live,signals,records,args:{known,now:()=>tick,sleep:async ms=>{tick+=ms;},graceMs:20,killMs:20,pollMs:10,
  census:async()=>({owned:[...live.values()].filter(row=>retainedProcessOwned(row,known)),unresolved:[]}),
  readIdentity:async pid=>live.get(pid)??null,
  signal:async(pid,signal)=>{signals.push({pid,signal});live.delete(pid);},
  record:async receipt=>records.push(structuredClone(receipt)),...options}};
}
test('retained crashpad generation survives reparent and marker EACCES; exact termination is receipted',async()=>{
 const crashpad=processRow(2520717,{start:'3787764',executable:'/private/chrome_crashpad_handler',session:2520716,processGroup:2520716});
 const f=fixture([crashpad]);
 // No environ read is necessary after positive private-home custody was retained.
 let envReads=0;f.args.census=async()=>{
  const rows=[];for(const row of f.live.values()){
   if(retainedProcessOwned(row,f.args.known)){rows.push(row);continue;}
   envReads++;throw Object.assign(Error('denied'),{code:'EACCES'});
  }return {owned:rows,unresolved:[]};
 };
 const result=await cleanupExactOwnedProcesses(f.args);
 assert.equal(result.qualified,true);assert.equal(envReads,0);
 assert.deepEqual(f.signals,[{pid:2520717,signal:'SIGTERM'}]);
 assert.equal(f.records[0].signals[0].state,'intent');
 assert.equal(f.records[0].signals[0].executable,crashpad.executable);
});
test('foreign churn does not prevent known termination; unresolved ownership still blocks release',async()=>{
 const f=fixture();let samples=0;
 f.args.census=async()=>({owned:[...f.live.values()],unresolved:++samples<3?[{pid:900,code:'EACCES'}]:[],foreignDenied:[{pid:901,start:'1',proof:'baseline_ancestor'}]});
 const result=await cleanupExactOwnedProcesses(f.args);
 assert.equal(f.signals.length,1);assert.equal(result.qualified,true);assert(result.passes[0].unresolved.length);
 const blocked=fixture();blocked.args.census=async()=>({owned:[...blocked.live.values()],unresolved:[{pid:900,code:'EACCES'}]});
 const refused=await cleanupExactOwnedProcesses(blocked.args);
 assert.equal(blocked.signals.length,1);assert.equal(refused.qualified,false);assert.equal(refused.unresolved[0].pid,900);
});
test('unknown owned identity never grants signal authority or absence proof',async()=>{
 const f=fixture();f.args.readIdentity=async()=>{throw Object.assign(Error('denied'),{code:'EACCES'});};
 const result=await cleanupExactOwnedProcesses({...f.args,allowSigkill:true});
 assert.equal(result.qualified,false);assert.equal(f.signals.length,0);assert.equal(result.unknown[0].code,'EACCES');
});
test('surviving daemon escalates only with explicit permission and fresh same UID/start/exe',async()=>{
 for(const allowSigkill of [false,true]){
  const f=fixture();f.args.signal=async(pid,signal)=>{f.signals.push({pid,signal});if(signal==='SIGKILL')f.live.delete(pid);};
  const result=await cleanupExactOwnedProcesses({...f.args,allowSigkill});
  assert.deepEqual(f.signals.map(row=>row.signal),allowSigkill?['SIGTERM','SIGKILL']:['SIGTERM']);
  assert.equal(result.qualified,allowSigkill);
  if(allowSigkill)assert(f.records.some(row=>row.signals.at(-1)?.signal==='SIGKILL'&&row.signals.at(-1).state==='intent'));
 }
});
test('PID reuse between TERM and escalation preserves replacement and proves original generation gone',async()=>{
 const f=fixture();f.args.signal=async(pid,signal)=>{f.signals.push({pid,signal});f.live.set(pid,processRow(pid,{start:'21',executable:'/foreign/other'}));};
 const result=await cleanupExactOwnedProcesses({...f.args,allowSigkill:true});
 assert.equal(result.qualified,true);assert.equal(f.live.get(100).start,'21');assert.deepEqual(f.signals,[{pid:100,signal:'SIGTERM'}]);
 assert(result.observations.some(row=>row.state==='replaced'));
});
test('identity change after census blocks signal and release; signal intent is not replayed on error',async()=>{
 for(const extra of [{uid:43},{executable:'/foreign/changed'},{executable:null}]){
  const f=fixture();f.args.readIdentity=async()=>processRow(100,extra);
  const result=await cleanupExactOwnedProcesses({...f.args,allowSigkill:true});
  assert.equal(result.qualified,false);assert.equal(f.signals.length,0);
 }
 const f=fixture();f.args.signal=async(pid,signal)=>{f.signals.push({pid,signal});throw Object.assign(Error('denied'),{code:'EPERM'});};
 const result=await cleanupExactOwnedProcesses({...f.args,allowSigkill:true});
 assert.equal(result.qualified,false);assert.equal(f.signals.length,1);assert.equal(result.signals[0].state,'failed');
});
test('new exact owned child discovered during settling is cleaned without a process-group kill',async()=>{
 const f=fixture();let created=false;
 f.args.signal=async(pid,signal)=>{
  f.signals.push({pid,signal});f.live.delete(pid);
  if(!created){created=true;const child=processRow(101,{start:'21'});f.args.known.set(child.pid,child);f.live.set(child.pid,child);}
 };
 const result=await cleanupExactOwnedProcesses(f.args);
 assert.equal(result.qualified,true);assert.deepEqual(f.signals.map(row=>row.pid),[100,101]);
});

test('PID replacement while intent is persisted is rechecked before any signal',async()=>{
 const f=fixture();f.args.record=async receipt=>{
  f.records.push(structuredClone(receipt));
  if(receipt.signals.at(-1)?.state==='intent')f.live.set(100,processRow(100,{start:'21',executable:'/foreign/replacement'}));
 };
 const result=await cleanupExactOwnedProcesses({...f.args,allowSigkill:true});
 assert.equal(result.qualified,true);assert.equal(f.signals.length,0);
 assert.equal(result.signals[0].state,'not_dispatched_identity_changed');
});

test('census cannot replace retained same-generation executable custody',async()=>{
 const f=fixture([processRow(),processRow(101)]);
 f.live.set(100,processRow(100,{executable:'/foreign/exec-replacement'}));
 f.args.census=async()=>({owned:[...f.live.values()],unresolved:[]});
 const result=await cleanupExactOwnedProcesses({...f.args,allowSigkill:true});
 assert.equal(result.qualified,false);
 assert.deepEqual(f.signals,[{pid:101,signal:'SIGTERM'}]);
 assert.equal(result.owned.find(row=>row.pid===100).executable,'/private/candidate');
 assert(result.unresolved.some(row=>row.code==='fixture_cleanup_retained_identity_conflict'));
 assert.equal(result.unknown[0].pid,100);
});
