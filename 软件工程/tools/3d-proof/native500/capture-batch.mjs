// Reuse the existing installed Playwright and product reviewer; no per-batch wrapper needed.
import {createRequire} from 'node:module';

const [playwrightModule, batch, stagesText = 'views', ...devices] = process.argv.slice(2);
if (!playwrightModule || playwrightModule === '--help' || !batch) {
  console.log('node capture-batch.mjs <Playwright module> <B02-Qxx> <coarse|views|feet|mechanisms[,stage...]> [device-id ...]');
  console.log('views: seven directions per LOD + one mechanism close-up; device selection is supported for views only.');
  process.exit(playwrightModule === '--help' ? 0 : 1);
}
const stages = stagesText.split(',');
if (!/^B\d{2}-Q\d{2}(?:-R\d+)?$/.test(batch)) throw Error('Invalid review batch: ' + batch);
if (stages.some(stage => !['coarse', 'views', 'feet', 'mechanisms'].includes(stage))) throw Error('Unknown capture stage');
if (devices.length && stages.some(stage => stage !== 'views')) throw Error('Device selection is supported for views only; batch-specific coarse hooks still cover their batch.');
const {chromium} = createRequire(import.meta.url)(playwrightModule);
const browser = await chromium.launch({headless: true, channel: 'msedge'});
try {
  const page = await browser.newPage({viewport: {width: 1660, height: 1150}});
  page.on('pageerror', error => process.stderr.write(error.message + '\n'));
  const origin = 'http://127.0.0.1:19291';
  for (const stage of stages) {
    if (stage === 'views') {
      const query = new URLSearchParams({review: batch, capture: 'lean'});
      if (devices.length) query.set('devices', devices.join(','));
      await page.goto(origin + '/tools/3d-proof/native500/index.html?' + query);
      await page.waitForFunction(() => document.getElementById('details').textContent.length > 10);
      await page.evaluate(() => document.getElementById('capture').onclick());
      const status = await page.locator('#status').innerText();
      if (!status.startsWith('完成：')) throw Error(status);
      console.log(status);
    } else {
      const method = {coarse: 'checkBatch', feet: 'checkFeet', mechanisms: 'checkMechanisms'}[stage];
      await page.goto(origin + '/tools/3d-proof/native500/' + batch.toLowerCase().replaceAll('-', '') + '/coarse.html');
      await page.waitForFunction(name => typeof window[name] === 'function', method);
      const result = await page.evaluate(name => window[name](), method);
      console.log(JSON.stringify({batch, stage, result}));
    }
  }
} finally {
  await browser.close();
}
