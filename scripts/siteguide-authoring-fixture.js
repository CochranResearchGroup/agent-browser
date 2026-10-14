import assert from 'node:assert/strict';
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {createPlaybookClient} from '../packages/client/src/playbooks.js';
import {createMcpStdioClient} from './smoke-utils.js';

/** An agent proposes a version, validates it with an ordinary candidate client,
 * and leaves canonical publication to normal reviewed source changes. */
export async function runAuthoringFixture({call,software,taskId,guide,context,directory,appHtml}) {
 const original=readFileSync(join(directory,guide.guideId+'.json'),'utf8');
 await call('browser_command',{taskId,action:'evaluate',params:{script:
  `document.body.innerHTML='<main data-auth-account="fixture"><input id="username"><input id="password" type="password"><button id="submit">Log in</button></main>'`}});
 const options={taskId,guideId:guide.guideId,accountRef:'demo-account',organizationRef:'demo-company'};
 const prepared=await software.prepareLogin(options);assert(prepared.plan,'A frozen action plan is prepared');
 assert.equal((await software.validatePlan({...options,plan:prepared.plan})).reasonCode,'plan_current');
 await call('browser_command',{taskId,action:'evaluate',params:{script:'document.querySelector("#password").type="text"'}});
 assert.equal((await software.validatePlan({...options,plan:prepared.plan})).reasonCode,'prepared_controls_changed');
 assert.equal((await software.prepareLogin(options)).status,'adaptation_required','Never prepare password filling into an unmasked field');
 await call('browser_command',{taskId,action:'evaluate',params:{script:'document.querySelector("#password").type="password"'}});
 await call('browser_command',{taskId,action:'evaluate',params:{script:'document.querySelector("#password").id="password-v2"'}});
 const invalid=await software.validatePlan({...options,plan:prepared.plan});
 assert.equal(invalid.status,'adaptation_required');assert.equal(invalid.reasonCode,'prepared_controls_changed');
 assert.equal(invalid.effectState,'no_effect');
 assert.equal((await software.prepareLogin(options)).status,'adaptation_required','Fresh preparation with the old guide must not bless missing controls');
 const proposal=join(context.tempHome,'guide-proposal');mkdirSync(proposal,{mode:0o700});
 const revised=structuredClone(guide);revised.guideId=guide.guideId+'-v2';
 revised.controls.password='#password-v2';revised.states.find(state=>state.state==='password_required').selector='#password-v2';
 writeFileSync(join(proposal,revised.guideId+'.json'),JSON.stringify(revised),{mode:0o600});
 const candidate=createMcpStdioClient({context:{...context,env:{...context.env,AGENT_BROWSER_SITEGUIDE_DIRECTORY:proposal}},args:['mcp','serve'],onFatal:()=>{}});
 try {
  await candidate.send('initialize',{protocolVersion:'2025-06-18',capabilities:{},clientInfo:{name:'guide-candidate',version:'1'}});candidate.notify('notifications/initialized');
  const reviewer=createPlaybookClient((name,args)=>candidate.send('tools/call',{name,arguments:args}));
  const fresh=await reviewer.prepareLogin({...options,guideId:revised.guideId});
  assert(fresh.plan);assert.equal(fresh.taskId,taskId);assert.notEqual(fresh.plan.guideDigest,prepared.plan.guideDigest);
  assert.equal((await reviewer.validatePlan({...options,guideId:revised.guideId,plan:fresh.plan})).reasonCode,'plan_current');
  assert.equal(readFileSync(join(directory,guide.guideId+'.json'),'utf8'),original,'Canonical guide is unchanged');
  assert(!(await software.listGuides(taskId)).guides.includes(revised.guideId),'Candidate validation is not publication');
 }finally{candidate.close();}
 await call('browser_command',{taskId,action:'evaluate',params:{script:`document.body.innerHTML=${JSON.stringify(appHtml)}`}});
 console.log('Frozen controls rejected after change; fresh versioned guide candidate validated without canonical rewrite');
}
