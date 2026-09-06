#!/usr/bin/env node
// Mount the actual viewport with React in a disposable Chrome. Only decorative
// UI and store atoms are stubbed; connection planning, effects and iframe keys
// execute unchanged. Provider HTTP is synthetic and never leaves the fixture.
import { createRequire } from 'node:module';
import { readFileSync, existsSync } from 'node:fs';
const root = process.cwd(), require = createRequire(root + '/package.json');
const ts = createRequire(root + '/packages/dashboard/package.json')('typescript'), { chromium } = require('playwright');
const dashboard = root + '/packages/dashboard', sources = [], ids = new Map();
const mock = (s) => s.startsWith('@/components/ui/') || s === 'lucide-react' ? `const R=require('react');module.exports=new Proxy({}, {get:(_,k)=>({children,onSelect})=>R.createElement('div',{onClick:onSelect},children)});` : s === 'jotai/react' ? `exports.useAtomValue=a=>a;exports.useSetAtom=()=>()=>{};` : s === '@/store/sessions' ? `exports.activePortAtom=null;exports.activeSessionNameAtom='fixture';exports.sessionsAtom=[];` : s === '@/store/stream' ? `exports.appendConsoleLogsAtom=null;` : null;
function add(name, from = dashboard + '/src/components/workspace-remote-viewport.tsx') {
    let body = mock(name), file = name;
    if (body === null) {
        if (name.startsWith('@/'))
            file = dashboard + '/src/' + name.slice(2);
        else
            file = createRequire(from).resolve(name);
        if (!existsSync(file)) {
            file = ['.ts', '.tsx', '.js'].map(x => file + x).find(existsSync);
            if (!file)
                throw Error(name);
        }
    }
    if (ids.has(file))
        return ids.get(file);
    let id = sources.length;
    ids.set(file, id);
    sources.push('');
    if (body === null)
        body = readFileSync(file, 'utf8');
    if (/\.tsx?$/.test(file))
        body = ts.transpileModule(body, { fileName: file, compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022, jsx: ts.JsxEmit.ReactJSX, esModuleInterop: true } }).outputText;
    body = body.replace(/require\(["']([^"']+)["']\)/g, (_, dep) => `require(${add(dep, file.startsWith('/') ? file : from)})`);
    sources[id] = `function(module,exports,require){${body}\n}`;
    return id;
}
const component = add(dashboard + '/src/components/workspace-remote-viewport.tsx'), react = add('react'), dom = add('react-dom/client');
const fixture = `
const R=req(${react}),{createRoot}=req(${dom}),{WorkspaceRemoteViewport}=req(${component});
const stream={id:'rdp-1',routeId:'route-1',connectionId:'connection-1',provider:'rdp_gateway',providerMode:'simultaneous_view',frameUrl:location.origin+'/guacamole/#/client/test',externalUrl:location.origin+'/guacamole/#/client/test',attachability:{state:'attached_ready'},state:'ready',health:'ready',viewerLeaseIds:[],controlInput:'manual_attached_desktop',readiness:{state:'ready'},remoteReadiness:{state:'ready'},kind:'rdp',transport:'rdp'};
const browser={id:'session:fixture',profileId:'fixture',health:'ready',host:'remote_headed',viewStreams:[stream]};
const selected={browser,stream,streamChoices:[stream],streamChoiceKeys:['rdp-1'],canView:true,canControl:false,authority:{lifecycle:{live:true}},tabSelection:{tab:null,tabIndex:null,recoveredFromStaleSelection:false,staleSelectionId:null},readiness:{status:'ready',recoveryAction:null}};
const projection={selected,candidates:[selected],tiles:[]};
window.leaseRequested=false;window.leaseCompleted=false;window.failures=[];
window.fetch=async (url,options={})=>{
 const u=new URL(String(url),location.href),body=options.body?JSON.parse(options.body.startsWith('{')?options.body:'{}'):{};
 let value={};
 if(body.action==='service_viewer_lease_request'){window.leaseRequested=true;await new Promise(r=>window.finishLease=r);window.leaseCompleted=true;value={success:true,data:{}};}
 else if(u.pathname.endsWith('/api/tokens'))value={authToken:'synthetic'};
 else if(u.pathname.endsWith('/activeConnections'))value={};
 else if(u.pathname==='/api/guacamole-primary-claim')value={granted:true,claimId:'claim',revision:'revision'};
 else if(u.pathname.includes('failure'))window.failures.push(body);
 return new Response(JSON.stringify(value),{status:200,headers:{'content-type':'application/json'}});
};
createRoot(document.getElementById('root')).render(R.createElement(WorkspaceRemoteViewport,{fallback:'fallback',projection,onRefresh:async()=>{},onSelectStream:()=>{}}));
`;
const bundle = `var process={env:{NODE_ENV:'development'}};const mods=[${sources.join(',')}],cache={};function req(id){if(cache[id])return cache[id].exports;const m=cache[id]={exports:{}};mods[id](m,m.exports,req);return m.exports;}\n${fixture}`;
const browser = await chromium.launch({ executablePath: process.env.AGENT_BROWSER_TEST_BROWSER_EXECUTABLE || '/opt/google/chrome/chrome', headless: true, args: ['--no-sandbox'] });
try {
    const page = await browser.newPage();
    const pageErrors = [];
    page.on('pageerror', e => pageErrors.push(e.message));
    await page.route('**/*', route => route.abort());
    await page.route('http://p159.test/**', async (route) => { const u = new URL(route.request().url()); await route.fulfill({ contentType: 'text/html', body: u.pathname === '/guacamole/' ? '<html><body>synthetic remote frame</body></html>' : '<html><body><div id="root"></div></body></html>' }); });
    await page.goto('http://p159.test/?view=workspace:view&browser=session:fixture&session=fixture');
    await page.addScriptTag({ content: bundle });
    await page.waitForFunction(() => window.leaseRequested, {}, { timeout: 5000 });
    await page.frameLocator('iframe').getByText('synthetic remote frame').waitFor({ timeout: 5000 });
    await page.evaluate(() => { window.originalFrame = document.querySelector('iframe'); window.finishLease(); });
    await page.waitForFunction(() => document.body.innerText.includes('Reconnected the service-owned observer lease'), {}, { timeout: 5000 });
    const result = await page.evaluate(() => ({ sameFrame: window.originalFrame === document.querySelector('iframe'), originalConnected: window.originalFrame.isConnected, frameCount: document.querySelectorAll('iframe').length, leaseCompleted: window.leaseCompleted }));
    console.log(JSON.stringify(result));
    if (!result.sameFrame || !result.originalConnected)
        throw Error('Observer lease acknowledgement detached the original remote-view iframe');
    await page.getByText('Reload view', { exact: true }).click();
    await page.waitForFunction(() => window.originalFrame !== document.querySelector('iframe'), {}, { timeout: 5000 });
    if (pageErrors.length) throw Error(pageErrors.join('\n'));
    console.log('Observer lease preserves the rendered frame; explicit reload replaces it.');
}
catch (e) {
    console.error(await browser.contexts()[0].pages()[0].locator('body').innerText());
    throw e;
}
finally {
    await browser.close();
}
