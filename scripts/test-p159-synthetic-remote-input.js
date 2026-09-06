#!/usr/bin/env node
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { chromium } from 'playwright';
import { createSyntheticVisualFixtureServer } from './p158-synthetic-visual-fixture.js';
import { verifySyntheticRemoteInput } from './lib/p159-synthetic-remote-input.js';
import { observeRemoteDisplayClip } from './lib/p159-remote-display-geometry.js';

const server = createSyntheticVisualFixtureServer({ port: 0, allowEphemeralTestPort: true });
const address = await server.listen();
const outputDir = mkdtempSync(join(tmpdir(), 'p159-input-test-'));
let browser;
try {
  browser = await chromium.launch({ executablePath: process.env.AGENT_BROWSER_TEST_BROWSER_EXECUTABLE || '/opt/google/chrome/chrome', headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
  await page.setContent(`<iframe style="position:absolute;inset:0;border:0;width:1440px;height:1000px" src="http://127.0.0.1:${address.port}/fixture"></iframe>`);
  const marker = page.frameLocator('iframe').locator('#pixel-marker');
  await marker.waitFor();
  await marker.evaluate(async (element) => { await element.decode(); });
  const region = { x: 300, y: 200, width: 100, height: 80, coordinateSpace: 'remote-view-iframe' };
  const before = await page.screenshot({ clip: { x: 300, y: 200, width: 100, height: 80 } });
  const expectedPixelHash = createHash('sha256').update(before).digest('hex');
  await marker.evaluate((element) => element.click());
  assert.equal(await marker.getAttribute('data-input-state'), 'ready', 'untrusted click must not acknowledge remote input');
  await marker.evaluate((element) => {
    element.style.filter = 'grayscale(1)';
    setTimeout(() => { element.style.filter = ''; }, 500);
  });
  const result = await verifySyntheticRemoteInput(page, { region, expectedPixelHash, outputDir });
  assert.equal(result.success, true);
  assert.notEqual(result.mouse.sha256, expectedPixelHash);
  assert.equal(result.keyboard.sha256, expectedPixelHash);
  assert.equal(await marker.getAttribute('data-input-state'), 'ready');
  // Reproduce a centered, scaled Guacamole desktop, including browser chrome
  // offset. The original fixed iframe crop misses the marker in this geometry.
  const oracle = await page.screenshot({ clip: { x: 300, y: 200, width: 400, height: 100 } });
  const oracleHash = createHash('sha256').update(oracle).digest('hex');
  // This direct-Chrome baseline is not the historical RDP PNG oracle: provider
  // color conversion is outside this geometry test. Never rewrite that oracle.
  await page.locator('iframe').evaluate((frame) => {
    frame.style.cssText = 'position:absolute;left:310px;top:337px;border:0;width:1108px;height:640px';
  });
  await marker.evaluate((element) => {
    const doc = element.ownerDocument;
    const fixture = doc.querySelector('#fixture');
    const display = doc.createElement('div');
    display.className = 'display';
    display.style.cssText = 'position:absolute;left:16px;top:0';
    display.innerHTML = '<div style="position:relative;width:1075.2px;height:604.8px"><div style="position:relative;width:1920px;height:1080px;transform-origin:0 0;transform:scale(.56)"></div></div>';
    fixture.style.position = 'absolute';
    fixture.style.top = '86px';
    display.firstElementChild.firstElementChild.append(fixture);
    doc.body.append(display);
  });
  const displayRegion = { coordinateSpace: 'remote-view-display', x: 240, y: 206,
    width: 960, height: 320, sampleWidth: 400, sampleHeight: 100 };
  const mapped = await observeRemoteDisplayClip(page, displayRegion);
  assert.deepEqual(mapped, { x: 529, y: 492, width: 400, height: 100 });
  const oldCrop = await page.screenshot({ clip: { x: 610, y: 637, width: 400, height: 100 } });
  assert.notEqual(createHash('sha256').update(oldCrop).digest('hex'), oracleHash);
  const mappedInput = await verifySyntheticRemoteInput(page, {
    region: displayRegion, expectedPixelHash: oracleHash, outputDir,
  });
  assert.equal(mappedInput.keyboard.sha256, oracleHash);
  assert.equal(await observeRemoteDisplayClip(page, { ...displayRegion, sampleWidth: 600 }), null,
    'do not shrink the oracle when the rendered marker is too small');
  // Restore the existing direct-iframe cases below.
  await marker.evaluate((element) => {
    const doc = element.ownerDocument;
    const fixture = doc.querySelector('#fixture');
    fixture.style.top = '0';
    doc.body.append(fixture);
    doc.querySelector('.display').remove();
  });
  await page.locator('iframe').evaluate((frame) => {
    frame.style.cssText = 'position:absolute;inset:0;border:0;width:1440px;height:1000px';
  });
  await marker.evaluate((element) => { element.style.pointerEvents = 'none'; });
  await assert.rejects(verifySyntheticRemoteInput(page, { region, expectedPixelHash, outputDir, timeoutMs: 200 }), /acknowledgment missing: mouse/);
  await assert.rejects(verifySyntheticRemoteInput(page, { region, expectedPixelHash: '0'.repeat(64), outputDir, timeoutMs: 200 }), /acknowledgment missing: baseline/);
  await marker.evaluate((element) => {
    element.style.pointerEvents = '';
    element.ownerDocument.addEventListener('keydown', (event) => event.stopImmediatePropagation(), true);
  });
  await assert.rejects(verifySyntheticRemoteInput(page, { region, expectedPixelHash, outputDir, timeoutMs: 200 }), /acknowledgment missing: keyboard/);
  console.log('Synthetic input real-Chrome fixture checks passed; this is not RDP acceptance.');
} finally {
  await browser?.close();
  await server.close();
  rmSync(outputDir, { recursive: true, force: true });
}
