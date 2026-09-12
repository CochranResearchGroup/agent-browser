# CAPTCHA Practice Lab

Local Cloudflare Turnstile and hCaptcha applications for exercising browser
perception, pointer control, callback observation, widget reset, and
server-side token verification.

The defaults are vendor-published test credentials. They provide no anti-bot
protection and must never be deployed as production credentials.

## Start

```bash
pnpm captcha-lab:start
```

Open:

- `http://captcha-lab.localtest.me:4178/turnstile.html`
- `http://captcha-lab.localtest.me:4178/hcaptcha.html`

`localtest.me` resolves to `127.0.0.1`. It gives hCaptcha a non-localhost
hostname without modifying `/etc/hosts`. The loopback URL remains available
for server diagnostics, but hCaptcha documents that its widget should not be
loaded from a `localhost` or `127.0.0.1` hostname.

## Test modes

Turnstile defaults to Cloudflare's forced-interaction test sitekey. Override
the keys when another documented test mode is needed:

```bash
TURNSTILE_SITEKEY=1x00000000000000000000AA \
TURNSTILE_SECRET=1x0000000000000000000000000000000AA \
pnpm captcha-lab:start
```

hCaptcha's public test sitekey always passes and does not show an image
challenge. To practice a visual hCaptcha flow, create a non-production
sitekey configured for Always Challenge and start the lab with its credentials:

```bash
HCAPTCHA_SITEKEY=your_test_sitekey \
HCAPTCHA_SECRET=your_test_secret \
pnpm captcha-lab:start
```

Environment-provided secrets stay server-side and are not returned by
`/api/config` or rendered into HTML.

## Verify

```bash
pnpm test:captcha-lab
```
