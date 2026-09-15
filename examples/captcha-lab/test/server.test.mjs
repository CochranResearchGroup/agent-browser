import assert from 'node:assert/strict';
import { after, before, test } from 'node:test';

import { createCaptchaLabServer } from '../server.mjs';

let baseUrl;
let server;
const requests = [];

before(async () => {
  server = createCaptchaLabServer({
    fetchImpl: async (url, options) => {
      requests.push({ url, body: options.body.toString() });
      return new Response(JSON.stringify({ success: true }), {
        status: 200,
        headers: { 'content-type': 'application/json' },
      });
    },
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  baseUrl = `http://127.0.0.1:${server.address().port}`;
});

after(async () => {
  await new Promise((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())));
});

test('serves both provider applications with official test sitekeys', async () => {
  const turnstile = await (await fetch(`${baseUrl}/turnstile.html`)).text();
  const hcaptcha = await (await fetch(`${baseUrl}/hcaptcha.html`)).text();
  assert.match(turnstile, /3x00000000000000000000FF/);
  assert.match(hcaptcha, /10000000-ffff-ffff-ffff-000000000001/);
});

test('forwards Turnstile verification as form data', async () => {
  const response = await fetch(`${baseUrl}/api/verify`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ provider: 'turnstile', token: 'turnstile-token' }),
  });
  const payload = await response.json();
  assert.equal(payload.result.success, true);
  assert.equal(requests.at(-1).url, 'https://challenges.cloudflare.com/turnstile/v0/siteverify');
  assert.match(requests.at(-1).body, /response=turnstile-token/);
});

test('forwards hCaptcha verification with the expected sitekey', async () => {
  const response = await fetch(`${baseUrl}/api/verify`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ provider: 'hcaptcha', token: 'hcaptcha-token' }),
  });
  assert.equal(response.status, 200);
  assert.equal(requests.at(-1).url, 'https://api.hcaptcha.com/siteverify');
  assert.match(requests.at(-1).body, /sitekey=10000000-ffff-ffff-ffff-000000000001/);
});

test('rejects malformed verification requests', async () => {
  const response = await fetch(`${baseUrl}/api/verify`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ provider: 'hcaptcha' }),
  });
  assert.equal(response.status, 400);
});
