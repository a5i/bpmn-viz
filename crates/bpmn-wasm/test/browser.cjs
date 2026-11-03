// Browser smoke via Playwright Chromium: serve the crate root, load
// test/browser.html against the pkg-web artifact, require PASS.
const http = require('node:http');
const fs = require('node:fs/promises');
const path = require('node:path');

const root = path.resolve(__dirname, '..');
const mime = {
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.mjs': 'text/javascript',
  '.wasm': 'application/wasm',
};

async function main() {
  const { chromium } = require('playwright');
  const server = http.createServer(async (req, res) => {
    try {
      const file = path.resolve(root, `.${new URL(req.url, 'http://localhost').pathname}`);
      if (!file.startsWith(root + path.sep)) throw new Error('Outside root');
      const body = await fs.readFile(file);
      res.writeHead(200, { 'Content-Type': mime[path.extname(file)] ?? 'application/octet-stream' });
      res.end(body);
    } catch {
      res.writeHead(404);
      res.end();
    }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const url = `http://127.0.0.1:${server.address().port}/test/browser.html`;
  try {
    const browser = await chromium.launch();
    try {
      const page = await browser.newPage();
      await page.goto(url, { waitUntil: 'load' });
      await page.waitForFunction(
        () => !document.getElementById('result')?.textContent?.includes('RUNNING'),
        undefined,
        { timeout: 30000 },
      );
      const outcome = await page.textContent('#result');
      if (outcome !== 'PASS') throw new Error(`Browser smoke failed: ${outcome}`);
      console.log('Browser WASM smoke: PASS');
    } finally {
      await browser.close();
    }
  } finally {
    await new Promise(resolve => server.close(resolve));
  }
}

main().catch(error => {
  console.error(error);
  process.exitCode = 1;
});
