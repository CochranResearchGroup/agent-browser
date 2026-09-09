import { createHash } from 'node:crypto';

const LAUNCH_ARGS = Object.freeze([
  '--disable-background-timer-throttling',
  '--disable-backgrounding-occluded-windows',
  '--disable-renderer-backgrounding',
]);
const DASHBOARD_SESSION_COOKIE = 'agent_browser_dashboard_session';

function hash(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function streamFailureRendered(text) {
  const normalized = text.toLowerCase();
  return ['remote disconnected', 'stream unavailable', 'stream sign-in expired']
    .some((needle) => normalized.includes(needle));
}

export function createPlaywrightRetainedAnchorAdapter({ chromium, convergenceTimeoutMs = 45_000 }) {
  let browser = null;
  let context = null;
  let page = null;
  let dashboardOrigin = null;
  let markerRegion = null;
  let expectedMarkerSha256 = null;
  const oracleFindingCodes = new Set();

  async function probeAuthenticatedSession() {
    if (!context || !dashboardOrigin) return false;
    try {
      const authStatus = await context.request.get(
        new URL('/api/dashboard-auth/status', dashboardOrigin).href,
        { failOnStatusCode: false },
      );
      if (!authStatus.ok()) return false;
      const status = await authStatus.json().catch(() => null);
      if (status?.authenticated !== true) return false;
      const cookies = await context.cookies(dashboardOrigin);
      return cookies.some((cookie) =>
        cookie.name === DASHBOARD_SESSION_COOKIE && cookie.secure === true && cookie.httpOnly === true);
    } catch {
      return false;
    }
  }

  async function sample() {
    const authenticatedSession = await probeAuthenticatedSession();
    const frames = page.locator('iframe');
    const iframeCount = await frames.count();
    let guacamoleIframe = false;
    let iframeBox = null;
    if (iframeCount === 1) {
      iframeBox = await frames.first().boundingBox();
      try {
        const src = await frames.first().getAttribute('src');
        const path = new URL(src).pathname;
        guacamoleIframe = path === '/guacamole' || path === '/guacamole/';
      } catch {
        guacamoleIframe = false;
      }
    }
    let clip = markerRegion;
    if (markerRegion.coordinateSpace === 'remote-view-iframe') {
      if (!iframeBox || markerRegion.x + markerRegion.width > iframeBox.width ||
          markerRegion.y + markerRegion.height > iframeBox.height) {
        clip = null;
      } else {
        clip = {
          x: iframeBox.x + markerRegion.x,
          y: iframeBox.y + markerRegion.y,
          width: markerRegion.width,
          height: markerRegion.height,
        };
      }
    }
    const markerSha256 = clip ? hash(await page.screenshot({ clip })) : null;
    const bodyText = await page.locator('body').innerText().catch(() => '');
    return {
      authenticatedSession,
      markerSha256,
      iframeCount,
      guacamoleIframe,
      streamFailure: streamFailureRendered(bodyText),
      oracleFindingCodes: [...oracleFindingCodes].sort(),
    };
  }

  return {
    async open({ handoffUrl, username, password, markerRegion: region, expectedMarkerSha256: digest }) {
      markerRegion = region;
      expectedMarkerSha256 = digest;
      // The anchor owns stop signals: sample and seal its final receipt before closing.
      browser = await chromium.launch({
        headless: true, args: [...LAUNCH_ARGS], handleSIGTERM: false, handleSIGINT: false,
      });
      context = await browser.newContext({
        viewport: { width: 1440, height: 1000 },
        reducedMotion: 'reduce',
        locale: 'en-US',
      });
      dashboardOrigin = new URL(handoffUrl).origin;
      const auth = await context.request.post(new URL('/api/dashboard-auth/login', dashboardOrigin).href, {
        data: { username, password },
        failOnStatusCode: false,
      });
      if (!auth.ok()) {
        const error = new Error('Dashboard authentication failed');
        error.code = 'dashboard_authentication_failed';
        throw error;
      }
      const authenticatedSession = await probeAuthenticatedSession();
      if (!authenticatedSession) {
        const error = new Error('Dashboard authenticated session was not verified');
        error.code = 'dashboard_authenticated_session_unproven';
        throw error;
      }
      page = await context.newPage();
      page.on('console', (entry) => {
        if (entry.type() === 'error') oracleFindingCodes.add('console_error');
      });
      page.on('requestfailed', (request) => {
        if (request.failure()?.errorText !== 'net::ERR_ABORTED') {
          oracleFindingCodes.add('network_request_failed');
        }
      });
      page.on('response', (response) => {
        if (response.status() >= 500) oracleFindingCodes.add('gateway_or_server_error');
      });
      const response = await page.goto(handoffUrl, {
        waitUntil: 'domcontentloaded',
        timeout: convergenceTimeoutMs,
      });
      if (!response || response.status() >= 400) {
        const error = new Error('Durable handoff navigation failed');
        error.code = 'durable_handoff_navigation_failed';
        throw error;
      }
    },

    async observe({ waitForConvergence }) {
      if (!waitForConvergence) return sample();
      const deadline = Date.now() + convergenceTimeoutMs;
      let observation = null;
      do {
        observation = await sample();
        if (observation.markerSha256 === expectedMarkerSha256 &&
            observation.iframeCount === 1 && observation.guacamoleIframe &&
            !observation.streamFailure) {
          return observation;
        }
        await page.waitForTimeout(500);
      } while (Date.now() < deadline);
      return observation;
    },

    async close() {
      await context?.close().catch(() => {});
      await browser?.close().catch(() => {});
    },
  };
}
