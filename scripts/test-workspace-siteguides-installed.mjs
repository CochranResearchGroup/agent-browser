import assert from 'node:assert/strict';
import {mkdtemp,readFile,writeFile,mkdir,rm,copyFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';import {join} from 'node:path';import {spawnSync,spawn} from 'node:child_process';
const root=await mkdtemp(join(tmpdir(),'p300-installed-'));
try{
 const support=join(root,'generation','support');await mkdir(join(support,'teaching'),{recursive:true});
 await writeFile(join(support,'package.json'),'{"type":"module"}');
 const rust=await readFile(new URL('../cli/src/workstation_install.rs',import.meta.url),'utf8');
 const assets=[...rust.matchAll(/\(\s*"teaching\/([^"/]+\.js)",\s*include_str!\("\.\.\/\.\.\/packages\/client\/src\/([^"/]+)"\),\s*false,?\s*\)/g)];
 assert.equal(assets.length,9,'exact installed manifest assets');
 for(const [,name,source]of assets)await copyFile(new URL('../packages/client/src/'+source,import.meta.url),join(support,'teaching',name));
 const unrelated=join(root,'empty-cwd');await mkdir(unrelated);
 const run=input=>spawnSync(process.execPath,[join(support,'teaching','workspace-siteguides-host.js')],{cwd:unrelated,input:JSON.stringify(input),encoding:'utf8',env:{PATH:process.env.PATH,HOME:root}});
 const missing=run({configPath:join(root,'absent'),actor:'fixture',allowedTasks:{},request:{operation:'list'}});
 assert.equal(missing.status,1);assert.equal(JSON.parse(missing.stdout).success,false);
 const candidates=join(root,'candidates');await mkdir(candidates,{mode:0o700});
 const config=join(root,'config.json');await writeFile(config,JSON.stringify({schemaVersion:'agent-browser.workspace-siteguides-host.v1',candidateDirectory:candidates}),{mode:0o600});
 const response=run({configPath:config,actor:'fixture',allowedTasks:{},request:{operation:'list',actor:'forged',candidateDirectory:'/'}});
 assert.equal(response.status,0,response.stdout+response.stderr);assert.deepEqual(JSON.parse(response.stdout),{success:true,data:{guides:[]}});
 const guide=JSON.parse(await readFile(new URL('./fixtures/teaching-report/draft.json',import.meta.url),'utf8'));
 await writeFile(join(candidates,guide.guideId+'.json'),JSON.stringify(guide),{mode:0o600});
 const mcp=join(root,'mock-mcp.cjs');
 await writeFile(mcp,`#!/usr/bin/env node
const fs=require('node:fs'),path=require('node:path');let input='';process.stdin.on('data',chunk=>input+=chunk);process.stdin.on('end',()=>{const value=JSON.parse(input),args=value.params.arguments;if(value.params.name!=='browser_command'||args.action!=='playbook')process.exit(1);const data=args.params.operation==='inspect_guide'?JSON.parse(fs.readFileSync(path.join(process.env.AGENT_BROWSER_SITEGUIDE_DIRECTORY,args.params.guideId+'.json'),'utf8')):{valid:true};process.stdout.write(JSON.stringify({jsonrpc:'2.0',id:1,result:{content:[{type:'text',text:JSON.stringify({success:true,data})}]}})+'\\n');});`,{mode:0o700});
 const legacy={schemaVersion:'agent-browser.siteguide.v1',guideId:'legacy-existing',name:'Existing ordinary sales guide',allowedOrigins:['https://teaching-report.example.invalid'],states:[]};
 await writeFile(join(candidates,legacy.guideId+'.json'),JSON.stringify(legacy),{mode:0o600});
 const listed=run({configPath:config,browserExecutable:mcp,actor:'fixture',allowedTasks:{task:{read:true,use:true,profileId:'work'}},request:{operation:'list'}});
 assert.equal(listed.status,0,listed.stdout+listed.stderr);const actual=JSON.parse(listed.stdout);assert.equal(actual.data.guides[0].guideId,guide.guideId);assert.equal(actual.data.guides[0].source.state,'unchecked');assert.equal(actual.data.guides[0].ordinaryReuseAllowed,false);assert.equal(actual.data.guides.find(item=>item.guideId===legacy.guideId).kind,'ordinary');
 const inspected=run({configPath:config,browserExecutable:mcp,actor:'fixture',allowedTasks:{task:{read:true,use:true,profileId:'work',profileKind:'named',currentBrowser:false}},request:{operation:'inspect_guide',taskId:'task',guideId:legacy.guideId}});assert.equal(inspected.status,0,inspected.stdout);assert.equal(JSON.parse(inspected.stdout).data.guide.name,legacy.name);
 const denied=run({configPath:config,browserExecutable:mcp,actor:'fixture',allowedTasks:{},request:{operation:'list'}});assert.deepEqual(JSON.parse(denied.stdout).data.guides,[]);
 if(process.platform!=='win32'){
  const identities=join(root,'owned-mcp-pids.json'),sleepMcp=join(root,'sleep-mcp.cjs');
  await writeFile(sleepMcp,`#!/usr/bin/env node
const fs=require('node:fs'),{spawn}=require('node:child_process');const child=spawn(process.execPath,['-e','setInterval(()=>{},1000)'],{stdio:'ignore'});fs.writeFileSync(process.env.TEACHING_TEST_PROCESS_FILE,JSON.stringify([process.pid,child.pid]));setInterval(()=>{},1000);`,{mode:0o700});
  const child=spawn(process.execPath,[join(support,'teaching','workspace-siteguides-host.js')],{cwd:unrelated,detached:true,stdio:['pipe','pipe','pipe'],env:{PATH:process.env.PATH,HOME:root,TEACHING_TEST_PROCESS_FILE:identities}});
  let owned=[];
  try{
   child.stdin.end(JSON.stringify({configPath:config,browserExecutable:sleepMcp,actor:'fixture',allowedTasks:{task:{read:true,use:true}},request:{operation:'list'}}));
   for(let attempt=0;attempt<250;attempt++){try{owned=JSON.parse(await readFile(identities,'utf8'));break;}catch{await new Promise(resolve=>setTimeout(resolve,20));}}
   assert.equal(owned.length,2,'owned helper tree started');
   const closed=new Promise(resolve=>child.once('close',resolve));process.kill(-child.pid,'SIGTERM');
   await Promise.race([closed,new Promise((_,reject)=>setTimeout(()=>reject(Error('owned host did not reap after SIGTERM')),5000))]);
   for(const pid of owned){let absent=false;for(let attempt=0;attempt<100;attempt++){try{process.kill(pid,0);}catch(error){if(error.code==='ESRCH'){absent=true;break;}throw error;}await new Promise(resolve=>setTimeout(resolve,20));}assert(absent,'owned MCP child remains: '+pid);}
  }finally{try{process.kill(-child.pid,'SIGKILL');}catch(error){if(error.code!=='ESRCH')throw error;}}
 }
 console.log('Installed teaching assets execute source-free from unrelated cwd; missing configuration fails closed.');
}finally{await rm(root,{recursive:true,force:true});}
