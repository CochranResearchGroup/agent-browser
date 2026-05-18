#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const servicePanel = readFileSync('packages/dashboard/src/components/service-panel.tsx', 'utf8');
const dashboardCss = readFileSync('packages/dashboard/src/app/globals.css', 'utf8');
const validationSelector = readFileSync('scripts/dev/select-validation.js', 'utf8');

assert.match(
  servicePanel,
  /type BrowserLifecycleFilter = "actionable" \| "all" \| "live" \| "retained";/,
  'Browser table must keep explicit live, retained, all, and actionable lifecycle filters',
);

assert.match(
  servicePanel,
  /const \[lifecycleFilter, setLifecycleFilter\] = useState<BrowserLifecycleFilter>\("actionable"\);/,
  'Browser table must default to actionable records instead of showing all inert retained records first',
);

assert.match(
  servicePanel,
  /function isInertRetainedBrowserRecord\(browser: ServiceBrowser\): boolean[\s\S]*browser\.health[\s\S]*"not_started"/,
  'Browser table must classify inert retained not_started browser records explicitly',
);

assert.match(
  servicePanel,
  /DropdownMenuCheckboxItem[\s\S]*Visible columns[\s\S]*Reset columns/,
  'Browser table must expose column visibility controls',
);

assert.match(
  servicePanel,
  /browserDefaultRank\(left\) - browserDefaultRank\(right\)/,
  'Browser table sorting must keep non-ready or live records ahead of inert retained records',
);

assert.match(
  dashboardCss,
  /\.service-browser-table-controls/,
  'Browser table lifecycle and column controls must have an explicit layout hook',
);

assert.match(
  validationSelector,
  /pnpm test:dashboard-browser-table/,
  'Validation selector must recommend the browser table smoke for Service dashboard wiring changes',
);

console.log('Dashboard browser table contract smoke passed');
