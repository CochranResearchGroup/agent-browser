#!/usr/bin/env node
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { chromium } from 'playwright';
import { createSyntheticVisualFixtureServer } from './p158-synthetic-visual-fixture.js';
import { verifySyntheticRemoteInput } from './lib/p159-synthetic-remote-input.js';

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
  const result = await verifySyntheticRemoteInput(page, { region, expectedPixelHash, outputDir });
  assert.equal(result.success, true);
  assert.notEqual(result.mouse.sha256, expectedPixelHash);
  assert.equal(result.keyboard.sha256, expectedPixelHash);
  assert.equal(await marker.getAttribute('data-input-state'), 'ready');
  await marker.evaluate((element) => { element.style.pointerEvents = 'none'; });
  await assert.rejects(verifySyntheticRemoteInput(page, { region, expectedPixelHash, outputDir, timeoutMs: 200 }), /acknowledgment missing: mouse/);
  await assert.rejects(verifySyntheticRemoteInput(page, { region, expectedPixelHash: '0'.repeat(64), outputDir }), /baseline pixels do not match/);
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
