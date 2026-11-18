#!/usr/bin/env node
import {execFileSync,spawnSync} from 'node:child_process';
import {readFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';

const selector=fileURLToPath(new URL('./select-validation.js',import.meta.url));
const allowed=new Set(['pnpm test:login-workflow','pnpm test:login-workflow-gate']);
const git=args=>execFileSync('git',args,{encoding:'utf8',stdio:['ignore','pipe','pipe']}).trim();
const commit=ref=>git(['rev-parse','--verify',`${ref}^{commit}`]);

try {
  const args=process.argv.slice(2);
  let ranges;
  if(args.length) {
    if(args.length!==4||args[0]!=='--base'||args[2]!=='--head'||[args[1],args[3]].some(ref=>!ref||ref.startsWith('-')||/[\r\n\0]/.test(ref)))throw Error('login_gate_arguments_invalid');
    ranges=[{base:commit(args[1]),head:commit(args[3])}];
  } else {
    ranges=readFileSync(0,'utf8').split('\n').filter(Boolean).flatMap(line=>{
      const fields=line.trim().split(/\s+/);
      if(fields.length!==4||![fields[1],fields[3]].every(sha=>/^[a-f0-9]{40,64}$/.test(sha)))throw Error('login_gate_push_input_invalid');
      if(/^0+$/.test(fields[1]))return [];
      const head=commit(fields[1]);
      return [{head,base:/^0+$/.test(fields[3])?git(['merge-base','origin/main',head]):commit(fields[3])}];
    });
  }
  const selected=new Set();
  for(const {base,head} of ranges) {
    const report=JSON.parse(execFileSync(process.execPath,[selector,'--base',base,'--head',head,'--ci','--json'],{encoding:'utf8',stdio:['ignore','pipe','pipe']}));
    if(!report.coverage?.complete)throw Error('login_gate_coverage_unresolved');
    const checks=report.recommendations.map(check=>check.command).filter(command=>allowed.has(command));
    if(checks.length&&(head!==commit('HEAD')||git(['status','--porcelain'])))throw Error('login_gate_source_not_clean_current_head');
    checks.forEach(command=>selected.add(command));
  }
  const head=commit('HEAD');
  for(const command of selected) {
    const result=spawnSync('pnpm',[command.slice(5)],{stdio:'inherit'});
    if(result.status!==0||result.error||result.signal)throw Error('login_workflow_check_failed');
  }
  if(selected.size&&(commit('HEAD')!==head||git(['status','--porcelain'])))throw Error('login_gate_source_changed');
  console.log(JSON.stringify({qualification:'focused_source_only',sourceHead:head,checks:[...selected],passed:true}));
} catch(error) {
  const known=new Set(['login_gate_arguments_invalid','login_gate_push_input_invalid','login_gate_coverage_unresolved','login_gate_source_not_clean_current_head','login_gate_source_changed','login_workflow_check_failed']);
  console.error(known.has(error.message)?error.message:'login_workflow_gate_unavailable');
  process.exitCode=1;
}
