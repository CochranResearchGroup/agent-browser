"use server";

import * as serverless from "@/lib/agent-browser";
import * as sandbox from "@/lib/agent-browser-sandbox";

export type EnvStatus = {
  serverless: {
    hasChromiumPath: boolean;
    isVercel: boolean;
  };
  sandbox: {
    hasSnapshot: boolean;
  };
};

export async function getEnvStatus(): Promise<EnvStatus> {
  return {
    serverless: {
      hasChromiumPath: !!process.env.CHROMIUM_PATH,
      isVercel:
        !!process.env.VERCEL || !!process.env.AWS_LAMBDA_FUNCTION_NAME,
    },
    sandbox: {
      hasSnapshot: !!process.env.AGENT_BROWSER_SNAPSHOT_ID,
    },
  };
}

export type ScreenshotResult = {
  ok: boolean;
  screenshot?: string;
  title?: string;
  error?: string;
};

export type SnapshotResult = {
  ok: boolean;
  snapshot?: string;
  title?: string;
  error?: string;
};

export type Mode = "serverless" | "sandbox";

/**
 * Server action: screenshot a URL.
 *
 * mode="serverless" -- runs @sparticuz/chromium + puppeteer-core in the function
 * mode="sandbox"    -- runs agent-browser inside a Vercel Sandbox microVM
 */
export async function takeScreenshot(
  url: string,
  mode: Mode = "serverless",
): Promise<ScreenshotResult> {
  try {
    if (mode === "sandbox") {
      const { screenshot, title } = await sandbox.screenshotUrl(url);
      return { ok: true, screenshot, title };
    }

    const { screenshot, title } = await serverless.screenshotUrl(url);
    return { ok: true, screenshot, title };
  } catch (err) {
    return {
      ok: false,
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

/**
 * Server action: snapshot a URL (accessibility tree).
 *
 * mode="serverless" -- runs @sparticuz/chromium + puppeteer-core in the function
 * mode="sandbox"    -- runs agent-browser inside a Vercel Sandbox microVM
 */
export async function takeSnapshot(
  url: string,
  mode: Mode = "serverless",
): Promise<SnapshotResult> {
  try {
    if (mode === "sandbox") {
      const { snapshot, title } = await sandbox.snapshotUrl(url, {
        interactive: true,
        compact: true,
      });
      return { ok: true, snapshot, title };
    }

    const { snapshot, title } = await serverless.snapshotUrl(url);
    return { ok: true, snapshot, title };
  } catch (err) {
    return {
      ok: false,
      error: err instanceof Error ? err.message : String(err),
    };
  }
}
