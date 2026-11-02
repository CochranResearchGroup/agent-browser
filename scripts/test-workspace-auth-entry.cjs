const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const http = require('node:http');
const { chromium } = require('playwright');
const { buildWorkspaceViewportFixture } = require('./lib/workspace-viewport-browser-fixture.cjs');
(async () => {
  const root = path.resolve(__dirname, '..');
  const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'p524-auth-gate-'));
  let browser;
  const server = http.createServer((req, res) => {
    res.setHeader('Content-Type', req.url === '/bundle.js' ? 'application/javascript' : 'text/html');
    res.end(req.url === '/bundle.js' ? fs.readFileSync(path.join(scratch, 'bundle.js')) : '<div id="root"></div><script src="/bundle.js"></script>');
  });
  try {
    await buildWorkspaceViewportFixture(root, path.join(root, 'scripts/fixtures/workspace-auth-entry.tsx'), scratch);
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    browser = await chromium.launch({ executablePath: process.env.AGENT_BROWSER_TEST_CHROME || '/usr/bin/google-chrome', headless: true });
    const page = await browser.newPage();
    let authenticationSource='ingress';
    await page.route('**/api/**', route=>{
      const pathname=new URL(route.request().url()).pathname;
      if(pathname==='/api/dashboard-auth/status')return route.fulfill({json:{authenticated:true,user:{username:'operator',displayName:'Operator',role:'superuser',authenticationSource}}});
      return route.fulfill({status:503,json:{success:false,error:'Synthetic source unavailable'}});
    });
    await page.goto(`http://127.0.0.1:${server.address().port}/?browser=selected-task`);
    await page.getByRole('button',{name:'Open browser',exact:true}).waitFor();
    assert.equal(await page.getByRole('button',{name:'End local session',exact:true}).count(),0,'actual ingress entry must not offer local logout');
    assert.equal(new URL(page.url()).searchParams.get('browser'),'selected-task');
    authenticationSource='local';await page.reload();
    await page.getByRole('button',{name:'End local session',exact:true}).waitFor();
    assert.equal(await page.locator('input[type=password]').count(),0);
    console.log('Actual workspace auth entry passed: ingress hides local logout, local session retains it, selection survives.');
  } finally {
    if (browser) await browser.close();
    server.close();
    fs.rmSync(scratch, { recursive: true, force: true });
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
