// Attach to the released, explicitly supplied actual Native WebView2 endpoint.
// No app launch, extra port, Host replacement or picker substitution.
import { createRequire } from 'node:module';
import { writeFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
const require = createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json');
const { chromium } = require('playwright');

export async function attachNative(endpoint, expectedHost) {
  assert.ok(endpoint && expectedHost);
  const browser = await chromium.connectOverCDP(endpoint);
  const pages = browser.contexts().flatMap(context => context.pages()).filter(page => !page.url().startsWith('devtools:'));
  assert.equal(pages.length, 1, 'One isolated Native product page is required');
  const page = pages[0];
  const identity = await page.evaluate(() => window.__TAURI_INTERNALS__.invoke('agent_connection_info'));
  assert.equal(identity.host_instance_id, expectedHost);
  const session = await page.context().newCDPSession(page);
  const version = await session.send('Browser.getVersion');
  return { browser, page, session, identity, version };
}

// Same trace categories used for DevTools frame events and User Timing marks.
// Primary reference: https://chromedevtools.github.io/devtools-protocol/tot/Tracing/
export async function beginTrace(session, screenshots = false) {
  const categories = ['devtools.timeline', 'disabled-by-default-devtools.timeline',
    'disabled-by-default-devtools.timeline.frame', 'blink.user_timing', 'cc'];
  if (screenshots) categories.push('disabled-by-default-devtools.screenshot');
  await session.send('Tracing.start', { categories: categories.join(','), options: 'record-until-full', transferMode: 'ReturnAsStream', streamFormat: 'json' });
}

export async function endTrace(session, tracePath, metadataPath, metadata) {
  const complete = new Promise(resolve => session.once('Tracing.tracingComplete', resolve));
  await session.send('Tracing.end');
  const state = await complete;
  assert.ok(state.stream, 'Native trace stream was not returned');
  const chunks = []; let eof = false;
  while (!eof) {
    const part = await session.send('IO.read', { handle: state.stream });
    chunks.push(Buffer.from(part.data, part.base64Encoded ? 'base64' : 'utf8')); eof = part.eof;
  }
  await session.send('IO.close', { handle: state.stream });
  await writeFile(tracePath, Buffer.concat(chunks));
  await writeFile(metadataPath, JSON.stringify({ ...metadata, trace_state: state, status: 'raw_collected_not_reduced', actual_performance_result: null }, null, 2));
  return state;
}
