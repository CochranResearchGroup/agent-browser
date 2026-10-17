const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const http = require('node:http');
const {chromium} = require('playwright');
const {buildWorkspaceViewportFixture} = require('./lib/workspace-viewport-browser-fixture.cjs');
(async()=>{
  const root=path.resolve(__dirname,'..');
  const artifacts=fs.mkdtempSync(path.join(os.tmpdir(),'rail-observation-refresh-'));
  await buildWorkspaceViewportFixture(root,path.join(root,'scripts/fixtures/native-handoff-policy-refresh.tsx'),artifacts);
  let token=1000, held=false, statusReads=0;
  let notifyHeld;
  const heldAccepted = new Promise(resolve=>{notifyHeld=resolve;});
  const releases=[];
  let releaseResources;
  const resources=new Promise(resolve=>{releaseResources=resolve;});
  const methods=[];
  const server=http.createServer(async(req,res)=>{
    methods.push(req.method);
    res.setHeader('Content-Type',req.url==='/bundle.js'?'application/javascript':'application/json');
    if(req.url.startsWith('/api/service/status?')){
      statusReads++;
      if(held){notifyHeld();await new Promise(resolve=>releases.push(resolve));}
      return res.end(JSON.stringify({success:true,data:{service_state:{browsers:{},sessions:{},tabs:{},jobs:{}},browserRuntime:{workspace:{
        schemaVersion:'agent-browser.managed-workspace.v1',source:'managed-owner',observedAtMs:token,
        browsers:{b:{id:'b',profileId:'p',pid:42,desktop:null,activeSessionIds:['s'],state:'reported',presence:{state:'live'}}},
        sessions:{s:{id:'s',name:'task',profileId:'p',browserId:'b'}},
        tabs:{t:{id:'t',targetId:'target',browserId:'b',sessionId:'s',presence:{state:'live'}}}
      }}}}));
    }
    if(req.url==='/api/service/resources'){await resources;return res.end(JSON.stringify({success:true,data:{resources:[]}}));}
    if(req.url==='/bundle.js')return res.end(fs.readFileSync(path.join(artifacts,'bundle.js')));
    res.setHeader('Content-Type','text/html; charset=utf-8');
    res.end('<div id="root"></div><script src="/bundle.js"></script>');
  });
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  let browser;
  try{
    browser=await chromium.launch({executablePath:process.env.AGENT_BROWSER_TEST_CHROME||'/usr/bin/google-chrome',headless:true});
    const page=await browser.newPage();
    const errors=[];page.on('pageerror',error=>errors.push(error.message));
    await page.clock.install({time:new Date('2040-01-01T00:00:00Z')});
    const start=performance.now();
    await page.goto(`http://127.0.0.1:${server.address().port}`);
    await page.waitForFunction(()=>document.querySelector('[aria-label="Live tab count"]')?.textContent==='1',null,{timeout:3000});
    const initialLiveMs=performance.now()-start;
    assert.ok(initialLiveMs < 3000, `live inventory publication took ${initialLiveMs}ms while resources were held`);
    assert.equal(statusReads,1,'rail must publish status while resource diagnostics remain held');
    assert.equal(await page.getByLabel('Presence current').innerText(),'true','client wall-clock skew must not invalidate an owner observation');
    held=true;
    await page.clock.runFor(3100);
    await page.waitForFunction(()=>document.querySelector('[role="status"]')?.textContent==='Loading');
    await heldAccepted;
    assert.equal(statusReads,2,'three-second rail poll must not reuse the old ten-second document cache');
    await page.clock.runFor(28000);
    await page.waitForFunction(()=>document.querySelector('[aria-label="Live tab count"]')?.textContent==='0');
    assert.equal(await page.getByLabel('Presence current').innerText(),'false','a held request must not keep a stale live observation forever');
    held=false;token=2000;releases.splice(0).forEach(resolve=>resolve());releaseResources();
    await page.getByRole('button',{name:'Refresh handoffs'}).click();
    await page.waitForFunction(()=>document.querySelector('[aria-label="Live tab count"]')?.textContent==='1');
    assert.equal(await page.getByLabel('Presence current').innerText(),'true');
    assert.ok(methods.every(method=>method==='GET'),'inventory refresh must stay read-only');
    assert.deepEqual(errors,[]);
    const receipt={success:true,initialLiveMs,statusReads,resourcesHeldAtInitialPublication:true,staleExpiredDuringHeldPoll:true,wallClockYear:2040,seam:'real workspace hook and coordinator over synthetic HTTP'};
    fs.writeFileSync(path.join(artifacts,'receipt.json'),JSON.stringify(receipt,null,2));
    console.log(JSON.stringify({...receipt,artifacts}));
  }finally{
    releases.splice(0).forEach(resolve=>resolve());releaseResources();
    if(browser){
      const session=await browser.newBrowserCDPSession();const {processInfo}=await session.send('SystemInfo.getProcessInfo');await session.detach();
      const identities=processInfo.flatMap(process=>{try{const raw=fs.readFileSync(`/proc/${process.id}/stat`,'utf8');return [{pid:process.id,start:raw.slice(raw.lastIndexOf(')')+2).split(' ')[19]}];}catch{return [];}});
      await browser.close();
      const remaining=identities.filter(identity=>{try{const raw=fs.readFileSync(`/proc/${identity.pid}/stat`,'utf8');const fields=raw.slice(raw.lastIndexOf(')')+2).split(' ');return fields[0]!=='Z'&&fields[19]===identity.start;}catch{return false;}});
      fs.writeFileSync(path.join(artifacts,'cleanup.json'),JSON.stringify({identities,remaining},null,2));assert.deepEqual(remaining,[]);
    }
    await new Promise(resolve=>server.close(resolve));
  }
})().catch(error=>{console.error(error);process.exitCode=1;});
