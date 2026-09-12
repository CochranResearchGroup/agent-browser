import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('./public/', import.meta.url));
const defaults = Object.freeze({
  turnstileSitekey: '3x00000000000000000000FF',
  turnstileSecret: '1x0000000000000000000000000000000AA',
  hcaptchaSitekey: '10000000-ffff-ffff-ffff-000000000001',
  hcaptchaSecret: '0x0000000000000000000000000000000000000000',
});

const contentTypes = {
  '.css': 'text/css; charset=utf-8',
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
};

function json(response, status, body) {
  response.writeHead(status, {
    'cache-control': 'no-store',
    'content-type': 'application/json; charset=utf-8',
  });
  response.end(JSON.stringify(body));
}

async function readBody(request) {
  const chunks = [];
  let size = 0;
  for await (const chunk of request) {
    size += chunk.length;
    if (size > 16_384) throw new Error('request_too_large');
    chunks.push(chunk);
  }
  return JSON.parse(Buffer.concat(chunks).toString('utf8'));
}

async function verifyToken(provider, token, remoteip, configuration, fetchImpl) {
  const isTurnstile = provider === 'turnstile';
  const endpoint = isTurnstile
    ? 'https://challenges.cloudflare.com/turnstile/v0/siteverify'
    : 'https://api.hcaptcha.com/siteverify';
  const body = new URLSearchParams({
    secret: isTurnstile ? configuration.turnstileSecret : configuration.hcaptchaSecret,
    response: token,
  });
  if (remoteip) body.set('remoteip', remoteip);
  if (!isTurnstile) body.set('sitekey', configuration.hcaptchaSitekey);

  const upstream = await fetchImpl(endpoint, {
    method: 'POST',
    headers: { 'content-type': 'application/x-www-form-urlencoded' },
    body,
    signal: AbortSignal.timeout(10_000),
  });
  const result = await upstream.json();
  return { provider, upstreamStatus: upstream.status, result };
}

async function serveStatic(pathname, response, configuration) {
  const route = pathname === '/' ? '/index.html' : pathname;
  const safePath = normalize(route).replace(/^(\.\.[/\\])+/, '');
  const filePath = join(root, safePath);
  if (!filePath.startsWith(root)) {
    response.writeHead(404);
    response.end('Not found');
    return;
  }

  try {
    let body = await readFile(filePath);
    if (extname(filePath) === '.html') {
      body = Buffer.from(
        body
          .toString('utf8')
          .replaceAll('{{TURNSTILE_SITEKEY}}', configuration.turnstileSitekey)
          .replaceAll('{{HCAPTCHA_SITEKEY}}', configuration.hcaptchaSitekey),
      );
    }
    response.writeHead(200, {
      'cache-control': 'no-store',
      'content-type': contentTypes[extname(filePath)] ?? 'application/octet-stream',
    });
    response.end(body);
  } catch (error) {
    if (error.code === 'ENOENT') {
      response.writeHead(404);
      response.end('Not found');
      return;
    }
    throw error;
  }
}

export function createCaptchaLabServer(options = {}) {
  const configuration = {
    turnstileSitekey: options.turnstileSitekey ?? process.env.TURNSTILE_SITEKEY ?? defaults.turnstileSitekey,
    turnstileSecret: options.turnstileSecret ?? process.env.TURNSTILE_SECRET ?? defaults.turnstileSecret,
    hcaptchaSitekey: options.hcaptchaSitekey ?? process.env.HCAPTCHA_SITEKEY ?? defaults.hcaptchaSitekey,
    hcaptchaSecret: options.hcaptchaSecret ?? process.env.HCAPTCHA_SECRET ?? defaults.hcaptchaSecret,
  };
  const fetchImpl = options.fetchImpl ?? fetch;

  return createServer(async (request, response) => {
    try {
      const url = new URL(request.url, 'http://captcha-lab.localtest.me');
      if (request.method === 'GET' && url.pathname === '/api/config') {
        json(response, 200, {
          turnstileSitekey: configuration.turnstileSitekey,
          hcaptchaSitekey: configuration.hcaptchaSitekey,
        });
        return;
      }
      if (request.method === 'POST' && url.pathname === '/api/verify') {
        const body = await readBody(request);
        if (!['turnstile', 'hcaptcha'].includes(body.provider) || typeof body.token !== 'string' || !body.token) {
          json(response, 400, { error: 'provider_and_token_required' });
          return;
        }
        const remoteip = request.socket.remoteAddress;
        json(response, 200, await verifyToken(body.provider, body.token, remoteip, configuration, fetchImpl));
        return;
      }
      if (request.method !== 'GET') {
        json(response, 405, { error: 'method_not_allowed' });
        return;
      }
      await serveStatic(url.pathname, response, configuration);
    } catch (error) {
      json(response, error.message === 'request_too_large' ? 413 : 500, {
        error: error.message === 'request_too_large' ? error.message : 'internal_error',
      });
    }
  });
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const port = Number.parseInt(process.env.PORT ?? '4178', 10);
  const host = process.env.HOST ?? '0.0.0.0';
  const server = createCaptchaLabServer();
  server.listen(port, host, () => {
    process.stdout.write(`Captcha lab listening at http://captcha-lab.localtest.me:${port}\n`);
    process.stdout.write(`Loopback fallback: http://127.0.0.1:${port}\n`);
  });
}
