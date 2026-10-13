// Actual viewport requests forwarded into the task-local native selection fixture.
const fs = require('node:fs');
const http = require('node:http');
const assert = require('node:assert/strict');
const { chromium } = require('playwright');
const config = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
(async () => {
    const server = http.createServer((req, res) => { const bundle = req.url === '/bundle.js'; res.setHeader('Content-Type', bundle ? 'application/javascript' : 'text/html; charset=utf-8'); res.end(bundle ? fs.readFileSync(config.bundle) : `<div id="root"></div><script>window.P237_UI_CONFIG=${JSON.stringify(config)}</script><script src="/bundle.js"></script>`); });
    await new Promise(r => server.listen(0, '127.0.0.1', r));
    let browser;
    let held;
    let requests = [];
    let replies = [];
    let errors = [];
    try {
        process.env.TMPDIR = config.root + '/ui-temporary';
        fs.mkdirSync(process.env.TMPDIR, { recursive: true });
        browser = await chromium.launch({ executablePath: config.chrome, headless: true, timeout: 10000, args: ['--disable-background-networking', '--p237-ui-ordering-fixture'] });
        const ownershipSession = await browser.newBrowserCDPSession();
        const ownedInfo = await ownershipSession.send('SystemInfo.getProcessInfo');
        await ownershipSession.detach();
        const owner = ownedInfo.processInfo.find(item => item.type === 'browser');
        assert.ok(owner);
        const ownerStat = fs.readFileSync(`/proc/${owner.id}/stat`, 'utf8');
        const ownerFields = ownerStat.slice(ownerStat.lastIndexOf(')') + 2).split(' ');
        fs.writeFileSync(config.root + '/ui-selection-owned-session.json', JSON.stringify({ pid: owner.id, start: ownerFields[19], session: Number(ownerFields[3]), profileRoot: process.env.TMPDIR }));
        const page = await browser.newPage();
        page.setDefaultTimeout(5000);
        page.setDefaultNavigationTimeout(5000);
        page.on('pageerror', e => errors.push(e.message));
        const origin = `http://127.0.0.1:${server.address().port}`;
        async function forward(route) { const request = route.request(); const headers = { ...request.headers(), cookie: config.cookie }; delete headers.origin; const response = await route.fetch({ url: config.origin + '/api/service/request', headers }); const json = await response.json(); replies.push(json); assert.equal(json.success, true, JSON.stringify(json)); await route.fulfill({ response }); }
        await page.route('**/*', async (route) => {
            const req = route.request();
            const url = new URL(req.url());
            if (url.origin === origin && (url.pathname === '/' || url.pathname === '/bundle.js'))
                return route.continue();
            let body;
            try {
                body = req.postDataJSON();
            }
            catch { }
            if (body?.action === 'view_focus') {
                requests.push(body);
                if (body.params.targetId === config.tabs[0][0]) {
                    held = route;
                    return;
                }
                return forward(route).catch(async (error) => { errors.push(error.message); await route.fulfill({ status: 500, json: { success: false, error: error.message } }); });
            }
            if (body?.action === 'service_controller_lease_takeover')
                return route.fulfill({ json: { success: true, data: { viewerLease: { id: config.controllerLeaseId, state: 'controlling', viewerRole: 'controller' } } } });
            return route.fulfill({ json: { success: true, data: {} } });
        });
        await page.goto(origin + `/?view=workspace:control&browser=${config.browserId}&tab=alice`);
        fs.writeFileSync(config.root + '/ui-selection-progress.json', JSON.stringify({ stage: 'loaded', errors, text: await page.locator('body').innerText() }));
        await page.getByRole('button', { name: 'Advanced connection controls', exact: true }).click();
        await page.getByRole('menuitem', { name: 'Take control', exact: true }).click();
        for (let i = 0; i < 100 && !held; i++)
            await new Promise(r => setTimeout(r, 25));
        assert.ok(held, JSON.stringify({ stage: 'first UI request', requests, errors, text: await page.locator('body').innerText() }));
        await page.evaluate(() => window.select('bob'));
        for (let i = 0; i < 10 && requests.length < 2; i++)
            await new Promise(r => setTimeout(r, 25));
        const transportOverlap = requests.length === 2;
        if (transportOverlap) {
            for (let i = 0; i < 200 && replies.length < 1; i++)
                await new Promise(r => setTimeout(r, 25));
            assert.equal(replies.length, 1, JSON.stringify({ stage: 'newer request', requests, errors }));
        }
        await forward(held);
        for (let i = 0; i < 200 && replies.length < 2; i++)
            await new Promise(r => setTimeout(r, 25));
        assert.equal(replies.length, 2, JSON.stringify({ stage: 'completed requests', requests, errors }));
        await page.waitForTimeout(100);
        assert.deepEqual(requests.map(r => r.params.targetId), [config.tabs[0][0], config.tabs[1][0]]);
        assert.deepEqual(errors, []);
        const report = { transportOverlap, requests: requests.map(r => ({ browserId: r.browserId, params: r.params })), replies: replies.map(r => ({ success: r.success })), text: await page.locator('body').innerText() };
        fs.writeFileSync(config.root + '/ui-selection-observation.json', JSON.stringify(report, null, 2));
        console.log(JSON.stringify(report));
    }
    finally {
        try {
            if (browser) {
                let identities = [];
                try {
                    const session = await browser.newBrowserCDPSession();
                    const info = await session.send('SystemInfo.getProcessInfo');
                    await session.detach();
                    identities = info.processInfo.flatMap(process => { try {
                        const text = fs.readFileSync(`/proc/${process.id}/stat`, 'utf8');
                        return [{ pid: process.id, start: text.slice(text.lastIndexOf(')') + 2).split(' ')[19] }];
                    }
                    catch {
                        return [];
                    } });
                }
                finally {
                    await browser.close();
                }
                const remaining = identities.filter(identity => { try {
                    const text = fs.readFileSync(`/proc/${identity.pid}/stat`, 'utf8');
                    const fields = text.slice(text.lastIndexOf(')') + 2).split(' ');
                    return fields[0] !== 'Z' && fields[19] === identity.start;
                }
                catch {
                    return false;
                } });
                fs.writeFileSync(config.root + '/ui-selection-cleanup.json', JSON.stringify({ identities, remaining }, null, 2));
                assert.deepEqual(remaining, []);
            }
        }
        finally {
            await new Promise(r => server.close(r));
        }
    }
})().catch(e => { console.error(e); process.exitCode = 1; });
