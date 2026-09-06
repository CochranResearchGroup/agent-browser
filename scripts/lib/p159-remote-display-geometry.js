// Guacamole 1.5.5 Display.getElement() returns a scaled bounding div whose
// first child has the native desktop dimensions and a CSS scale transform.
// Read presentation geometry only; never inspect the remote browser DOM/pixels.
export async function observeRemoteDisplayClip(page, region) {
  const frames = page.locator('iframe');
  const count = await frames.count();
  if (!count) return null;
  if (count !== 1) throw new Error('Remote display requires exactly one iframe');
  const frameBox = await frames.first().boundingBox();
  const display = page.frameLocator('iframe').locator('.display > div > div');
  const displays = await display.count();
  if (!frameBox || !displays) return null;
  if (displays !== 1) throw new Error('Remote display requires exactly one desktop');
  const native = await display.evaluate((element) => ({
    width: element.offsetWidth, height: element.offsetHeight,
  }));
  const box = await display.boundingBox();
  return remoteDisplayClip(region, native, box, frameBox);
}

// Keep the sample's CSS dimensions unchanged so its PNG oracle remains exact.
// The declared native marker rectangle determines its center, not a pixel scan.
export function remoteDisplayClip(region, native, box, frameBox) {
  if (!box || !frameBox || native.width <= 0 || native.height <= 0) return null;
  const values = [region.x, region.y, region.width, region.height,
    region.sampleWidth, region.sampleHeight, native.width, native.height,
    box.x, box.y, box.width, box.height];
  if (!values.every(Number.isFinite) || region.x < 0 || region.y < 0 ||
      region.width <= 0 || region.height <= 0 || region.sampleWidth <= 0 || region.sampleHeight <= 0 ||
      region.x + region.width > native.width || region.y + region.height > native.height) {
    throw new Error('Invalid native remote marker geometry');
  }
  const sx = box.width / native.width;
  const sy = box.height / native.height;
  if (sx <= 0 || sy <= 0 || Math.abs(sx - sy) > 0.001) return null;
  const marker = { x: box.x + region.x * sx, y: box.y + region.y * sy,
    width: region.width * sx, height: region.height * sy };
  const clip = {
    x: Math.round(marker.x + (marker.width - region.sampleWidth) / 2),
    y: Math.round(marker.y + (marker.height - region.sampleHeight) / 2),
    width: region.sampleWidth, height: region.sampleHeight,
  };
  for (const boundary of [marker, frameBox]) {
    if (clip.x < boundary.x || clip.y < boundary.y ||
        clip.x + clip.width > boundary.x + boundary.width ||
        clip.y + clip.height > boundary.y + boundary.height) return null;
  }
  return clip;
}
