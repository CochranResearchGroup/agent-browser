import { createHash } from 'node:crypto';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';

export const SYNTHETIC_INPUT_PROTOCOL = 'trusted-marker-click-white-enter-reset-v1';
const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');

// Observe rendered pixels only. This never evaluates or changes the remote DOM.
export async function verifySyntheticRemoteInput(page, { region, expectedPixelHash, outputDir, timeoutMs = 10_000 }) {
  const frames = page.locator('iframe');
  if (await frames.count() !== 1) throw new Error('Synthetic input requires exactly one remote frame');
  const box = await frames.first().boundingBox();
  if (!box || region.coordinateSpace !== 'remote-view-iframe' ||
      region.x < 0 || region.y < 0 || region.width < 1 || region.height < 1 ||
      region.x + region.width > box.width || region.y + region.height > box.height) {
    throw new Error('Synthetic input region does not fit the remote frame');
  }
  const clip = { x: box.x + region.x, y: box.y + region.y, width: region.width, height: region.height };
  const before = await page.screenshot({ clip });
  if (digest(before) !== expectedPixelHash) throw new Error('Synthetic input baseline pixels do not match');
  const waitForPixels = async (label, accept) => {
    const deadline = Date.now() + timeoutMs;
    do {
      const bytes = await page.screenshot({ clip });
      if (await accept(bytes)) {
        const file = `remote-input-${label}.png`;
        writeFileSync(join(outputDir, file), bytes);
        return { file, sha256: digest(bytes), observedAt: new Date().toISOString() };
      }
      await page.waitForTimeout(100);
    } while (Date.now() < deadline);
    throw new Error(`Synthetic remote input acknowledgment missing: ${label}`);
  };
  await page.mouse.click(clip.x + clip.width / 2, clip.y + clip.height / 2);
  // Keep a provider-rendered cursor outside the acknowledgment crop.
  await page.mouse.move(box.x + 10, box.y + 10);
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
  return { protocol: SYNTHETIC_INPUT_PROTOCOL, success: true, mouse, keyboard };
}
