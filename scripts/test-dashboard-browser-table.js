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
  /BROWSER_TABLE_LIFECYCLE_FILTER_STORAGE_KEY = "agent-browser-dashboard-browser-table-lifecycle-filter"/,
  'Browser table must persist the selected lifecycle filter under a stable localStorage key',
);

assert.match(
  servicePanel,
  /BROWSER_TABLE_VISIBLE_COLUMNS_STORAGE_KEY = "agent-browser-dashboard-browser-table-visible-columns"/,
  'Browser table must persist visible column preferences under a stable localStorage key',
);

assert.match(
  servicePanel,
  /function initialBrowserLifecycleFilter\(\): BrowserLifecycleFilter[\s\S]*return isBrowserLifecycleFilter\(stored\) \? stored : "actionable";/,
  'Browser table must validate persisted lifecycle filters and default to actionable records',
);

assert.match(
  servicePanel,
  /function initialBrowserTableColumns\(\): BrowserTableColumnKey\[\][\s\S]*parsed\.filter\(isBrowserTableColumnKey\)[\s\S]*DEFAULT_BROWSER_TABLE_COLUMNS/,
  'Browser table must validate persisted visible columns before applying them',
);

assert.match(
  servicePanel,
  /const \[lifecycleFilter, setLifecycleFilter\] = useState<BrowserLifecycleFilter>\(initialBrowserLifecycleFilter\);/,
  'Browser table lifecycle state must use the persisted preference initializer',
);

assert.match(
  servicePanel,
  /const \[visibleColumns, setVisibleColumns\] = useState<BrowserTableColumnKey\[\]>\(initialBrowserTableColumns\);/,
  'Browser table visible columns state must use the persisted preference initializer',
);

assert.match(
  servicePanel,
  /localStorage\.setItem\(BROWSER_TABLE_LIFECYCLE_FILTER_STORAGE_KEY, lifecycleFilter\)/,
  'Browser table must save lifecycle filter changes locally',
);

assert.match(
  servicePanel,
  /localStorage\.setItem\(BROWSER_TABLE_VISIBLE_COLUMNS_STORAGE_KEY, JSON\.stringify\(visibleColumns\)\)/,
  'Browser table must save visible column changes locally',
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
