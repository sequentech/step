#!/usr/bin/env node
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// Print the election administrator manual to one PDF per version and locale.
//
// Run after `yarn build`, with the same BASE_URL:
//
//   BASE_URL=/docusaurus/main yarn pdf
//
// The script serves ./build, opens the first page of each manual version,
// follows the "Next" links (the sidebar order) and prints all pages as one
// document with a cover and a table of contents. The files go to
// build/[<locale>/]pdf/sequent-admin-manual-<version>-<locale>.pdf, where the
// ManualPdfLink component links to them.

import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import puppeteer from 'puppeteer-core';

const siteDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const buildDir = path.join(siteDir, 'build');
const baseUrl = withSlashes(process.env.BASE_URL || '/step/');
const publicOrigin = process.env.PUBLIC_ORIGIN || 'https://docs.sequentech.io';
const defaultLocale = 'en';
const locales = ['en', 'es'];
const titles = {
  en: {
    title: 'Election Administrator Manual',
    contents: 'Contents',
    version: 'Version',
    generated: 'Generated',
    page: 'Page',
    of: 'of',
  },
  es: {
    title: 'Manual del administrador electoral',
    contents: 'Índice',
    version: 'Versión',
    generated: 'Generado',
    page: 'Página',
    of: 'de',
  },
};

function withSlashes(p) {
  return ('/' + p + '/').replace(/\/+/g, '/');
}

function chromePath() {
  const candidates = [
    process.env.CHROME_PATH,
    '/usr/bin/google-chrome',
    '/usr/bin/google-chrome-stable',
    '/usr/bin/chromium',
    '/usr/bin/chromium-browser',
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    '/Applications/Chromium.app/Contents/MacOS/Chromium',
  ].filter(Boolean);
  const found = candidates.find((c) => fs.existsSync(c));
  if (!found) throw new Error('No Chrome found. Set CHROME_PATH.');
  return found;
}

// The versions of the manual instance, newest first. The last released
// version is served without a version segment; the current docs as "next".
function manualVersions() {
  const released = JSON.parse(
    fs.readFileSync(path.join(siteDir, 'manual_versions.json'), 'utf8'),
  );
  return [
    ...released.map((name, i) => ({name, path: i === 0 ? '' : '/' + name})),
    {name: 'next', path: '/next'},
  ];
}

const mimeTypes = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript',
  '.css': 'text/css',
  '.json': 'application/json',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif': 'image/gif',
  '.webp': 'image/webp',
  '.ico': 'image/x-icon',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
  '.ttf': 'font/ttf',
};

// Serve the build directory below the site's base URL, resolving the
// extensionless URLs that `trailingSlash: false` produces.
function serve() {
  const server = http.createServer((req, res) => {
    const url = decodeURIComponent(new URL(req.url, 'http://x').pathname);
    if (!url.startsWith(baseUrl)) {
      res.writeHead(404).end();
      return;
    }
    const rel = path
      .normalize(url.slice(baseUrl.length))
      .replace(/^(\.\.[/\\])+/, '')
      .replace(/\/$/, '');
    const base = path.join(buildDir, rel);
    const file = [base, base + '.html', path.join(base, 'index.html')].find(
      (f) => f.startsWith(buildDir) && fs.existsSync(f) && fs.statSync(f).isFile(),
    );
    if (!file) {
      res.writeHead(404).end();
      return;
    }
    res.writeHead(200, {
      'content-type': mimeTypes[path.extname(file)] || 'application/octet-stream',
    });
    fs.createReadStream(file).pipe(res);
  });
  return new Promise((resolve) =>
    server.listen(0, '127.0.0.1', () => resolve(server)),
  );
}

async function loadPage(page, url) {
  await page.goto(url, {waitUntil: 'networkidle0'});
  // Load lazy images and let Mermaid finish rendering its diagrams.
  await page.evaluate(async () => {
    document.documentElement.setAttribute('data-theme', 'light');
    for (const img of document.querySelectorAll('article img')) img.loading = 'eager';
    await Promise.all(
      [...document.querySelectorAll('article img')].map((img) =>
        img.complete ? null : new Promise((r) => (img.onload = img.onerror = r)),
      ),
    );
  });
  await page
    .waitForFunction(
      () =>
        [...document.querySelectorAll('.docusaurus-mermaid-container')].every((c) =>
          c.querySelector('svg'),
        ),
      {timeout: 15000},
    )
    .catch(() => console.warn(`  Mermaid did not finish on ${url}`));
}

// Visit the manual in sidebar order and keep each page's article.
async function collect(page, startUrl) {
  const pages = [];
  const seen = new Set();
  let url = startUrl;
  while (url && !seen.has(url)) {
    seen.add(url);
    await loadPage(page, url);
    const data = await page.evaluate(() => {
      const article = document.querySelector('article');
      const next = document.querySelector('a.pagination-nav__link--next');
      return {
        path: location.pathname.replace(/\/$/, ''),
        title: article?.querySelector('h1')?.textContent?.trim() || document.title,
        html: article ? article.innerHTML : '',
        next: next ? next.href : null,
      };
    });
    if (!data.html) throw new Error(`No article on ${url}`);
    pages.push(data);
    url = data.next;
  }
  return pages;
}

async function printManual(browser, origin, version, locale) {
  const localePrefix = locale === defaultLocale ? '' : `${locale}/`;
  const startUrl = `${origin}${baseUrl}${localePrefix}manual${version.path}`;
  const page = await browser.newPage();
  await page.emulateMediaFeatures([{name: 'prefers-color-scheme', value: 'light'}]);
  const pages = await collect(page, startUrl);
  const t = titles[locale] || titles[defaultLocale];
  const label = version.name === 'next' ? 'Next' : version.name;
  let commit = '';
  try {
    commit = execFileSync('git', ['rev-parse', '--short', 'HEAD'], {cwd: siteDir})
      .toString()
      .trim();
  } catch {}

  // Rebuild the last visited page as the print document so the site's
  // stylesheets stay loaded.
  await page.evaluate(
    ({pages, t, label, locale, commit, publicOrigin, baseUrl}) => {
      const index = new Map(pages.map((p, i) => [p.path, i]));
      const body = document.createElement('div');
      body.className = 'manual-print';
      const today = new Date().toISOString().slice(0, 10);
      body.innerHTML = `
        <section class="manual-print__cover">
          <img src="${baseUrl}img/company_logo.svg" alt="Sequent">
          <h1>${t.title}</h1>
          <p class="manual-print__version">${t.version} ${label}</p>
          <p>${t.generated} ${today}${commit ? ` · ${commit}` : ''}</p>
        </section>
        <section class="manual-print__toc">
          <h1>${t.contents}</h1>
          <ol>${pages.map((p, i) => `<li><a href="#p${i}">${p.title}</a></li>`).join('')}</ol>
        </section>`;
      pages.forEach((p, i) => {
        const section = document.createElement('section');
        section.className = 'manual-print__page markdown';
        section.id = `p${i}`;
        section.innerHTML = p.html;
        // Keep heading anchors unique across pages.
        for (const el of section.querySelectorAll('[id]')) el.id = `p${i}-${el.id}`;
        for (const a of section.querySelectorAll('a[href]')) {
          const url = new URL(a.getAttribute('href'), location.origin + p.path);
          const target = index.get(url.pathname.replace(/\/$/, ''));
          if (url.origin === location.origin && target !== undefined) {
            a.setAttribute('href', url.hash ? `#p${target}-${url.hash.slice(1)}` : `#p${target}`);
          } else if (url.origin === location.origin) {
            a.setAttribute('href', publicOrigin + url.pathname + url.hash);
          }
        }
        // Videos cannot play on paper: print a link instead.
        for (const iframe of section.querySelectorAll('iframe')) {
          const link = document.createElement('p');
          link.innerHTML = `<a href="${iframe.src}">${iframe.src}</a>`;
          iframe.closest('.video-container, iframe').replaceWith(link);
        }
        for (const el of section.querySelectorAll(
          '.theme-doc-breadcrumbs, .theme-doc-version-badge, .theme-doc-footer, .hash-link, button',
        ))
          el.remove();
        body.appendChild(section);
      });
      document.documentElement.lang = locale;
      document.body.replaceChildren(body);
      const style = document.createElement('style');
      style.textContent = `
        :root { --ifm-background-color: #fff; --ifm-background-surface-color: #fff; }
        html, body, .manual-print section { background: #fff !important; color: #0F054C; }
        .manual-print { padding: 0; font-size: 11pt; }
        .manual-print__cover { height: 240mm; display: flex; flex-direction: column;
          justify-content: center; align-items: flex-start; }
        .manual-print__cover img { width: 60mm; margin-bottom: 20mm; }
        .manual-print__cover h1 { font-size: 30pt; }
        .manual-print__version { font-size: 16pt; font-weight: 600; }
        .manual-print__toc, .manual-print__page { break-before: page; }
        .manual-print__toc a { color: inherit; text-decoration: none; }
        .manual-print pre, .manual-print tr, .manual-print img,
        .manual-print .admonition, .manual-print .theme-admonition { break-inside: avoid; }
        .manual-print td:first-child a { white-space: nowrap; }
        .manual-print h2, .manual-print h3 { break-after: avoid; }
        .manual-print img { max-width: 100%; }
        .manual-print details { display: block; }
        .manual-print details > :not(summary) { display: block !important; }
      `;
      document.head.appendChild(style);
    },
    {pages, t, label, locale, commit, publicOrigin, baseUrl},
  );
  // Expand collapsed <details> blocks so their content prints.
  await page.evaluate(() =>
    document.querySelectorAll('details').forEach((d) => (d.open = true)),
  );

  const outDir = path.join(buildDir, localePrefix, 'pdf');
  fs.mkdirSync(outDir, {recursive: true});
  const out = path.join(outDir, `sequent-admin-manual-${version.name}-${locale}.pdf`);
  const small = 'font-size:8px;width:100%;padding:0 15mm;color:#555;font-family:sans-serif;';
  await page.pdf({
    path: out,
    format: 'A4',
    printBackground: true,
    displayHeaderFooter: true,
    headerTemplate: `<div style="${small}">${t.title} · ${t.version} ${label}</div>`,
    footerTemplate: `<div style="${small}text-align:right">${t.page} <span class="pageNumber"></span> ${t.of} <span class="totalPages"></span></div>`,
    margin: {top: '18mm', bottom: '18mm', left: '15mm', right: '15mm'},
  });
  await page.close();
  console.log(`${out} (${pages.length} pages of content)`);
}

async function main() {
  if (!fs.existsSync(buildDir)) throw new Error('Run `yarn build` first.');
  const server = await serve();
  const origin = `http://127.0.0.1:${server.address().port}`;
  const browser = await puppeteer.launch({
    executablePath: chromePath(),
    args: ['--no-sandbox', '--font-render-hinting=none'],
  });
  try {
    for (const locale of locales) {
      const localeBuild = locale === defaultLocale ? buildDir : path.join(buildDir, locale);
      if (!fs.existsSync(path.join(localeBuild, 'manual'))) {
        console.warn(`Skip locale ${locale}: not built`);
        continue;
      }
      for (const version of manualVersions()) {
        await printManual(browser, origin, version, locale);
      }
    }
  } finally {
    await browser.close();
    server.close();
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
