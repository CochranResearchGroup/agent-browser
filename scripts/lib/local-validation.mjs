import {execFileSync,spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {closeSync,existsSync,lstatSync,mkdirSync,openSync,readFileSync,readdirSync,readlinkSync,realpathSync,renameSync,writeFileSync} from 'node:fs';
import {basename,delimiter,dirname,isAbsolute,join,relative,resolve,sep} from 'node:path';
import {fileURLToPath} from 'node:url';
import {preflightAuthComposition} from './auth-composition-preflight.mjs';

export const PLAN_SCHEMA='agent-browser.local-validation-plan.v1';
export const RECEIPT_SCHEMA='agent-browser.local-validation.v2';
const selector=fileURLToPath(new URL('../dev/select-validation.js',import.meta.url));
const hash=value=>createHash('sha256').update(value).digest('hex');
const stable=value=>JSON.stringify(value);
const fail=code=>{throw Object.assign(Error(code),{code});};
const gitEnvironment=()=>({...Object.fromEntries(Object.entries(process.env).filter(([key])=>!key.startsWith('GIT_'))),GIT_CONFIG_GLOBAL:'/dev/null',GIT_CONFIG_NOSYSTEM:'1',GIT_OPTIONAL_LOCKS:'0'});
function git(args){return execFileSync('git',['-c','core.fsmonitor=false',...args],{encoding:'utf8',stdio:['ignore','pipe','pipe'],env:gitEnvironment()}).trim();}
export function parseOptions(args,allowed,required){
 const options={};
 for(let i=0;i<args.length;i+=2){if(!allowed.includes(args[i])||!args[i+1]||Object.hasOwn(options,args[i]))fail('validation_arguments_invalid');options[args[i]]=args[i+1];}
 if(required.some(key=>!options[key]))fail('validation_arguments_invalid');return options;
}
export function assertOutsideSource(path){const root=realpathSync(git(['rev-parse','--show-toplevel'])),candidate=resolve(path),actual=join(realpathSync(dirname(candidate)),basename(candidate)),rel=relative(root,actual);if(rel===''||!rel.startsWith(`..${sep}`)&&rel!=='..'&&!isAbsolute(rel))fail('validation_output_must_be_outside_source');}
export function writeNewJson(path,value){assertOutsideSource(path);writeFileSync(path,`${JSON.stringify(value,null,2)}\n`,{flag:'wx',mode:0o600});}
function executableIdentity(command){
 const candidates=command.includes('/')?[resolve(command)]:process.env.PATH.split(delimiter).map(directory=>join(directory,command));
 for(const path of candidates){try{const actual=realpathSync(path),stat=lstatSync(actual);if(stat.isFile()&&(stat.mode&0o111))return {digest:hash(readFileSync(actual)),mode:stat.mode&0o111};}catch{}}
 return {missing:true};
}
function repositorySnapshot(){
 const files=execFileSync('git',['-c','core.fsmonitor=false','ls-files','--cached','--others','--exclude-standard','-z'],{encoding:'utf8',env:gitEnvironment()}).split('\0').filter(Boolean).sort();
 const digest=createHash('sha256');
 for(const path of [...new Set(files)]){
  let stat;try{stat=lstatSync(path);}catch(error){if(error.code==='ENOENT'){digest.update(stable([path,'missing']));continue;}throw error;}
  if(stat.isDirectory())fail('validation_submodule_input_unsupported');
  if(stat.isSymbolicLink()){
   const target=readlinkSync(path);if(isAbsolute(target)||relative(process.cwd(),resolve(path,'..',target)).startsWith('..'))fail('validation_external_source_link_unsupported');
   digest.update(stable([path,'link',target]));
  }
  if(!stat.isFile()&&!stat.isSymbolicLink())fail('validation_input_type_unsupported');
  digest.update(stable([path,stat.mode&0o111,stat.isSymbolicLink()?lstatSync(realpathSync(path)).mode&0o111:null,hash(readFileSync(path))]));
 }
 return {scope:'tracked-and-unignored-repository',digest:digest.digest('hex')};
}
/** Hash present installed dependency trees, following content links with cycle and
 * resource bounds. Ignored build/output trees are deliberately not inputs. */
function installedDependencies(){
 const files=execFileSync('git',['ls-files','--cached','--others','--exclude-standard','-z'],{encoding:'utf8',env:gitEnvironment()}).split('\0').filter(Boolean);
 const roots=[...new Set(['node_modules',...files.filter(path=>path.endsWith('/package.json')).map(path=>join(path,'..','node_modules'))])].sort().filter(path=>existsSync(path));
 const cache=new Map(),active=new Set();let entries=0,bytes=0;
 const visit=(path,depth=0)=>{
  if(depth>64)fail('validation_dependency_depth_exceeded');
  const actual=realpathSync(path);if(cache.has(actual))return cache.get(actual);if(active.has(actual))return hash('dependency-cycle');
  if(++entries>200000)fail('validation_dependency_entry_limit_exceeded');
  const stat=lstatSync(actual);active.add(actual);let digest;
  if(stat.isDirectory())digest=hash(stable(readdirSync(actual).sort().map(name=>[name,visit(join(actual,name),depth+1)])));
  else if(stat.isFile()){
   bytes+=stat.size;if(bytes>2147483648||stat.size>268435456)fail('validation_dependency_byte_limit_exceeded');
   digest=hash(stable([stat.mode&0o111,hash(readFileSync(actual))]));
  }else fail('validation_dependency_type_unsupported');
  active.delete(actual);cache.set(actual,digest);return digest;
 };
 return {scope:'present-installed-repository-dependencies',digest:hash(stable(roots.map(path=>[path,visit(path)])))};
}
function toolingIdentity(){
 const files=['../dev/select-validation.js','validation-selection.js','local-validation.mjs','../dev/run-validation.js','../dev/validation-plan.mjs','../dev/assess-validation.mjs','auth-composition-preflight.mjs'];
 return hash(stable(files.map(path=>[path,hash(readFileSync(new URL(path,import.meta.url)))])));
}
function environmentIdentity(){return {node:process.versions.node,nodeExecutable:executableIdentity('node'),platform:process.platform,arch:process.arch,injectionDigest:hash(stable(['NODE_OPTIONS','NODE_PATH'].map(key=>[key,process.env[key]??null])))};}
function selectedCheck(recommendation){
 const parts=recommendation.command.split(' ');const command=parts.shift();
 const unresolved= /[<>]|[\n\r\0]/.test(recommendation.command)||parts.some(arg=>arg.startsWith('/'))||/^pnpm (?:publish:|development-runtime:|install |version:sync)/.test(recommendation.command);
 return {id:`check-${hash(recommendation.command).slice(0,16)}`,command,args:parts.map(arg=>isAbsolute(arg)?'<private-path>':arg),...(unresolved?{hold:'unsupported_or_effectful_recommendation'}:{})};
}
/** Build public required checks from the owner selector, never from a caller's subset. */
export async function createValidationPlan({base,head='HEAD',timeoutMs}){
 if(realpathSync(process.cwd())!==realpathSync(git(['rev-parse','--show-toplevel'])))fail('validation_repo_root_required');
 if(git(['rev-parse',head])!==git(['rev-parse','HEAD']))fail('validation_head_not_current');
 if(timeoutMs!==undefined&&(!Number.isInteger(timeoutMs)||timeoutMs<1||timeoutMs>2147483647))fail('validation_timeout_invalid');
 const selection=JSON.parse(execFileSync(process.execPath,[selector,'--base',base,'--head',head,'--json'],{encoding:'utf8',stdio:['ignore','pipe','pipe'],env:gitEnvironment()}));
 const checks=selection.recommendations.map(selectedCheck);
 const coverage={complete:selection.coverage.complete,gaps:[...selection.coverage.gaps,...checks.filter(check=>check.hold).map(check=>({checkId:check.id,reason:check.hold}))]};coverage.complete=coverage.complete&&coverage.gaps.length===0;
 let auth;
 if(checks.some(check=>check.args.some(arg=>/^(?:test:owner-auth-source|test:standalone-auth-bootstrap)$/.test(arg))))auth=await preflightAuthComposition({source:process.env.AUTH_RECEIPTS_SOURCE,sdkSource:process.env.AUTH_RECEIPTS_SDK_SOURCE,parentSource:process.env.RECEIPTS_SOURCE});
 const repository=repositorySnapshot(),dependencies=installedDependencies(),environment=environmentIdentity(),toolingDigest=toolingIdentity();
 return {schemaVersion:PLAN_SCHEMA,base,head,...(timeoutMs===undefined?{}:{timeoutMs}),source:{head:git(['rev-parse','HEAD']),dirty:git(['status','--porcelain'])!==''},coverage,environment,toolingDigest,checks:checks.map(check=>{
  const inputs={repository,dependencies,environment,toolingDigest,requirement:{command:check.command,args:check.args,timeoutMs:timeoutMs??null},executable:executableIdentity(check.command),...(auth?{auth}: {})};
  return {...check,...(timeoutMs===undefined?{}:{timeoutMs}),inputs,inputDigest:hash(stable(inputs))};
 })};
}
/** Recompute the owner selection and bytes so an edited plan cannot drop checks. */
export async function verifyValidationPlan(plan){
 if(plan?.schemaVersion!==PLAN_SCHEMA||typeof plan.base!=='string'||typeof plan.head!=='string')fail('validation_plan_schema_invalid');
 const current=await createValidationPlan({base:plan.base,head:plan.head,timeoutMs:plan.timeoutMs});
 if(!current.coverage.complete)fail('validation_coverage_unresolved');
 if(stable(plan)!==stable(current))fail('validation_plan_stale_or_incomplete');
 return current;
}
/** Execute trusted selected direct commands; assessment never dispatches them. */
export async function runBoundValidation(plan,output){
 await verifyValidationPlan(plan);assertOutsideSource(output);mkdirSync(output,{mode:0o700});
 const receipt={schemaVersion:RECEIPT_SCHEMA,planDigest:hash(stable(plan)),source:plan.source,environment:plan.environment,startedAt:new Date().toISOString(),state:'running',checks:plan.checks.map((check,index)=>({...check,state:'pending',log:`${index+1}-${check.id}.log`}))};
 const save=()=>{writeFileSync(join(output,'receipt.tmp'),`${JSON.stringify(receipt,null,2)}\n`,{mode:0o600});renameSync(join(output,'receipt.tmp'),join(output,'receipt.json'));};save();
 for(const check of receipt.checks){
  check.state='running';check.startedAt=new Date().toISOString();save();const log=openSync(join(output,check.log),'wx',0o600),started=performance.now();let result;
  try{result=spawnSync(check.command,check.args,{shell:false,stdio:['ignore',log,log],timeout:check.timeoutMs,env:gitEnvironment()});}finally{closeSync(log);}
  check.durationMs=Math.round(performance.now()-started);check.finishedAt=new Date().toISOString();check.exitCode=result.status;check.signal=result.signal;check.errorCode=result.error?.code??null;check.state=result.status===0&&!result.signal&&!result.error?'success':'failure';save();console.log(`${check.id}: ${check.state}`);
 }
 // A check may modify its inputs: a successful exit does not preserve qualification.
 let sourceUnchanged=true;try{await verifyValidationPlan(plan);}catch{sourceUnchanged=false;}
 receipt.sourceUnchanged=sourceUnchanged;receipt.state=receipt.checks.every(check=>check.state==='success')&&sourceUnchanged?'success':'failure';receipt.finishedAt=new Date().toISOString();save();
 console.log(`Batch: ${receipt.state}`);return receipt;
}
/** Read-only assessment of complete required checks and the exact current source. */
export async function assessValidation(plan,receipt){
 const problems=[];let current;
 try{current=await verifyValidationPlan(plan);}catch(error){problems.push(error.code??'validation_plan_unavailable');}
 if(receipt?.schemaVersion!==RECEIPT_SCHEMA)problems.push('validation_receipt_schema_unsupported');
 if(receipt?.planDigest!==hash(stable(plan)))problems.push('validation_receipt_plan_mismatch');
 if(receipt?.state!=='success'||!receipt?.finishedAt||receipt?.sourceUnchanged!==true)problems.push('validation_batch_incomplete_or_failed');
 const checks=[];
 for(const expected of current?.checks??plan?.checks??[]){
  const matches=Array.isArray(receipt?.checks)?receipt.checks.filter(check=>check.id===expected.id):[];
  const actual=matches[0];let status='success';
  if(matches.length!==1||!actual||actual.state!=='success'||actual.exitCode!==0||actual.signal||actual.errorCode||!actual.finishedAt||actual.inputDigest!==expected.inputDigest||stable(actual.inputs)!==stable(expected.inputs)||actual.timeoutMs!==expected.timeoutMs||actual.command!==expected.command||stable(actual.args)!==stable(expected.args)){problems.push(`validation_check_unqualified:${expected.id}`);status='unqualified';}
  checks.push({id:expected.id,state:status});
 }
 if(receipt?.checks?.length!==plan?.checks?.length)problems.push('validation_receipt_check_set_mismatch');
 return {schemaVersion:'agent-browser.local-validation-readiness.v1',ready:problems.length===0,source:current?.source??plan?.source,checks,problems:[...new Set(problems)]};
}
