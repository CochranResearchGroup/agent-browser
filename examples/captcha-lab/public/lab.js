const status = document.querySelector('[data-status]');
const provider = document.body.dataset.provider;
let token = '';

function report(event, detail = {}) {
  status.textContent = JSON.stringify(
    {
      provider,
      event,
      tokenPresent: Boolean(token),
      tokenLength: token.length,
      at: new Date().toISOString(),
      ...detail,
    },
    null,
    2,
  );
  document.body.dataset.challengeState = event;
}

window.captchaSolved = (value) => {
  token = value;
  report('solved');
};

window.captchaExpired = () => {
  token = '';
  report('expired');
};

window.captchaError = (code) => {
  token = '';
  report('error', { code: String(code) });
};

document.querySelector('[data-reset]').addEventListener('click', () => {
  token = '';
  if (provider === 'turnstile') window.turnstile?.reset();
  if (provider === 'hcaptcha') window.hcaptcha?.reset();
  report('reset');
});

document.querySelector('[data-verify]').addEventListener('click', async () => {
  if (!token) {
    report('verification_skipped', { reason: 'no_token' });
    return;
  }
  report('verifying');
  try {
    const response = await fetch('/api/verify', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ provider, token }),
    });
    const result = await response.json();
    report('verified', { httpStatus: response.status, result });
  } catch (error) {
    report('verification_error', { message: error.message });
  }
});

report('loading');
