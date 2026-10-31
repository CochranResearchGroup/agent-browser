import assert from 'node:assert/strict';
import {spawn,spawnSync,execFileSync} from 'node:child_process';
import {chmodSync,existsSync,mkdirSync,mkdtempSync,readFileSync,rmSync,symlinkSync,writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
import test from 'node:test';

const commands=Object.fromEntries(['validation-plan.mjs','run-validation.js','assess-validation.mjs'].map(name=>[name,resolve('scripts/dev',name)]));
function fixture(t){
 const root=mkdtempSync(join(tmpdir(),'local-validation-505-'));t.after(()=>rmSync(root,{recursive:true,force:true}));
 const repo=join(root,'repo'),bin=join(root,'bin');mkdirSync(repo);mkdirSync(bin);
 const pnpm=join(bin,'pnpm');writeFileSync(pnpm,`#!/usr/bin/env node
if(process.env.FIXTURE_CALLS)require('node:fs').appendFileSync(process.env.FIXTURE_CALLS,process.argv.slice(2).join(' ')+'\\n');
if(process.env.FIXTURE_MODE==='touch-pnpm'){const fs=require('node:fs'),path='node_modules/.pnpm-workspace-state-v1.json',data=JSON.parse(fs.readFileSync(path,'utf8'));data.lastValidatedTimestamp++;fs.writeFileSync(path,JSON.stringify(data));}
if(process.env.FIXTURE_MODE==='workspace-build'){const fs=require('node:fs');fs.mkdirSync('docs/.next',{recursive:true});fs.writeFileSync('docs/.next/generated',String(Date.now()));}
if(process.env.FIXTURE_MODE==='fail-docs' && process.argv.includes('docs'))process.exit(7);
if(process.env.FIXTURE_MODE==='mutate')require('node:fs').writeFileSync('docs/change.md','# Modified during check');
if(process.env.FIXTURE_MODE==='timeout')setInterval(()=>{},1000);
else console.log('synthetic check passed');
`);chmodSync(pnpm,0o700);
 const env={...process.env,PATH:`${bin}:${process.env.PATH}`};
 const git=args=>execFileSync('git',args,{cwd:repo,env,stdio:'pipe'}).toString().trim();
 git(['init']);git(['config','user.name','Fixture']);git(['config','user.email','fixture@example.invalid']);
 mkdirSync(join(repo,'docs'));writeFileSync(join(repo,'.gitignore'),'node_modules/\n.next/\n');writeFileSync(join(repo,'docs','base.md'),'# Base\n');git(['add','.']);git(['commit','-m','base']);const base=git(['rev-parse','HEAD']);
 writeFileSync(join(repo,'docs','change.md'),'# Change\n');git(['add','.']);git(['commit','-m','change']);
 const run=(name,args)=>spawnSync(process.execPath,[commands[name],...args],{cwd:repo,env,encoding:'utf8',timeout:15000});
 const planPath=join(root,'plan.json'),output=join(root,'results');
 return {root,repo,bin,env,git,run,base,planPath,output,plan:()=>JSON.parse(readFileSync(planPath,'utf8')),receipt:()=>JSON.parse(readFileSync(join(output,'receipt.json'),'utf8'))};
}
test('selected local checks run and assess as complete source-bound evidence',t=>{
 const f=fixture(t),planned=f.run('validation-plan.mjs',['--base',f.base,'--output',f.planPath]);
 assert.equal(planned.status,0,planned.stderr);
 const plan=f.plan();assert.ok(plan.checks.length>=3);assert.ok(plan.checks.some(c=>c.args.includes('test:policy-wiring')));
 const ran=f.run('run-validation.js',['--plan',f.planPath,'--output-dir',f.output]);assert.equal(ran.status,0,ran.stderr);
 const assessed=f.run('assess-validation.mjs',['--plan',f.planPath,'--receipt',join(f.output,'receipt.json'),'--json']);
 assert.equal(assessed.status,0,assessed.stderr);const result=JSON.parse(assessed.stdout);assert.equal(result.ready,true);
 assert.equal(f.receipt().schemaVersion,'agent-browser.local-validation.v2');
 assert.ok(!JSON.stringify(result).includes(f.root));
});

test('a subset plan cannot execute or qualify the complete selection',t=>{
 const f=fixture(t);assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',f.planPath]).status,0);
 const plan=f.plan();plan.checks=plan.checks.slice(0,1);writeFileSync(f.planPath,JSON.stringify(plan));
 const result=f.run('run-validation.js',['--plan',f.planPath,'--output-dir',f.output]);assert.equal(result.status,1);assert.match(result.stderr,/validation_plan_stale_or_incomplete/);
});
test('relevant dirty bytes invalidate evidence even while the dirty flag stays true',t=>{
 const f=fixture(t),file=join(f.repo,'docs','change.md');writeFileSync(file,'# Dirty first bytes\n');
 assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',f.planPath]).status,0);assert.equal(f.plan().source.dirty,true);
 assert.equal(f.run('run-validation.js',['--plan',f.planPath,'--output-dir',f.output]).status,0);
 writeFileSync(file,'# Different dirty bytes\n');
 const assessed=f.run('assess-validation.mjs',['--plan',f.planPath,'--receipt',join(f.output,'receipt.json'),'--json']);assert.equal(assessed.status,1);assert.ok(JSON.parse(assessed.stdout).problems.includes('validation_plan_stale_or_incomplete'));
});
test('failed documentation and incomplete required checks never qualify',t=>{
 const f=fixture(t);mkdirSync(join(f.repo,'docs','src','app','sample'),{recursive:true});writeFileSync(join(f.repo,'docs','src','app','sample','page.mdx'),'# Docs\n');
 assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',f.planPath]).status,0);f.env.FIXTURE_MODE='fail-docs';
 const run=f.run('run-validation.js',['--plan',f.planPath,'--output-dir',f.output]);assert.equal(run.status,1);
 const original=readFileSync(join(f.output,'receipt.json'),'utf8'),receipt=f.receipt();assert.equal(receipt.state,'failure');assert.ok(receipt.checks.some(c=>c.exitCode===7));
 const failed=f.run('assess-validation.mjs',['--plan',f.planPath,'--receipt',join(f.output,'receipt.json'),'--json']);assert.equal(failed.status,1);assert.equal(readFileSync(join(f.output,'receipt.json'),'utf8'),original,'assessment preserves historical receipt');
 receipt.state='success';receipt.checks[0].state='pending';delete receipt.checks[0].finishedAt;writeFileSync(join(f.root,'incomplete.json'),JSON.stringify(receipt));
 assert.equal(f.run('assess-validation.mjs',['--plan',f.planPath,'--receipt',join(f.root,'incomplete.json'),'--json']).status,1);
});
test('unknown changed paths remain explicit unresolved coverage',t=>{
 const f=fixture(t);writeFileSync(join(f.repo,'unknown.surface'),'unknown');
 const result=f.run('validation-plan.mjs',['--base',f.base,'--output',f.planPath]);assert.equal(result.status,1);assert.equal(f.plan().coverage.complete,false);
 assert.equal(f.run('run-validation.js',['--plan',f.planPath,'--output-dir',f.output]).status,1);
});
test('selected check timeout retains failure and prevents readiness',t=>{
 const f=fixture(t);const planned=f.run('validation-plan.mjs',['--base',f.base,'--output',f.planPath,'--timeout-ms','100']);assert.equal(planned.status,0,planned.stderr);
 f.env.FIXTURE_MODE='timeout';assert.equal(f.run('run-validation.js',['--plan',f.planPath,'--output-dir',f.output]).status,1);
 assert.ok(f.receipt().checks.some(c=>c.errorCode==='ETIMEDOUT'));
 assert.equal(f.run('assess-validation.mjs',['--plan',f.planPath,'--receipt',join(f.output,'receipt.json'),'--json']).status,1);
});

function authSources(f){
 const layouts=[
 ['AUTH_RECEIPTS_SOURCE',['src/standalone.mjs','src/credential-service.mjs','src/receipts-approval.mjs','src/keepass-backend.mjs','src/credential-sources.mjs','src/native-browser-receiver.mjs']],
 ['AUTH_RECEIPTS_SDK_SOURCE',['src/client-sdk.mjs','src/client-sdk-verification.mjs','src/auth-client.mjs','src/auth-transport.mjs']],
 ['RECEIPTS_SOURCE',['apps/receipts-interoperation/src/request-hub.mjs','apps/receipts-interoperation/src/notifications.mjs']]];
 for(const [key,files]of layouts){
  const root=join(f.root,key);mkdirSync(root);writeFileSync(join(root,'package.json'),JSON.stringify({dependencies:{}}));
  for(const file of files){mkdirSync(join(root,file,'..'),{recursive:true});writeFileSync(join(root,file),"throw Error('Source must not execute in preflight');\n");}
  for(const args of [['init','-q'],['add','.'],['-c','user.name=Fixture','-c','user.email=fixture@example.invalid','commit','-qm','reviewed synthetic source']])execFileSync('git',args,{cwd:root,stdio:'pipe'});
  f.env[key]=root;
 }
 mkdirSync(join(f.repo,'scripts'),{recursive:true});writeFileSync(join(f.repo,'scripts','test-owner-staged-factor.js'),'// selected Auth fixture\n');
}
test('Auth selected prerequisites and changed external source bytes gate readiness',t=>{
 const f=fixture(t);authSources(f);
 const sdk=f.env.AUTH_RECEIPTS_SDK_SOURCE;delete f.env.AUTH_RECEIPTS_SDK_SOURCE;
 const missing=f.run('validation-plan.mjs',['--base',f.base,'--output',f.planPath]);assert.equal(missing.status,1);assert.match(missing.stderr,/AUTH_RECEIPTS_SDK_SOURCE/);assert.ok(!missing.stderr.includes(f.root));
 f.env.AUTH_RECEIPTS_SDK_SOURCE=sdk;assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',f.planPath]).status,0);
 assert.ok(f.plan().checks.filter(c=>c.args.includes('test:owner-auth-source')).every(c=>c.inputs.auth.sources.service.contentDigest));
 assert.ok(!JSON.stringify(f.plan()).includes(f.root));
 assert.equal(f.run('run-validation.js',['--plan',f.planPath,'--output-dir',f.output]).status,0);
 writeFileSync(join(sdk,'src','auth-client.mjs'),'// different actual SDK bytes\n');
 const result=f.run('assess-validation.mjs',['--plan',f.planPath,'--receipt',join(f.output,'receipt.json'),'--json']);assert.equal(result.status,1);assert.ok(JSON.parse(result.stdout).problems.includes('validation_plan_stale_or_incomplete'));
});
test('successful checks that mutate relevant inputs do not qualify their former source',t=>{
 const f=fixture(t);assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',f.planPath]).status,0);f.env.FIXTURE_MODE='mutate';
 assert.equal(f.run('run-validation.js',['--plan',f.planPath,'--output-dir',f.output]).status,1);assert.equal(f.receipt().sourceUnchanged,false);
});
test('changed Node injection settings invalidate a selected execution environment',t=>{
 const f=fixture(t);assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',f.planPath]).status,0);f.env.NODE_OPTIONS='--no-warnings';
 const result=f.run('run-validation.js',['--plan',f.planPath,'--output-dir',f.output]);assert.equal(result.status,1);assert.match(result.stderr,/validation_plan_stale_or_incomplete/);
});

test('an interrupted running batch remains incomplete and read-only assessment rejects it',async t=>{
 const f=fixture(t);assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',f.planPath]).status,0);f.env.FIXTURE_MODE='timeout';
 const child=spawn(process.execPath,[commands['run-validation.js'],'--plan',f.planPath,'--output-dir',f.output],{cwd:f.repo,env:f.env,detached:true,stdio:'ignore'});
 const kill=()=>{try{process.kill(-child.pid,'SIGKILL');}catch{}};t.after(kill);
 const terminal=new Promise(resolve=>child.once('exit',(code,signal)=>resolve({code,signal})));
 const deadline=Date.now()+5000;let running=false;
 while(Date.now()<deadline){try{running=existsSync(join(f.output,'receipt.json'))&&f.receipt().checks.some(check=>check.command==='pnpm'&&check.state==='running');}catch{}if(running)break;await new Promise(resolve=>setTimeout(resolve,20));}
 assert.equal(running,true,'persisted check-running state is the readiness signal');kill();await terminal;
 assert.equal(f.receipt().state,'running');const original=readFileSync(join(f.output,'receipt.json'),'utf8');
 assert.equal(f.run('assess-validation.mjs',['--plan',f.planPath,'--receipt',join(f.output,'receipt.json'),'--json']).status,1);assert.equal(readFileSync(join(f.output,'receipt.json'),'utf8'),original);
});

test('ignored installed dependency bytes remain relevant even without a tracked diff',t=>{
 const f=fixture(t);
 const dependency=join(f.repo,'node_modules','synthetic','index.js');mkdirSync(join(dependency,'..'),{recursive:true});writeFileSync(dependency,'// executable dependency v1\n');
 assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',f.planPath]).status,0);
 assert.equal(f.run('run-validation.js',['--plan',f.planPath,'--output-dir',f.output]).status,0);assert.equal(f.git(['status','--porcelain']),'');
 writeFileSync(dependency,'// executable dependency v2\n');assert.equal(f.git(['status','--porcelain']),'');
 const assessed=f.run('assess-validation.mjs',['--plan',f.planPath,'--receipt',join(f.output,'receipt.json'),'--json']);assert.equal(assessed.status,1);assert.ok(JSON.parse(assessed.stdout).problems.includes('validation_plan_stale_or_incomplete'));
});
test('a historical head cannot omit later changes while qualifying the current checkout',t=>{
 const f=fixture(t);const result=f.run('validation-plan.mjs',['--base',f.base,'--head',f.base,'--output',f.planPath]);
 assert.equal(result.status,1);assert.match(result.stderr,/validation_head_not_current/);
});


test('an unrelated source change reuses an equivalent docs check with immutable history',t=>{
 const f=fixture(t);f.env.FIXTURE_CALLS=join(f.root,'calls.log');
 mkdirSync(join(f.repo,'docs','src','app','sample'),{recursive:true});writeFileSync(join(f.repo,'docs','src','app','sample','page.mdx'),'# Docs\n');
 assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',f.planPath]).status,0);
 assert.equal(f.run('run-validation.js',['--plan',f.planPath,'--output-dir',f.output]).status,0);
 const previousPath=join(f.output,'receipt.json'),previousBytes=readFileSync(previousPath,'utf8');
 writeFileSync(join(f.repo,'README.md'),'# Unrelated repository introduction\n');f.git(['add','.']);f.git(['commit','-m','unrelated introduction']);
 const nextPlan=join(f.root,'next-plan.json'),nextOutput=join(f.root,'next-results');
 assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',nextPlan]).status,0);
 const run=f.run('run-validation.js',['--plan',nextPlan,'--output-dir',nextOutput,'--reuse-receipt',previousPath]);assert.equal(run.status,0,run.stderr);
 const receipt=JSON.parse(readFileSync(join(nextOutput,'receipt.json'),'utf8'));
 const docs=receipt.checks.find(check=>check.command==='pnpm'&&check.args.join(' ')==='--dir docs build');
 assert.equal(docs.outcome,'reused');assert.ok(docs.reusedFrom.receiptDigest);assert.equal(docs.reusedFrom.source.head,JSON.parse(previousBytes).source.head);
 assert.equal(readFileSync(join(f.root,'calls.log'),'utf8').split('\n').filter(line=>line==='--dir docs build').length,1);
 assert.equal(readFileSync(previousPath,'utf8'),previousBytes);
 const assessed=f.run('assess-validation.mjs',['--plan',nextPlan,'--receipt',join(nextOutput,'receipt.json'),'--json']);assert.equal(assessed.status,0,assessed.stderr);
 const summary=JSON.parse(assessed.stdout);assert.equal(summary.ready,true);assert.ok(summary.checks.some(check=>check.id===docs.id&&check.outcome==='reused'));
});


test('changed relevant docs bytes execute again while old receipts remain unchanged',t=>{
 const f=fixture(t);f.env.FIXTURE_CALLS=join(f.root,'calls.log');
 const file=join(f.repo,'docs','src','app','sample','page.mdx');mkdirSync(join(file,'..'),{recursive:true});writeFileSync(file,'# First docs\n');
 assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',f.planPath]).status,0);assert.equal(f.run('run-validation.js',['--plan',f.planPath,'--output-dir',f.output]).status,0);
 const prior=join(f.output,'receipt.json'),bytes=readFileSync(prior,'utf8');writeFileSync(file,'# Changed docs\n');
 const plan=join(f.root,'changed-plan.json'),output=join(f.root,'changed-results');assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',plan]).status,0);
 assert.equal(f.run('run-validation.js',['--plan',plan,'--output-dir',output,'--reuse-receipt',prior]).status,0);
 const receipt=JSON.parse(readFileSync(join(output,'receipt.json'),'utf8'));assert.equal(receipt.checks.find(check=>check.args.join(' ')==='--dir docs build').outcome,'executed');
 assert.equal(readFileSync(join(f.root,'calls.log'),'utf8').split('\n').filter(line=>line==='--dir docs build').length,2);assert.equal(readFileSync(prior,'utf8'),bytes);
});

test('Auth byte identity permits documentation-only source revisions but rejects changed factor source',t=>{
 const f=fixture(t);authSources(f);
 assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',f.planPath]).status,0);assert.equal(f.run('run-validation.js',['--plan',f.planPath,'--output-dir',f.output]).status,0);
 const prior=join(f.output,'receipt.json'),bytes=readFileSync(prior,'utf8'),service=f.env.AUTH_RECEIPTS_SOURCE;
 writeFileSync(join(service,'README.md'),'# Source-only documentation revision\n');execFileSync('git',['add','.'],{cwd:service,stdio:'pipe'});execFileSync('git',['-c','user.name=Fixture','-c','user.email=fixture@example.invalid','commit','-qm','documentation only'],{cwd:service,stdio:'pipe'});
 const next=join(f.root,'auth-next.json'),output=join(f.root,'auth-next-results');assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',next]).status,0);
 const nextPlan=JSON.parse(readFileSync(next,'utf8'));assert.notEqual(nextPlan.authProvenance.sources.service.revision,f.plan().authProvenance.sources.service.revision);
 assert.equal(f.run('run-validation.js',['--plan',next,'--output-dir',output,'--reuse-receipt',prior]).status,0);
 const reused=JSON.parse(readFileSync(join(output,'receipt.json'),'utf8'));assert.equal(reused.checks.find(check=>check.args.includes('test:owner-auth-source')).outcome,'reused');
 writeFileSync(join(service,'src','native-browser-receiver.mjs'),'// changed source input\n');
 const changed=join(f.root,'auth-changed.json'),changedOutput=join(f.root,'auth-changed-results');assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',changed]).status,0);
 assert.equal(f.run('run-validation.js',['--plan',changed,'--output-dir',changedOutput,'--reuse-receipt',join(output,'receipt.json')]).status,0);
 const ran=JSON.parse(readFileSync(join(changedOutput,'receipt.json'),'utf8'));assert.equal(ran.checks.find(check=>check.args.includes('test:owner-auth-source')).outcome,'executed');assert.equal(readFileSync(prior,'utf8'),bytes);
 assert.equal(f.run('assess-validation.mjs',['--plan',changed,'--receipt',join(changedOutput,'receipt.json'),'--json']).status,0);
});

test('failed or interrupted batches and legacy evidence cannot supply reuse',t=>{
 const f=fixture(t);assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',f.planPath]).status,0);assert.equal(f.run('run-validation.js',['--plan',f.planPath,'--output-dir',f.output]).status,0);
 const original=readFileSync(join(f.output,'receipt.json'),'utf8');
 for(const state of ['failure','running','pending','legacy']){
  const candidate=JSON.parse(original);if(state==='legacy')candidate.schemaVersion='agent-browser.local-validation.v1';else candidate.state=state;
  const path=join(f.root,`${state}.json`),output=join(f.root,`${state}-results`);writeFileSync(path,JSON.stringify(candidate));
  const run=f.run('run-validation.js',['--plan',f.planPath,'--output-dir',output,'--reuse-receipt',path]);assert.equal(run.status,1);assert.match(run.stderr,/validation_reuse_receipt_unqualified/);assert.equal(existsSync(output),false);
 }
 assert.equal(readFileSync(join(f.output,'receipt.json'),'utf8'),original);
});

test('changed timeout requirements and installed dependency bytes invalidate reuse',t=>{
 const f=fixture(t);const dependency=join(f.repo,'node_modules','synthetic','index.js');mkdirSync(join(dependency,'..'),{recursive:true});writeFileSync(dependency,'// dependency first\n');
 assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',f.planPath]).status,0);assert.equal(f.run('run-validation.js',['--plan',f.planPath,'--output-dir',f.output]).status,0);
 const next=join(f.root,'new-budget.json'),output=join(f.root,'new-budget-results');assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--timeout-ms','1000','--output',next]).status,0);
 assert.equal(f.run('run-validation.js',['--plan',next,'--output-dir',output,'--reuse-receipt',join(f.output,'receipt.json')]).status,0);
 assert.ok(JSON.parse(readFileSync(join(output,'receipt.json'),'utf8')).checks.every(check=>check.outcome==='executed'));
 writeFileSync(dependency,'// dependency changed\n');const after=join(f.root,'new-dependency.json'),afterOutput=join(f.root,'new-dependency-results');
 assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--timeout-ms','1000','--output',after]).status,0);assert.equal(f.run('run-validation.js',['--plan',after,'--output-dir',afterOutput,'--reuse-receipt',join(output,'receipt.json')]).status,0);
 assert.ok(JSON.parse(readFileSync(join(afterOutput,'receipt.json'),'utf8')).checks.every(check=>check.outcome==='executed'));
});


test('PNPM operational timestamp updates preserve qualification but settings remain bound',t=>{
 const f=fixture(t),metadata=join(f.repo,'node_modules','.pnpm-workspace-state-v1.json');mkdirSync(join(metadata,'..'),{recursive:true});
 writeFileSync(metadata,JSON.stringify({lastValidatedTimestamp:1,settings:{nodeLinker:'isolated',storeDir:'synthetic-store'}}));f.env.FIXTURE_MODE='touch-pnpm';
 assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',f.planPath]).status,0);
 const ran=f.run('run-validation.js',['--plan',f.planPath,'--output-dir',f.output]);assert.equal(ran.status,0,ran.stderr);assert.equal(f.receipt().sourceUnchanged,true);
 assert.ok(JSON.parse(readFileSync(metadata,'utf8')).lastValidatedTimestamp>1);
 const prior=join(f.output,'receipt.json'),priorBytes=readFileSync(prior,'utf8'),next=join(f.root,'pnpm-next.json'),output=join(f.root,'pnpm-next-results');
 assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',next]).status,0);assert.equal(f.run('run-validation.js',['--plan',next,'--output-dir',output,'--reuse-receipt',prior]).status,0);
 assert.ok(JSON.parse(readFileSync(join(output,'receipt.json'),'utf8')).checks.every(check=>check.outcome==='reused'));
 const semantic=JSON.parse(readFileSync(metadata,'utf8'));semantic.settings.nodeLinker='hoisted';writeFileSync(metadata,JSON.stringify(semantic));
 const changed=join(f.root,'pnpm-changed.json'),changedOutput=join(f.root,'pnpm-changed-results');assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',changed]).status,0);
 assert.equal(f.run('run-validation.js',['--plan',changed,'--output-dir',changedOutput,'--reuse-receipt',join(output,'receipt.json')]).status,0);
 assert.ok(JSON.parse(readFileSync(join(changedOutput,'receipt.json'),'utf8')).checks.every(check=>check.outcome==='executed'));
 assert.equal(readFileSync(prior,'utf8'),priorBytes);
});


test('reuse requires complete source provenance and unambiguous prior check custody',t=>{
 const f=fixture(t);assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',f.planPath]).status,0);assert.equal(f.run('run-validation.js',['--plan',f.planPath,'--output-dir',f.output]).status,0);
 const bytes=readFileSync(join(f.output,'receipt.json'),'utf8');
 for(const kind of ['source','plan','duplicate']){
  const prior=JSON.parse(bytes);if(kind==='source')delete prior.source;else if(kind==='plan')delete prior.planDigest;else prior.checks.push(prior.checks[0]);
  const file=join(f.root,`${kind}-prior.json`),output=join(f.root,`${kind}-output`);writeFileSync(file,JSON.stringify(prior));
  const ran=f.run('run-validation.js',['--plan',f.planPath,'--output-dir',output,'--reuse-receipt',file]);assert.equal(ran.status,1,kind);assert.match(ran.stderr,/validation_reuse_receipt_unqualified/);assert.equal(existsSync(output),false);
 }
});


test('linked Next workspace build output is excluded while source and runtime dependencies stay bound',t=>{
 const f=fixture(t);
 writeFileSync(join(f.repo,'docs','package.json'),JSON.stringify({name:'synthetic-docs',scripts:{build:'next build'}}));
 const source=join(f.repo,'docs','source.js');writeFileSync(source,'// original executable source\n');
 const links=join(f.repo,'node_modules','.pnpm','node_modules');mkdirSync(links,{recursive:true});symlinkSync(join(f.repo,'docs'),join(links,'docs'),'dir');
 const dependency=join(f.repo,'docs','node_modules','synthetic','index.js');mkdirSync(join(dependency,'..'),{recursive:true});writeFileSync(dependency,'// original installed dependency\n');
 f.env.FIXTURE_MODE='workspace-build';
 assert.equal(f.run('validation-plan.mjs',['--base',f.base,'--output',f.planPath]).status,0);
 const ran=f.run('run-validation.js',['--plan',f.planPath,'--output-dir',f.output]);assert.equal(ran.status,0,ran.stderr);
 assert.equal(f.receipt().sourceUnchanged,true);assert.equal(f.run('assess-validation.mjs',['--plan',f.planPath,'--receipt',join(f.output,'receipt.json'),'--json']).status,0);
 writeFileSync(source,'// changed executable source\n');assert.equal(f.run('assess-validation.mjs',['--plan',f.planPath,'--receipt',join(f.output,'receipt.json'),'--json']).status,1);
 writeFileSync(source,'// original executable source\n');writeFileSync(dependency,'// changed installed dependency\n');assert.equal(f.run('assess-validation.mjs',['--plan',f.planPath,'--receipt',join(f.output,'receipt.json'),'--json']).status,1);
});
