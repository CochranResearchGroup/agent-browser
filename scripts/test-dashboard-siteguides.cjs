const assert=require('node:assert/strict'),fs=require('node:fs'),os=require('node:os'),path=require('node:path'),http=require('node:http');
const {chromium}=require('playwright');const {buildWorkspaceViewportFixture}=require('./lib/workspace-viewport-browser-fixture.cjs');
(async()=>{
 const root=path.resolve(__dirname,'..'),artifacts=fs.mkdtempSync(path.join(os.tmpdir(),'siteguides-mounted-'));let browser,server;
 try{
  await buildWorkspaceViewportFixture(root,path.join(root,'scripts/fixtures/siteguides-library.tsx'),artifacts);
  server=http.createServer((req,res)=>{res.setHeader('Content-Type',req.url==='/bundle.js'?'application/javascript':'text/html');res.end(req.url==='/bundle.js'?fs.readFileSync(path.join(artifacts,'bundle.js')):'<div id="root"></div><div id="native-result"></div><div id="agent-result"></div><script src="/bundle.js"></script>');});await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  browser=await chromium.launch({executablePath:'/opt/google/chrome/chrome',headless:true});const page=await browser.newPage();let agentCalls=0;
  await page.route('**/api/dashboard/siteguides',async route=>{const request=route.request().postDataJSON();
   let data;
   if(request.operation==='list')data={guides:[{guideId:'sales',rootGuideId:'sales',name:'Daily sales',purpose:'Verify requested dates and CSV content',allowedOrigins:['https://synthetic.invalid'],sourceTaskId:'task',review:'not_reviewed',repeat:'not_validated',source:{state:'unavailable',replayable:false},ordinaryReuseAllowed:false,history:[]}]};
   else if(request.operation==='inspect_source'){assert.equal(request.taskId,'task');data={recordingId:'a'.repeat(32),video:{sha256:'b'.repeat(64)},narration:{sha256:'c'.repeat(64)},gaps:[{reason:'operator_pause'}]};}
   else if(request.operation==='import_narration')data={taskId:'task',recordingId:'a'.repeat(32),video:{sha256:'b'.repeat(64)},narration:{sha256:'c'.repeat(64)},speechSource:{artifactId:'saved-original',revision:'d'.repeat(64),durationMs:1000}};
   else if(request.operation==='transcribe')data={operationId:'speech-original',request:{...request.range,artifactId:'saved-original',revision:'d'.repeat(64)}};
   else if(request.operation==='read_transcript')data={status:'available',operationId:'speech-original',transcript:{text:'Open Reports and verify requested dates.'}};
   else if(request.operation==='agent_packet'){agentCalls++;assert.equal(request.rootGuideId,request.operationId);data={...request,state:'awaiting_agent',source:{recordingId:'a'.repeat(32)}};}
   else if(request.operation==='lookup_operation'||request.operation==='resume_agent')data={status:'not_observed',retryAuthority:'not_inferred'};
   else throw Error('Unexpected mutation '+request.operation);
   await route.fulfill({json:{success:true,data}});
  });
  const url=`http://127.0.0.1:${server.address().port}`;await page.goto(url);
  assert(await page.getByRole('button',{name:'Open native Remote View'}).isDisabled());await page.getByText('Select a browser task to teach this site.').waitFor();
  await page.goto(url+'?selected');await page.getByText('Intended identity does not establish authentication freshness.',{exact:false}).waitFor();
  await page.getByRole('button',{name:'Open native Remote View'}).click();assert.equal(await page.locator('#native-result').textContent(),'native-owner');
  await page.getByLabel('Search SiteGuides').fill('CSV');await page.getByRole('button',{name:'Daily sales',exact:true}).waitFor();
  await page.getByLabel('Finalized teaching source reference').fill('source-1');await page.getByRole('button',{name:'Check finalized evidence'}).click();await page.getByText('Gaps: 1.',{exact:false}).waitFor();
  await page.getByRole('button',{name:'Import selected narration into Speech'}).click();
  await page.getByRole('button',{name:'Transcribe narration 0–1000 ms'}).click();
  await page.getByRole('button',{name:'Read original transcript',exact:true}).click();
  await page.getByText('Open Reports and verify requested dates.',{exact:true}).waitFor();
  await page.getByRole('button',{name:'Ask agent to interpret exact evidence'}).click();await page.getByText('Awaiting ordinary agent interpretation.',{exact:false}).waitFor();assert.equal(agentCalls,1);
  const packet=JSON.parse(await page.locator('#agent-result').textContent());assert.equal(packet.context.profileId,'work');assert.equal(packet.source.recordingId,'a'.repeat(32));assert.equal(packet.agentRequest.sourceKey,'source-1');assert.equal(packet.agentRequest.segments[0].operationId,'speech-original');assert.equal('profileId' in packet.agentRequest,false);
  await page.getByRole('button',{name:'Read original agent publication'}).click();await page.getByText('Original agent publication not observed.',{exact:false}).waitFor();assert.equal(agentCalls,1);
  console.log('Mounted SiteGuides: task/profile context, native owner callback, purpose search, finalized evidence, awaiting agent and original-operation readback without replay.');
 }finally{await browser?.close();if(server)await new Promise(resolve=>server.close(resolve));fs.rmSync(artifacts,{recursive:true,force:true});}
})().catch(error=>{console.error(error);process.exitCode=1;});
