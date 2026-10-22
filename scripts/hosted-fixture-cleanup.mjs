import {sameProcessInstance} from './hosted-process-ownership.mjs';

const absent = error => ['ENOENT','ESRCH'].includes(error?.code);
const describe = error => ({code:error?.code??'fixture_cleanup_observation_failed'});

/** Read kernel identity independently of /proc/PID/exe availability.
 * A zombie or unreadable executable is not kernel absence. Only an identity
 * read reporting ENOENT/ESRCH permits null; exe failures are wrapped so callers
 * cannot mistake their ENOENT for an absent PID.
 */
export async function readFreshCleanupIdentity(pid,{identity,executable}) {
 const readKernel=async()=>{
  try{
   const row=await identity(pid);
   if(row?.pid!==pid||!Number.isInteger(row.uid)||typeof row.start!=='string'||!/^\d+$/.test(row.start))throw Object.assign(Error('fixture_cleanup_kernel_identity_unknown'),{code:'fixture_cleanup_kernel_identity_unknown'});
   return row;
  }catch(error){if(absent(error))return null;throw error;}
 };
 const before=await readKernel();
 if(before===null)return null;
 let resolved,executableError;
 try{resolved=await executable(pid);}catch(error){executableError=error;}
 // Always re-read stat, including after an exe ENOENT. Its kernel generation
 // decides absence; executable readability never makes that decision.
 const after=await readKernel();
 if(after===null)return null;
 if(after.start!==before.start||after.uid!==before.uid)throw Object.assign(Error('fixture_cleanup_identity_changed'),{code:'fixture_cleanup_identity_changed'});
 if(executableError||typeof resolved!=='string'||!resolved.startsWith('/'))throw Object.assign(Error('fixture_cleanup_executable_unknown'),{code:'fixture_cleanup_executable_unknown',cause:executableError});
 return {...after,executable:resolved};
}

/**
 * Exact synthetic-fixture cleanup, with injectable OS operations. census returns
 * {owned,unresolved,foreignDenied}; unresolved observations block qualification,
 * but never prevent cleanup of positively recorded owned instances. readIdentity
 * returns a fresh UID/start/executable tuple, null only for positive absence.
 * The caller retains each custody row before invoking this helper. No PID group
 * operations, discovery-based kill, or automatic SIGKILL permission exists here.
 */
export async function cleanupExactOwnedProcesses({
 census,readIdentity,signal,known = new Map(),sleep,now = Date.now,
 graceMs = 1000,killMs = 1000,pollMs = 100,allowSigkill = false,
 receipt = {},record = async()=>{},
}) {
 if(![graceMs,killMs,pollMs].every(n=>Number.isFinite(n)&&n>=0)||pollMs===0)throw Error('fixture_cleanup_invalid_bounds');
 for(const operation of [census,readIdentity,signal,sleep,record])if(typeof operation!=='function')throw Error('fixture_cleanup_missing_adapter');
 Object.assign(receipt,{qualified:false,allowSigkill,signals:[],observations:[],passes:[]});
 const owned=new Map();
 const retain=row=>{
  if(!Number.isSafeInteger(row?.pid)||row.pid<=1||!Number.isInteger(row.uid)||!/^\d+$/.test(row.start??'')||typeof row.executable!=='string'||!row.executable.startsWith('/'))throw Error('fixture_cleanup_incomplete_owned_identity');
  // Keep generations independently so a reused PID cannot erase older custody.
  const key=`${row.pid}:${row.start}`,prior=owned.get(key);
  if(prior&&!sameProcessInstance(row,prior))throw Error('fixture_cleanup_retained_identity_conflict');
  owned.set(key,{...row});
 };
 for(const row of known.values())retain(row);
 const capture=async()=>{
  let sample;
  try{sample=await census();}catch(error){sample={owned:[],unresolved:[describe(error)]};}
  const normalized=Array.isArray(sample)?{owned:sample,unresolved:[],foreignDenied:[]}:sample;
  if(!normalized||!Array.isArray(normalized.owned)||!Array.isArray(normalized.unresolved??[]))throw Error('fixture_cleanup_invalid_census');
  for(const row of normalized.owned){
   try{retain(row);}catch(error){
    normalized.unresolved??=[];
    normalized.unresolved.push({pid:row?.pid,start:row?.start,code:error.message});
   }
  }
  receipt.passes.push({owned:normalized.owned.map(row=>({pid:row.pid,start:row.start})),unresolved:normalized.unresolved??[],foreignDenied:normalized.foreignDenied??[]});
  receipt.owned=[...owned.values()];
  return normalized;
 };
 const inspect=async row=>{
  try{
   const current=await readIdentity(row.pid);
   if(current===null)return {state:'absent'};
   if(current?.pid!==row.pid||typeof current.start!=='string'||!/^\d+$/.test(current.start))return {state:'unknown',code:'fixture_cleanup_incomplete_identity'};
   // A fresh different kernel generation proves the original is gone, but never
   // transfers its kill authority to the replacement process.
   if(current.start!==row.start)return {state:'replaced',currentStart:current.start};
   if(!sameProcessInstance(current,row))return {state:'unknown',code:'fixture_cleanup_identity_changed'};
   return {state:'live'};
  }catch(error){return absent(error)?{state:'absent'}:{state:'unknown',...describe(error)};}
 };
 const scan=async()=>{
  const live=[],unknown=[];
  for(const row of owned.values()){
   const observation=await inspect(row);receipt.observations.push({pid:row.pid,start:row.start,...observation});
   if(observation.state==='live')live.push(row);
   if(observation.state==='unknown')unknown.push({pid:row.pid,start:row.start,...observation});
  }
  return {live,unknown};
 };
 const failedSignaling=new Set();
 const dispatch=async(row,selectedSignal)=>{
  const fresh=await inspect(row);
  if(fresh.state!=='live'){receipt.observations.push({pid:row.pid,start:row.start,...fresh});return;}
  const effect={pid:row.pid,start:row.start,uid:row.uid,executable:row.executable,signal:selectedSignal,state:'intent'};
  receipt.signals.push(effect);
  // Retain the reviewed exact intent before the effect. An ambiguous signal
  // error is terminal for that generation/signal and is never replayed.
  await record(receipt);
  const confirmed=await inspect(row);
  if(confirmed.state!=='live'){
   effect.state='not_dispatched_identity_changed';
   receipt.observations.push({pid:row.pid,start:row.start,...confirmed});
   await record(receipt);return;
  }
  try{await signal(row.pid,selectedSignal);effect.state='dispatched';}
  catch(error){
   effect.state=absent(error)?'already_absent':'failed';effect.error=describe(error);
   if(!absent(error))failedSignaling.add(`${row.pid}:${row.start}`);
  }
  await record(receipt);
 };
 const dispatched=new Set();
 const phase=async(selectedSignal,duration)=>{
  const deadline=now()+duration;
  let sample,current;
  do{
   sample=await capture();current=await scan();
   for(const row of current.live){
    const key=`${row.pid}:${row.start}:${selectedSignal}`;
    if(!dispatched.has(key)&&!failedSignaling.has(`${row.pid}:${row.start}`)){dispatched.add(key);await dispatch(row,selectedSignal);}
   }
   if(current.live.length===0&&current.unknown.length===0&&!(sample.unresolved??[]).length)break;
   if(now()>=deadline)break;
   await sleep(Math.min(pollMs,Math.max(1,deadline-now())));
  }while(true);
  return {sample,current};
 };
 await phase('SIGTERM',graceMs);
 // Explicit escalation is limited to exact identities still observable now.
 if(allowSigkill)await phase('SIGKILL',killMs);
 const finalSample=await capture();
 const final=await scan();
 receipt.remaining=final.live;receipt.unknown=final.unknown;
 receipt.unresolved=finalSample.unresolved??[];
 receipt.qualified=final.live.length===0&&final.unknown.length===0&&receipt.unresolved.length===0&&!receipt.signals.some(row=>row.state==='failed'||row.state==='intent');
 await record(receipt);
 return receipt;
}
