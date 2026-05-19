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
  /BROWSER_TABLE_COLUMN_WIDTHS_STORAGE_KEY = "agent-browser-dashboard-browser-table-column-widths"/,
  'Browser table must persist adjusted column widths under a stable localStorage key',
);

assert.match(
  servicePanel,
  /BROWSER_TABLE_DENSITY_STORAGE_KEY = "agent-browser-dashboard-browser-table-density"/,
  'Browser table must persist row density under a stable localStorage key',
);

assert.match(
  servicePanel,
  /DEFAULT_BROWSER_TABLE_COLUMN_WIDTHS: Record<BrowserTableColumnId, number>/,
  'Browser table must define default widths for every adjustable column',
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
  /function initialBrowserTableColumnWidths\(\): Record<BrowserTableColumnId, number>[\s\S]*clampBrowserTableColumnWidth\(value\)/,
  'Browser table must validate persisted column widths before applying them',
);

assert.match(
  servicePanel,
  /function initialBrowserTableDensity\(\): BrowserTableDensity[\s\S]*return isBrowserTableDensity\(stored\) \? stored : "standard";/,
  'Browser table must validate persisted row density before applying it',
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
  /localStorage\.setItem\(BROWSER_TABLE_COLUMN_WIDTHS_STORAGE_KEY, JSON\.stringify\(columnWidths\)\)/,
  'Browser table must save column width changes locally',
);

assert.match(
  servicePanel,
  /localStorage\.setItem\(BROWSER_TABLE_DENSITY_STORAGE_KEY, density\)/,
  'Browser table must save row density changes locally',
);

assert.match(
  servicePanel,
  /function BrowserTableHeaderCell\([\s\S]*service-browser-table-resize[\s\S]*onMouseDown=\{\(event\) => onResizeStart\(column, event\)\}/,
  'Browser table headers must expose resize handles',
);

assert.match(
  servicePanel,
  /window\.addEventListener\("mousemove", handleMouseMove\)/,
  'Browser table column resizing must attach mousemove listeners',
);

assert.match(
  servicePanel,
  /window\.removeEventListener\("mousemove", handleMouseMove\)/,
  'Browser table column resizing must remove mousemove listeners',
);

assert.match(
  servicePanel,
  /function isInertRetainedBrowserRecord\(browser: ServiceBrowser\): boolean[\s\S]*browser\.health[\s\S]*"not_started"/,
  'Browser table must classify inert retained not_started browser records explicitly',
);

assert.match(
  servicePanel,
  /DropdownMenuCheckboxItem[\s\S]*Visible columns[\s\S]*Reset columns[\s\S]*Reset widths/,
  'Browser table must expose column visibility and width reset controls',
);

assert.match(
  servicePanel,
  /service-browser-table-density[\s\S]*Compact[\s\S]*Standard[\s\S]*Expanded/,
  'Browser table must expose compact, standard, and expanded density controls',
);

assert.match(
  servicePanel,
  /service-browser-table-density-\$\{density\}/,
  'Browser table must apply the selected density as a table class',
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
  dashboardCss,
  /\.service-browser-table-resize[\s\S]*cursor: col-resize/,
  'Browser table resize handles must have an explicit resize affordance',
);

assert.match(
  dashboardCss,
  /\.service-browser-table-density-compact[\s\S]*\.service-browser-table-density-expanded/,
  'Browser table density modes must have explicit compact and expanded CSS hooks',
);

assert.match(
  validationSelector,
  /pnpm test:dashboard-browser-table/,
  'Validation selector must recommend the browser table smoke for Service dashboard wiring changes',
);

console.log('Dashboard browser table contract smoke passed');
