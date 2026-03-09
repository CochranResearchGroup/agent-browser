/**
 * Run browser automation directly in a Vercel serverless function
 * using @sparticuz/chromium + puppeteer-core.
 *
 * In development, uses the local Chrome/Chromium installation.
 * In production (Vercel), uses @sparticuz/chromium's bundled binary.
 */

import puppeteer from "puppeteer-core";
import chromium from "@sparticuz/chromium";

async function launchBrowser() {
  const isLambda =
    !!process.env.VERCEL || !!process.env.AWS_LAMBDA_FUNCTION_NAME;

  const executablePath = isLambda
    ? await chromium.executablePath()
    : process.env.CHROMIUM_PATH ||
      "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

  const args = isLambda
    ? chromium.args
    : ["--no-sandbox", "--disable-setuid-sandbox"];

  return puppeteer.launch({
    args,
    executablePath,
    headless: true,
    defaultViewport: { width: 1280, height: 720 },
  });
}

export async function screenshotUrl(
  url: string,
  opts: { fullPage?: boolean } = {},
): Promise<{ screenshot: string; title: string }> {
  const browser = await launchBrowser();

  try {
    const page = await browser.newPage();
    await page.goto(url, { waitUntil: "networkidle2", timeout: 30_000 });

    const title = await page.title();
    const screenshot = await page.screenshot({
      fullPage: opts.fullPage,
      encoding: "base64",
    });

    return {
      title: title || url,
      screenshot: screenshot as string,
    };
  } finally {
    await browser.close();
  }
}

export async function snapshotUrl(
  url: string,
  opts: { selector?: string } = {},
): Promise<{ snapshot: string; title: string }> {
  const browser = await launchBrowser();

  try {
    const page = await browser.newPage();
    await page.goto(url, { waitUntil: "networkidle2", timeout: 30_000 });

    const title = await page.title();

    const snapshot = await page.accessibility.snapshot({
      root: opts.selector
        ? (await page.$(opts.selector)) ?? undefined
        : undefined,
    });

    return {
      title: title || url,
      snapshot: JSON.stringify(snapshot, null, 2),
    };
  } finally {
    await browser.close();
  }
}
