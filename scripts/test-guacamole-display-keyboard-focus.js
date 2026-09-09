#!/usr/bin/env node
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFileSync } from 'node:fs';
import { chromium } from 'playwright';

const extension = readFileSync(new URL('../cli/assets/workstation/guacamole/extensions/agent-browser-defaults.js', import.meta.url), 'utf8');
const server = createServer((request, response) => {
  response.setHeader('Content-Type', 'text/html');
  if (request.url === '/frame') {
    response.end(`<div class="client-main"><div class="display" style="width:500px;height:300px;background:#125c8e">Remote display</div></div>
      <div class="text-input"><textarea class="target" style="position:fixed;bottom:0;width:1px;height:1px"></textarea></div>
      <button id="settings">Settings</button><script>
      window.angular={module(){return {config(){},run(){}}}};
      window.remoteClicks=0;window.remoteKeys=[];
      // Guacamole.Mouse cancels the native mouse event after forwarding it.
      document.querySelector('.display').addEventListener('mousedown',event=>{event.preventDefault();window.remoteClicks++});
      document.addEventListener('keydown',event=>window.remoteKeys.push(event.key));
      </script><script>${extension}</script>`);
  } else {
    response.end(`<button id="advanced">Advanced</button><iframe style="display:block;width:600px;height:400px" src="http://localhost:${server.address().port}/frame"></iframe>
      <script>window.hostKeys=[];document.querySelector('button').focus();document.addEventListener('keydown',event=>window.hostKeys.push(event.key));</script>`);
  }
});
await new Promise(resolve => server.listen(0, resolve));
let browser;
try {
  browser = await chromium.launch({ executablePath: process.env.AGENT_BROWSER_TEST_BROWSER_EXECUTABLE || '/opt/google/chrome/chrome', headless: true });
  const page = await browser.newPage();
  await page.goto(`http://127.0.0.1:${server.address().port}`);
  const frame = page.frameLocator('iframe');
  await frame.locator('.display').waitFor();
  assert.equal(await page.evaluate(() => document.activeElement.id), 'advanced', 'loading must preserve host focus');
  await frame.locator('.display').dispatchEvent('mousedown');
  assert.equal(await page.evaluate(() => document.activeElement.id), 'advanced', 'synthetic mouse events cannot claim keyboard focus');
  await frame.locator('.display').click();
  assert.equal(await frame.locator('body').evaluate(() => window.remoteClicks), 2, 'native cancellation must preserve forwarding');
  await page.keyboard.press('Enter');
  assert.deepEqual(await frame.locator('body').evaluate(() => window.remoteKeys), ['Enter'], 'the key after a trusted display click belongs to the remote frame');
  assert.deepEqual(await page.evaluate(() => window.hostKeys), [], 'the dashboard must not act on that key');
  await page.locator('#advanced').click();
  await page.keyboard.press('Enter');
  assert.deepEqual(await page.evaluate(() => window.hostKeys), ['Enter'], 'explicit host interaction returns focus normally');
  console.log('Guacamole cross-origin display keyboard focus checks passed; no RDP claim.');
} finally {
  await browser?.close();
  await new Promise(resolve => server.close(resolve));
}
