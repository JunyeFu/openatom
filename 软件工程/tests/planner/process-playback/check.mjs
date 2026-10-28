import { build } from 'vite';
import react from '@vitejs/plugin-react';
import { readFile, writeFile, mkdir, readdir } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';
const root = fileURLToPath(new URL('../../../', import.meta.url));
const label = process.argv[2] ?? 'current';
const entry = process.argv[3] ? path.resolve(root, process.argv[3]) : fileURLToPath(new URL('browser.tsx', import.meta.url));
const out = path.join(root, '.work/full-closeout-20261021/process-playback', label);
await mkdir(out, { recursive: true });
await build({ root, configFile: false, plugins: [react()], define: { 'process.env.NODE_ENV': '"production"' }, logLevel: 'error', build: { target: 'es2022', outDir: out,
  emptyOutDir: false, lib: { entry, name: 'ProcessPlaybackCheck', formats: ['iife'], fileName: () => 'check.js' },
  } });
const styles = (await readdir(out)).filter(name => name.endsWith('.css')).map(name => `<link rel="stylesheet" href="${name}">`).join('');
await writeFile(path.join(out, 'index.html'), `<!doctype html><html><head><meta charset="utf-8">${styles}</head><body><div id="root"></div><script>window.__checkErrors=[];window.onerror=(message)=>window.__checkErrors.push(message)</script><script src="check.js"></script></body></html>`);
const chrome = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const profile = path.join(out, 'chrome-profile-' + Date.now());
const args = ['--headless=new', '--disable-gpu', '--no-first-run', '--no-default-browser-check', '--allow-file-access-from-files',
  '--user-data-dir=' + profile, '--remote-debugging-address=127.0.0.1', '--remote-debugging-port=0', 'about:blank'];
const child = spawn(chrome, args, { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
let stderr = ''; child.stderr.setEncoding('utf8'); child.stderr.on('data', value => stderr += value);
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
let port;
for (let i = 0; i < 200; i++) { try { port = Number((await readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).split('\n')[0]); break; } catch { await delay(25); } }
if (!port) throw new Error('Own headless Chrome did not start');
const pages = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
const socket = new WebSocket(pages.find(page => page.type === 'page').webSocketDebuggerUrl);
await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject; });
let sequence = 0; const pending = new Map();
socket.onmessage = event => { const value = JSON.parse(event.data); const p = pending.get(value.id); if (p) { pending.delete(value.id); value.error ? p.reject(new Error(JSON.stringify(value.error))) : p.resolve(value.result); } };
function call(method, params = {}) { return new Promise((resolve, reject) => { const id = ++sequence; pending.set(id, { resolve, reject }); socket.send(JSON.stringify({ id, method, params })); }); }
await call('Page.navigate', { url: pathToFileURL(path.join(out, 'index.html')).href });
let result;
for (let i = 0; i < 400; i++) {
  const value = await call('Runtime.evaluate', { expression: 'document.querySelector("#result")?.textContent', returnByValue: true });
  if (value.result.value) { result = JSON.parse(value.result.value); break; } await delay(50);
}
const dom = await call('Runtime.evaluate', { expression: 'document.documentElement.outerHTML', returnByValue: true });
await writeFile(path.join(out, 'dom.html'), dom.result.value); await writeFile(path.join(out, 'chrome.stderr.txt'), stderr);
result ??= { status: 'harness_failed', error: 'No result from actual component' };
result.browser = { headless: true, port, ordinaryNative: false };
result.browser_errors = (await call('Runtime.evaluate', { expression: 'window.__checkErrors', returnByValue: true })).result.value;
// Close only the fresh headless process owned by this check.
socket.send(JSON.stringify({ id: ++sequence, method: 'Browser.close' }));
await new Promise(resolve => child.on('exit', resolve)); socket.close();
await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n');
console.log(JSON.stringify({ ...result, stack: undefined })); process.exitCode = result.status === 'pass' ? 0 : 1;
