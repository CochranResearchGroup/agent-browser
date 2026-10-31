import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {lstat,readdir,readFile,realpath} from 'node:fs/promises';
import {isAbsolute,join,relative,sep} from 'node:path';

const definitions=[
 {role:'service',input:'AUTH_RECEIPTS_SOURCE',key:'source',required:['src/standalone.mjs','src/credential-service.mjs','src/receipts-approval.mjs','src/keepass-backend.mjs','src/credential-sources.mjs','src/native-browser-receiver.mjs'],scope:['src','scripts','package.json','package-lock.json'],dependencies:true},
 {role:'sdk',input:'AUTH_RECEIPTS_SDK_SOURCE',key:'sdkSource',required:['src/client-sdk.mjs','src/client-sdk-verification.mjs','src/auth-client.mjs','src/auth-transport.mjs'],scope:['src/client-sdk.mjs','src/client-sdk-verification.mjs','src/auth-client.mjs','src/auth-transport.mjs','package.json','package-lock.json']},
 {role:'parent',input:'RECEIPTS_SOURCE',key:'parentSource',required:['apps/receipts-interoperation/src/request-hub.mjs','apps/receipts-interoperation/src/notifications.mjs'],scope:['apps/receipts-interoperation/src','apps/receipts-interoperation/package.json','package.json','package-lock.json','pnpm-lock.yaml']},
];
const schemaVersion='agent-browser.auth-composition-preflight.v1';

/**
 * Validate caller-selected source trees without importing code, opening an
 * authority or reading installed discovery. Public metadata contains no roots.
 * contentDigest covers the declared source/manifest closure; dependencyDigest
 * additionally covers installed service dependencies, including their bytes.
 * A non-Git SDK copy has nullable revision/dirty and remains content-identified.
 */
export async function preflightAuthComposition(options={}){
 const problems=[],selected=[];
 for(const definition of definitions){
  const value=options[definition.key];
  if(typeof value!=='string'||!value){problems.push({input:definition.input,reason:'missing_root'});continue;}
  if(!isAbsolute(value)){problems.push({input:definition.input,reason:'absolute_root_required'});continue;}
  let root;
  try{root=await realpath(value);if(!(await lstat(root)).isDirectory())throw Error();}
  catch{problems.push({input:definition.input,reason:'invalid_root'});continue;}
  const missing=[];
  for(const path of ['package.json',...definition.required]){
   try{if(!(await lstat(join(root,path))).isFile())throw Error();}catch{missing.push(path);}
  }
  if(missing.length){problems.push({input:definition.input,reason:'missing_entry_points',entries:missing});continue;}
  selected.push({...definition,root});
 }
 if(problems.length)throw failure(problems);
 const sources={};
 for(const definition of selected){
  try{
   const {root}=definition;
   const contentDigest=await digestTrees(root,definition.scope);
   const manifest=JSON.parse(await readFile(join(root,'package.json'),'utf8'));
   if(definition.dependencies&&Object.keys(manifest.dependencies??{}).length){
    for(const dependency of Object.keys(manifest.dependencies))if(!(await lstat(join(root,'node_modules',dependency,'package.json'))).isFile())throw Error();
   }
   const dependencyDigest=definition.dependencies?await digestTrees(root,['node_modules']):emptyDigest();
   const git=gitIdentity(root);
   if(definition.role!=='sdk'&&git.revision===null)throw Error('Reviewed repository identity required');
   sources[definition.role]={...git,contentDigest,dependencyDigest};
  }catch{problems.push({input:definition.input,reason:'source_identity_unavailable'});}
 }
 if(problems.length)throw failure(problems);
 return {schemaVersion,sources,environment:{node:process.versions.node,platform:process.platform,arch:process.arch}};
}

function failure(problems){return Object.assign(Error('Auth composition preflight failed'),{code:'auth_composition_preflight_failed',problems});}
function emptyDigest(){return createHash('sha256').digest('hex');}
function gitIdentity(root){
 const environment=Object.fromEntries(Object.entries(process.env).filter(([key])=>!key.startsWith('GIT_')));
 const run=args=>execFileSync('git',['-c','core.fsmonitor=false','-c','core.untrackedCache=false','-C',root,...args],{encoding:'utf8',stdio:['ignore','pipe','ignore'],env:{...environment,GIT_CONFIG_GLOBAL:'/dev/null',GIT_CONFIG_NOSYSTEM:'1',GIT_OPTIONAL_LOCKS:'0'}}).trim();
 try{
  const revision=run(['rev-parse','HEAD']);if(!/^[a-f0-9]{40,64}$/.test(revision))throw Error();
  return {revision,dirty:run(['status','--porcelain','--untracked-files=normal','.'])!==''};
 }catch{return {revision:null,dirty:null};}
}
async function digestTrees(root,scopes){
 const entries=[];
 const visit=async path=>{
  let stat;try{stat=await lstat(join(root,path));}catch(error){if(error.code==='ENOENT')return;throw error;}
  if(stat.isSymbolicLink()){
   const resolved=await realpath(join(root,path)),inside=relative(root,resolved);
   if(inside==='..'||inside.startsWith(`..${sep}`)||isAbsolute(inside))throw Error('Source link escapes selected root');
   // File links are content-bound. Directory links can hide undeclared trees.
   if(!(await lstat(resolved)).isFile())throw Error('Directory source link unsupported');
   entries.push([path,await readFile(resolved)]);return;
  }
  if(stat.isDirectory()){for(const child of (await readdir(join(root,path))).sort())await visit(join(path,child));return;}
  if(!stat.isFile())throw Error('Unsupported source entry');
  entries.push([path,await readFile(join(root,path))]);
 };
 for(const scope of scopes)await visit(scope);
 const hash=createHash('sha256');
 for(const [path,bytes]of entries.sort(([a],[b])=>a.localeCompare(b))){hash.update(JSON.stringify([path,bytes.length]));hash.update(bytes);}
 return hash.digest('hex');
}
