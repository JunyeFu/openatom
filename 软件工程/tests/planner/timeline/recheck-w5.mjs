// Execute the original read-only W5 assertions against the actual product
// component. Only root/artifact locations and the installed browser executable
// are selected here; W5 tests, fixtures and expectations are never rewritten.
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
const root = new URL('../../../', import.meta.url);
const phase = process.argv[2];
if (!['before', 'after'].includes(phase)) throw new Error('Use before or after');
const output = new URL(`scenarios/从零搭建控制盒厂房/runtime-timeline/repair-W5-P204-01/${phase}/`, root);
await mkdir(output, { recursive: true });
const original = await readFile(new URL('tests/planner/acceptance/timeline_consumer.mjs', root), 'utf8');
const originalLocations = "const root=new URL('../../../',import.meta.url),out=new URL('.work/whole-packages/window5-timeline-evaluation/timeline-browser/',root);";
if (!original.includes(originalLocations)) throw new Error('W5 runner locations changed; inspect before executing');
const runner = original.replace(originalLocations, `const root=new URL(${JSON.stringify(root.href)}),out=new URL(${JSON.stringify(output.href)});`);
await writeFile(new URL('w5-readonly-consumer.mjs', output), runner);
const browser = `const {chromium}=require('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');module.exports={chromium:{launch:(options)=>chromium.launch({...options,executablePath:'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe'})}};`;
await writeFile(new URL('installed-browser.cjs', output), browser);
const result = spawnSync(process.execPath, [fileURLToPath(new URL('w5-readonly-consumer.mjs', output)), fileURLToPath(new URL('installed-browser.cjs', output))], { cwd: fileURLToPath(root), encoding: 'utf8', windowsHide: true });
await writeFile(new URL('stdout.txt', output), result.stdout ?? '');
await writeFile(new URL('stderr.txt', output), result.stderr ?? '');
const resultFile = new URL('result.json', output);
const details = JSON.parse(await readFile(resultFile, 'utf8'));
const summary = { phase, original_test: 'tests/planner/acceptance/timeline_consumer.mjs', original_test_or_expectations_written: false,
  changes_in_executed_copy: ['absolute workspace root', 'owned output directory', 'real installed Chromium executable adapter'],
  original_assertions_unchanged: true, exit_code: result.status, passed: details.results.filter(r => r.passed).length, total: details.results.length,
  failures: details.results.filter(r => !r.passed), errors: details.errors, actual_product_Native_UI: false, independent_W5_receipt_claimed: false };
await writeFile(new URL('recheck-summary.json', output), JSON.stringify(summary, null, 2) + '\n');
console.log(JSON.stringify(summary, null, 2));
process.exitCode = result.status ?? 1;
