import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync,chmodSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
import {execFileSync,spawnSync} from 'node:child_process';

const gate=resolve('scripts/dev/check-login-workflow.mjs');
const progress=resolve('packages/client/src/login-progress.js');
function fixture(t) {
  const root=mkdtempSync(join(tmpdir(),'login-gate-'));t.after(()=>rmSync(root,{recursive:true,force:true}));
  const git=args=>execFileSync('git',['-c','core.hooksPath=/dev/null',...args],{cwd:root,encoding:'utf8',stdio:'pipe'}).trim();
  git(['init']);git(['config','user.name','Fixture']);git(['config','user.email','fixture@example.invalid']);
  writeFileSync(join(root,'package.json'),JSON.stringify({type:'module',scripts:{'test:login-workflow':'node scripts/test-login-progress.mjs'}}));
  writeFileSync(join(root,'.gitignore'),'node_modules/\n');
  execFileSync('pnpm',['install','--lockfile-only','--ignore-scripts','--offline'],{cwd:root,stdio:'pipe'});
  writeFileSync(join(root,'README.md'),'# Fixture\n');git(['add','.']);git(['commit','-m','base']);
  const base=git(['rev-parse','HEAD']);
  return {root,git,base,run:()=>spawnSync(process.execPath,[gate,'--base',base,'--head','HEAD'],{cwd:root,encoding:'utf8'})};
}

test('public gate runs selected workflow and rejects the actual false-wait regression',t=>{
  const f=fixture(t);mkdirSync(join(f.root,'packages/client/src'),{recursive:true});
  writeFileSync(join(f.root,'packages/client/src/login-progress.js'),'// selected public contract\n');
  const contract=expected=>`import assert from 'node:assert/strict';import {summarizeLoginProgress} from ${JSON.stringify(progress)};assert.equal(summarizeLoginProgress({state:'owner_review'}).waitingForUser,${expected});\n`;
  mkdirSync(join(f.root,'scripts'));writeFileSync(join(f.root,'scripts/test-login-progress.mjs'),contract(false));f.git(['add','.']);f.git(['commit','-m','working contract']);
  const green=f.run();assert.equal(green.status,0,green.stderr+green.stdout);
  writeFileSync(join(f.root,'scripts/test-login-progress.mjs'),contract(true));f.git(['add','.']);f.git(['commit','-m','deliberate false-wait regression']);
  assert.equal(f.run().status,1);
});

test('documentation stays in its lane, while dirty source and unknown coverage hold publication',t=>{
  const f=fixture(t);writeFileSync(join(f.root,'README.md'),'# Documentation only\n');f.git(['add','.']);f.git(['commit','-m','docs']);
  const docs=f.run();assert.equal(docs.status,0,docs.stderr);assert.deepEqual(JSON.parse(docs.stdout.trim()).checks,[]);
  mkdirSync(join(f.root,'packages/client/src'),{recursive:true});writeFileSync(join(f.root,'packages/client/src/login-progress.js'),'// changed\n');f.git(['add','.']);f.git(['commit','-m','source']);
  writeFileSync(join(f.root,'README.md'),'# Dirty\n');
  const dirty=f.run();assert.equal(dirty.status,1);assert.match(dirty.stderr,/login_gate_source_not_clean_current_head/);
  f.git(['add','.']);f.git(['commit','-m','save docs']);writeFileSync(join(f.root,'unknown-new-file'),'unclassified');f.git(['add','.']);f.git(['commit','-m','unknown']);
  const unknown=f.run();assert.equal(unknown.status,1);assert.match(unknown.stderr,/login_gate_coverage_unresolved/);
});

test('installed worktree-local hook rejects a failing contract before a real local push',t=>{
  const f=fixture(t),remote=mkdtempSync(join(tmpdir(),'login-gate-remote-'));t.after(()=>rmSync(remote,{recursive:true,force:true}));
  execFileSync('git',['init','--bare',remote],{stdio:'pipe'});
  f.git(['remote','add','origin',remote]);f.git(['push','origin','HEAD:refs/heads/main']);
  mkdirSync(join(f.root,'scripts'));mkdirSync(join(f.root,'packages/client/src'),{recursive:true});
  writeFileSync(join(f.root,'packages/client/src/login-progress.js'),'// selected contract\n');
  const contract=expected=>`import assert from 'node:assert/strict';import {summarizeLoginProgress} from ${JSON.stringify(progress)};assert.equal(summarizeLoginProgress({state:'owner_review'}).waitingForUser,${expected});\n`;
  writeFileSync(join(f.root,'scripts/test-login-progress.mjs'),contract(false));f.git(['add','.']);f.git(['commit','-m','green']);
  const hooks=join(f.root,'.git','qualified-hooks');mkdirSync(hooks);
  const hook=join(hooks,'pre-push');writeFileSync(hook,`#!/bin/sh\nexec node '${gate}'\n`);chmodSync(hook,0o700);
  f.git(['config','extensions.worktreeConfig','true']);f.git(['config','--worktree','core.hooksPath',hooks]);
  const push=()=>spawnSync('git',['push','origin','HEAD:refs/heads/feature'],{cwd:f.root,encoding:'utf8'});
  assert.equal(push().status,0);
  writeFileSync(join(f.root,'scripts/test-login-progress.mjs'),contract(true));f.git(['add','.']);f.git(['commit','-m','red']);
  const result=push();assert.notEqual(result.status,0);assert.match(result.stderr,/login_workflow_check_failed/);
});

test('package-only workflow command changes cannot escape the publication gate',t=>{
  const f=fixture(t),path=join(f.root,'package.json');
  const pkg=JSON.parse(readFileSync(path,'utf8'));
  pkg.scripts['test:login-workflow']='node -e "process.exit(7)"';
  pkg.scripts['test:login-workflow-gate']='node -e "process.exit(0)"';
  writeFileSync(path,JSON.stringify(pkg));f.git(['add','.']);f.git(['commit','-m','broken workflow command only']);
  const result=f.run();assert.equal(result.status,1);assert.match(result.stderr,/login_workflow_check_failed/);
});
