#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

import {
  browserRowCloseTitle,
  browserRowRepairTitle,
} from '../packages/dashboard/src/lib/service-browser-row-actions.ts';

const require = createRequire(new URL('../packages/dashboard/package.json', import.meta.url));
const React = require('react');
const { renderToStaticMarkup } = require('react-dom/server');

function renderActionButton(label, title, disabled) {
  return renderToStaticMarkup(
    React.createElement('button', { disabled, title }, label),
  );
}

const unsupportedClose = browserRowCloseTitle({ available: false, supported: false });
const ineligibleClose = browserRowCloseTitle({ available: false, supported: true });
const availableClose = browserRowCloseTitle({ available: true, supported: true });

assert.match(
  renderActionButton('Close', unsupportedClose, true),
  /title="This service does not advertise row-scoped browser close support\."/,
);
assert.match(
  renderActionButton('Close', ineligibleClose, true),
  /title="Only the active service browser can be closed from this row\."/,
);
assert.match(
  renderActionButton('Close', availableClose, false),
  /title="Queue polite close for this service browser\."/,
);
assert.notEqual(unsupportedClose, ineligibleClose);

const unsupportedRepair = browserRowRepairTitle({ available: false, supported: false });
const ineligibleRepair = browserRowRepairTitle({ available: false, supported: true });
const availableRepair = browserRowRepairTitle({ available: true, supported: true });

assert.match(
  renderActionButton('Repair', unsupportedRepair, true),
  /title="This service does not advertise row-scoped browser repair support\."/,
);
assert.match(
  renderActionButton('Repair', ineligibleRepair, true),
  /title="Repair is available for degraded or faulted browser records\."/,
);
assert.match(
  renderActionButton('Repair', availableRepair, false),
  /title="Mark this degraded or faulted browser retryable\."/,
);
assert.notEqual(unsupportedRepair, ineligibleRepair);

console.log('Dashboard browser row action rendered-title smoke passed');
