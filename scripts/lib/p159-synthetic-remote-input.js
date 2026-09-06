import { createHash } from 'node:crypto';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { observeRemoteDisplayClip } from './p159-remote-display-geometry.js';

export const SYNTHETIC_INPUT_PROTOCOL = 'trusted-marker-click-white-enter-reset-v1';
const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');

// Observe rendered pixels only. This never evaluates or changes the remote DOM.
export async function verifySyntheticRemoteInput(page, { region, expectedPixelHash, outputDir, timeoutMs = 10_000 }) {
  const frames = page.locator('iframe');
  if (await frames.count() !== 1) throw new Error('Synthetic input requires exactly one remote frame');
  const box = await frames.first().boundingBox();
  if (region.coordinateSpace !== 'remote-view-display' && (!box || region.coordinateSpace !== 'remote-view-iframe' ||
      region.x < 0 || region.y < 0 || region.width < 1 || region.height < 1 ||
      region.x + region.width > box.width || region.y + region.height > box.height)) {
    throw new Error('Synthetic input region does not fit the remote frame');
  }
  const observeClip = async () => {
    if (region.coordinateSpace === 'remote-view-display') return observeRemoteDisplayClip(page, region);
    const current = await frames.first().boundingBox();
    if (!current || region.x + region.width > current.width || region.y + region.height > current.height) return null;
    return { x: current.x + region.x, y: current.y + region.y, width: region.width, height: region.height };
  };
  let clip;
  const waitForPixels = async (label, accept) => {
    const deadline = Date.now() + timeoutMs;
    let lastBytes;
    do {
      clip = await observeClip();
      if (!clip) {
        await page.waitForTimeout(100);
        continue;
      }
      const bytes = await page.screenshot({ clip });
      lastBytes = bytes;
      if (await accept(bytes)) {
        const file = `remote-input-${label}.png`;
        writeFileSync(join(outputDir, file), bytes);
        return { file, sha256: digest(bytes), clip, observedAt: new Date().toISOString() };
      }
      await page.waitForTimeout(100);
    } while (Date.now() < deadline);
    const file = `remote-input-${label}-failed.png`;
    if (lastBytes) writeFileSync(join(outputDir, file), lastBytes);
    await page.screenshot({ path: join(outputDir, `remote-input-${label}-failed-page.png`) });
    const error = new Error(`Synthetic remote input acknowledgment missing: ${label}`);
    error.code = `synthetic_remote_input_${label}_missing`;
    throw error;
  };
  // Controller takeover can reconnect the presentation. Wait for the exact
  // baseline before the first input; polling pixels does not repeat an action.
  const baseline = await waitForPixels('baseline', (bytes) => digest(bytes) === expectedPixelHash);
  if (JSON.stringify(await observeClip()) !== JSON.stringify(baseline.clip)) {
    throw new Error('Synthetic input geometry changed after baseline');
  }
  await page.mouse.click(clip.x + clip.width / 2, clip.y + clip.height / 2);
  // Keep a provider-rendered cursor outside the acknowledgment crop.
  const currentFrameBox = await frames.first().boundingBox();
  if (!currentFrameBox) throw new Error('Synthetic input frame disappeared after click');
  await page.mouse.move(currentFrameBox.x + 10, currentFrameBox.y + 10);
  const mouse = await waitForPixels('mouse', async (bytes) => page.evaluate(async (base64) => {
    const image = new Image();
    image.src = `data:image/png;base64,${base64}`;
    await image.decode();
    const canvas = document.createElement('canvas');
    canvas.width = image.width;
    canvas.height = image.height;
    const context = canvas.getContext('2d');
    context.drawImage(image, 0, 0);
    return context.getImageData(0, 0, canvas.width, canvas.height).data.every((value) => value === 255);
  }, bytes.toString('base64')));
  await page.keyboard.press('Enter');
  const keyboard = await waitForPixels('keyboard', (bytes) => digest(bytes) === expectedPixelHash);
  return { protocol: SYNTHETIC_INPUT_PROTOCOL, success: true, baseline, mouse, keyboard };
}
