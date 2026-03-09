import { NextRequest, NextResponse } from "next/server";
import * as ab from "@/lib/agent-browser";

/**
 * POST /api/browse
 *
 * Programmatic API route for browser automation.
 * Uses @sparticuz/chromium + puppeteer-core directly in the function.
 *
 * Body: { "action": "screenshot", "url": "https://example.com" }
 * Or:   { "action": "snapshot", "url": "https://example.com" }
 */
export async function POST(req: NextRequest) {
  try {
    const body = await req.json();
    const url = body.url;

    if (!url) {
      return NextResponse.json({ error: "Provide a 'url'" }, { status: 400 });
    }

    if (body.action === "screenshot") {
      const result = await ab.screenshotUrl(url, {
        fullPage: body.fullPage,
      });
      return NextResponse.json(result);
    }

    if (body.action === "snapshot") {
      const result = await ab.snapshotUrl(url, {
        selector: body.selector,
      });
      return NextResponse.json(result);
    }

    return NextResponse.json(
      { error: "Provide 'action' as 'screenshot' or 'snapshot'" },
      { status: 400 },
    );
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
