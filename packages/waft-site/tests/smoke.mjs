/**
 * Smoke test for waft.rs (`packages/waft-site`): verifies every page, markdown route, LLM index,
 * agent playbook, search endpoint, sitemap, and homepage interactive controls with zero console errors.
 */
import { spawn } from 'node:child_process';
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const siteRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const SITE_URL = process.env.WAFT_SITE_URL?.replace(/\/$/, '');
const PORT = Number(process.env.WAFT_SITE_PORT ?? 3313);
const BASE = SITE_URL ?? `http://localhost:${PORT}`;
const ORIGIN = 'https://waft.rs';

const DOCS_PAGES = [
  ['/docs/start', 'Start'],
  ['/docs/architecture', 'Architecture'],
  ['/docs/state', 'State'],
  ['/docs/handles', 'Handles and ownership'],
  ['/docs/layout', 'Layout'],
  ['/docs/text-input', 'Text input'],
  ['/docs/kits', 'The kits'],
  ['/docs/pages', 'Pages'],
  ['/docs/shortcuts', 'Keys and shortcuts'],
  ['/docs/testing', 'Testing'],
  ['/docs/cli', 'CLI and packaging'],
  ['/docs/embedder', 'Embedder and hosts'],
  ['/docs/reference', 'Reference'],
];

const DOCS_RS_CRATES = [
  'waft',
  'waft-winui',
  'waft-cupertino',
  'waft-material',
  'waft-markdown',
  'waft-test',
  'waft-embedder',
  'waft-foundation',
];

let failures = 0;
function check(name, ok, detail = '') {
  console.log(`${ok ? 'ok  ' : 'FAIL'}  ${name}${detail ? `  — ${detail}` : ''}`);
  if (!ok) failures += 1;
}

function startServer() {
  return spawn('npx', ['next', 'start', '-p', String(PORT)], { cwd: siteRoot, stdio: 'ignore' });
}

async function waitForServer() {
  for (let attempt = 0; attempt < 60; attempt += 1) {
    try {
      const response = await fetch(BASE);
      if (response.ok) return;
    } catch {
      /* not listening yet */
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`the site never came up on ${BASE}`);
}

async function newContext(browser, options = {}) {
  return browser.newContext({ viewport: { width: 1200, height: 900 }, ...options });
}

async function checkRoute(path, type, start, init) {
  const response = await fetch(`${BASE}${path}`, init);
  const body = await response.text();
  check(`${path} answers`, response.ok, `${response.status}`);
  check(`${path} is ${type}`, (response.headers.get('content-type') ?? '').startsWith(type));
  check(`${path} starts with its title`, body.startsWith(start), body.split('\n')[0]);
  return body;
}

async function checkMarkdownRoutes() {
  const index = await checkRoute('/llms.txt', 'text/markdown', '# Waft');
  const listed = DOCS_PAGES.every(([path]) => index.includes(`(${path})`));
  check('llms.txt lists every docs page', listed);
  const full = await checkRoute('/llms-full.txt', 'text/markdown', '# ');
  const whole = DOCS_PAGES.every(([path]) => full.includes(`(${ORIGIN}${path})`));
  check('llms-full.txt has every docs page, by its URL on waft.rs', whole);
  for (const [path, title] of DOCS_PAGES) {
    await checkRoute(`${path}.md`, 'text/markdown', `# ${title} (${ORIGIN}${path})`);
  }
  const asked = { headers: { Accept: 'text/markdown' } };
  await checkRoute('/docs/architecture', 'text/markdown', '# Architecture', asked);
  const playbook = await checkRoute('/agents.md', 'text/markdown', '# Build an app with Waft');
  check('the playbook links waft.rs/docs', playbook.includes(`${ORIGIN}/docs`));
}

async function checkSiteRoutes() {
  const sitemap = await (await fetch(`${BASE}/sitemap.xml`)).text();
  const urls = [ORIGIN, ...DOCS_PAGES.map(([path]) => `${ORIGIN}${path}`)];
  check('the sitemap lists every page on waft.rs', urls.every((url) => sitemap.includes(`${url}<`)));
  const robots = await (await fetch(`${BASE}/robots.txt`)).text();
  check('robots.txt points at the sitemap', robots.includes(`Sitemap: ${ORIGIN}/sitemap.xml`));
  const search = await fetch(`${BASE}/api/search?query=Entity`);
  const results = search.ok ? await search.json() : [];
  check('search finds the state guide', results.some((result) => result.url.startsWith('/docs/state')));
  const docs = await fetch(`${BASE}/docs`, { redirect: 'manual' });
  check('/docs opens on its first page', docs.headers.get('location') === '/docs/start');
}

async function checkPage(page, path, title) {
  const response = await page.goto(`${BASE}${path}`, { waitUntil: 'load' });
  const heading = (await page.locator('h1').first().innerText()).trim();
  check(`${path} answers`, response?.ok() === true, `${response?.status()}`);
  check(`${path} is titled ${title}`, heading === title, heading);
}

async function checkDocsPages(page) {
  for (const [path, title] of DOCS_PAGES) {
    await checkPage(page, path, title);
    check(`${path}'s document is titled ${title} — Waft`, (await page.title()) === `${title} — Waft`);
  }
}

async function checkReference(page) {
  await page.goto(`${BASE}/docs/reference`, { waitUntil: 'load' });
  const links = await page.locator('article a').evaluateAll((els) => els.map((el) => el.getAttribute('href')));
  const crates = DOCS_RS_CRATES.map((crate) => `https://docs.rs/${crate}/latest/${crate.replaceAll('-', '_')}/`);
  check('the reference links each main crate on docs.rs', crates.every((url) => links.includes(url)));
}

async function checkHome(page) {
  await checkPage(page, '/', 'Solid, fluent apps in Rust.\nDesktop, mobile, web, and WASI.');
  check("the home page's document is titled Waft", (await page.title()) === 'Waft');
  const bodyText = await page.locator('#nd-home-layout').innerText();
  check('the home page has no middot splits', !bodyText.includes('·'));
  const links = await page.locator('#nd-home-layout a').evaluateAll((els) => els.map((el) => el.getAttribute('href')));
  check('links WinUI Gallery', links.includes('https://winui-gallery.inset.rs'));
  check('links Cupertino Gallery', links.includes('https://cupertino-gallery.inset.rs'));
  check('links Material Gallery', links.includes('https://material-gallery.pages.dev'));
  await page.getByRole('button', { name: /Cupertino\s+waft-cupertino/ }).click();
  const cupertinoCode = await page.locator('pre').first().innerText();
  check('switching to Cupertino updates the studio code', cupertinoCode.includes('CupertinoApp::new()'));
  await page.getByRole('button', { name: /Material 3\s+waft-material/ }).click();
  const materialCode = await page.locator('pre').first().innerText();
  check('switching to Material 3 updates the studio code', materialCode.includes('MaterialApp::new()'));
  const allPre = (await page.locator('pre').allInnerTexts()).join('\n');
  check('the CLI workflow shows cargo waft new notes', allPre.includes('cargo waft new notes'));
}

const server = SITE_URL ? undefined : startServer();
let browser;
try {
  await waitForServer();
  await checkMarkdownRoutes();
  await checkSiteRoutes();

  browser = await chromium.launch({ headless: true });
  const context = await newContext(browser, { permissions: ['clipboard-read', 'clipboard-write'] });
  const page = await context.newPage();
  const consoleErrors = [];
  await page.route('https://*.inset.rs/**', (route) => route.fulfill({ status: 200, contentType: 'text/html', body: '<!doctype html><html><body></body></html>' }));
  await page.route('https://*.pages.dev/**', (route) => route.fulfill({ status: 200, contentType: 'text/html', body: '<!doctype html><html><body></body></html>' }));
  page.on('console', (message) => message.type() === 'error' && consoleErrors.push(message.text()));
  page.on('pageerror', (error) => consoleErrors.push(`pageerror: ${error.message}`));

  await checkHome(page);
  await checkDocsPages(page);
  await checkReference(page);

  const noisy = consoleErrors.filter((text) => !/favicon|Failed to load resource/.test(text));
  check('no console errors', noisy.length === 0, noisy.slice(0, 3).join(' | '));
} finally {
  await browser?.close();
  server?.kill();
}

console.log(failures === 0 ? '\nsite smoke: all checks passed' : `\nsite smoke: ${failures} failed`);
process.exit(failures === 0 ? 0 : 1);
