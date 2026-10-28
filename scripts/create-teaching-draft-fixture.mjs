/** Reproduce synthetic page frames; this never visits a real portal or records a person. */
import {chromium} from 'playwright';
import {readFile,mkdir,writeFile} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
const out=resolve(process.argv[2] ?? '');
if(!process.argv[2] || !out.startsWith('/tmp/'))throw Error('Pass an explicit /tmp output directory');
await mkdir(out,{recursive:true,mode:0o700});
const html=await readFile(new URL('./fixtures/teaching-report/demo.html',import.meta.url),'utf8');
const browser=await chromium.launch({executablePath:process.env.AGENT_BROWSER_SMOKE_BROWSER_EXECUTABLE ?? '/usr/bin/google-chrome',headless:true,args:['--no-sandbox']});
const frames=[];
try {
 const context=await browser.newContext({viewport:{width:1280,height:800},acceptDownloads:true});
 const page=await context.newPage();await page.route('**/*',route=>route.fulfill({contentType:'text/html',body:html}));
 await page.goto('https://teaching-report.example.invalid');
 for(const [name,action] of [['choose',null],['dates',()=>page.getByRole('button',{name:'Open report',exact:true}).click()],['verify',()=>page.getByRole('button',{name:'Run report',exact:true}).click()],['export',()=>page.getByRole('button',{name:'Export CSV',exact:true}).click()]]) {
  if(action)await action();
  const path=join(out,name+'.png');await page.screenshot({path});frames.push(path);
 }
 await context.close();
} finally {await browser.close();}
await writeFile(join(out,'frames.txt'),frames.map(path=>`file '${path}'\nduration 3`).join('\n')+`\nfile '${frames.at(-1)}'\n`);
execFileSync('ffmpeg',['-hide_banner','-loglevel','error','-f','concat','-safe','0','-i',join(out,'frames.txt'),'-f','lavfi','-i','sine=frequency=440:duration=12','-t','12','-vf','fps=10','-c:v','libvpx','-b:v','300k','-c:a','libopus','-y',join(out,'demonstration.webm')],{timeout:30000,stdio:'pipe'});
const bytes=await readFile(join(out,'demonstration.webm'));
const artifact={recordingId:'synthetic-daily-sales-474-v1',videoSha256:createHash('sha256').update(bytes).digest('hex'),durationMs:12000,frames:frames.map((path,index)=>({path,startMs:index*3000,endMs:(index+1)*3000})),audio:'synthetic reference tone, no speech or ASR transcript',captions:'visible teaching instructions are video evidence',privateData:false};
await writeFile(join(out,'manifest.json'),JSON.stringify(artifact,null,2)+'\n');console.log(JSON.stringify(artifact));
