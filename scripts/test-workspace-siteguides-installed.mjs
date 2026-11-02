import assert from 'node:assert/strict';
import {mkdtemp,readFile,writeFile,mkdir,rm,copyFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';import {join} from 'node:path';import {spawnSync} from 'node:child_process';
const root=await mkdtemp(join(tmpdir(),'p300-installed-'));
try{
 const support=join(root,'generation','support');await mkdir(join(support,'teaching'),{recursive:true});
 await writeFile(join(support,'package.json'),'{"type":"module"}');
 const rust=await readFile(new URL('../cli/src/workstation_install.rs',import.meta.url),'utf8');
 const assets=[...rust.matchAll(/\(\s*"teaching\/([^"/]+\.js)",\s*include_str!\("\.\.\/\.\.\/packages\/client\/src\/([^"/]+)"\),\s*false,?\s*\)/g)];
 assert.equal(assets.length,8,'exact installed manifest assets');
 for(const [,name,source]of assets)await copyFile(new URL('../packages/client/src/'+source,import.meta.url),join(support,'teaching',name));
 const unrelated=join(root,'empty-cwd');await mkdir(unrelated);
 const run=input=>spawnSync(process.execPath,[join(support,'teaching','workspace-siteguides-host.js')],{cwd:unrelated,input:JSON.stringify(input),encoding:'utf8',env:{PATH:process.env.PATH,HOME:root}});
 const missing=run({configPath:join(root,'absent'),actor:'fixture',allowedTasks:{},request:{operation:'list'}});
 assert.equal(missing.status,1);assert.equal(JSON.parse(missing.stdout).success,false);
 const candidates=join(root,'candidates');await mkdir(candidates,{mode:0o700});
 const config=join(root,'config.json');await writeFile(config,JSON.stringify({schemaVersion:'agent-browser.workspace-siteguides-host.v1',candidateDirectory:candidates}),{mode:0o600});
 const response=run({configPath:config,actor:'fixture',allowedTasks:{},request:{operation:'list',actor:'forged',candidateDirectory:'/'}});
 assert.equal(response.status,0,response.stdout+response.stderr);assert.deepEqual(JSON.parse(response.stdout),{success:true,data:{guides:[]}});
 console.log('Installed teaching assets execute source-free from unrelated cwd; missing configuration fails closed.');
}finally{await rm(root,{recursive:true,force:true});}
