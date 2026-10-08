#!/usr/bin/env node
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// Print the election administrator manual to one PDF per locale.
//
// Run after `yarn build`, with the same BASE_URL:
//
//   BASE_URL=/docusaurus/main yarn pdf
//
// The script serves ./build, opens the first page of the Election Managers
// section, follows the "Next" links (the sidebar order) while they stay in
// that section, and prints all pages as one document with a cover and a table
// of contents. The files go to
// build/[<locale>/]pdf/sequent-admin-manual-<version>-<locale>.pdf, where the
// ManualPdfLink component links to them.

import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {createRequire} from 'node:module';
import puppeteer from 'puppeteer-core';

const siteDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const buildDir = path.join(siteDir, 'build');
const baseUrl = withSlashes(process.env.BASE_URL || '/step/');
const publicOrigin = process.env.PUBLIC_ORIGIN || 'https://docs.sequentech.io';
const {versionName} = createRequire(import.meta.url)('../docs-version.js');
const version = versionName(process.env.BASE_URL);
// The manual is the Election Managers section of the docs.
const manualPath = 'docs/election_managers';
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
    video: 'This page has a video in the online documentation.',
    watch: 'Watch the video',
  },
  es: {
    title: 'Manual del administrador electoral',
    contents: 'Índice',
    version: 'Versión',
    generated: 'Generado',
    page: 'Página',
    of: 'de',
    video: 'Esta página tiene un vídeo en la documentación en línea.',
    watch: 'Ver el vídeo',
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
  // A slow page load in CI should not fail the whole PDF: try once more.
  try {
    await page.goto(url, {waitUntil: 'networkidle0', timeout: 60000});
  } catch (err) {
    console.warn(`  Retrying ${url}: ${err.message}`);
    await page.goto(url, {waitUntil: 'networkidle0', timeout: 60000});
  }
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

// Expand every category of the section in the sidebar, then read the sidebar
// tree: [{label, path (or null for a category without a page), children}].
async function readSidebar(page, startUrl, sectionPath) {
  await loadPage(page, startUrl);
  const findRoot = `(() => {
    const norm = (h) => new URL(h, location.href).pathname.replace(/\\/$/, '');
    const link = [...document.querySelectorAll('.theme-doc-sidebar-menu a.menu__link')]
      .find((a) => norm(a.getAttribute('href') || '#') === ${JSON.stringify(sectionPath + '/election_management')});
    return link && link.closest('li');
  })()`;
  for (let i = 0; i < 100; i++) {
    const clicked = await page.evaluate(`(() => {
      const root = ${findRoot};
      const li = root && root.querySelector('li.theme-doc-sidebar-item-category.menu__list-item--collapsed');
      if (!li) return false;
      const toggle =
        li.querySelector(':scope > .menu__list-item-collapsible > button.menu__caret') ||
        li.querySelector(':scope > .menu__list-item-collapsible > a.menu__link');
      toggle.click();
      return true;
    })()`);
    if (!clicked) break;
    await new Promise((r) => setTimeout(r, 150));
  }
  const tree = await page.evaluate(`(() => {
    const norm = (h) => new URL(h, location.href).pathname.replace(/\\/$/, '');
    const parse = (ul) =>
      ul ? [...ul.children].map((li) => {
        const a = li.querySelector(':scope > .menu__list-item-collapsible > a, :scope > a');
        const isCategory = li.classList.contains('theme-doc-sidebar-item-category');
        const hasPage = a && !a.classList.contains('menu__link--sublist-caret') &&
          (a.getAttribute('href') || '#') !== '#';
        return {
          label: a ? a.textContent.trim() : '',
          path: hasPage ? norm(a.getAttribute('href')) : null,
          children: isCategory ? parse(li.querySelector(':scope > ul')) : [],
        };
      }) : [];
    const root = ${findRoot};
    if (!root) return null;
    const a = root.querySelector(':scope > .menu__list-item-collapsible > a');
    return [
      {label: a.textContent.trim(), path: norm(a.getAttribute('href')), children: []},
      ...parse(root.querySelector(':scope > ul')),
    ];
  })()`);
  if (!tree) throw new Error(`No sidebar section for ${sectionPath}`);
  return tree;
}

// Visit each page of the tree in sidebar order and keep its article.
async function collect(page, origin, tree) {
  const pages = [];
  const seen = new Set();
  const walk = async (nodes) => {
    for (const node of nodes) {
      if (node.path && !seen.has(node.path)) {
        seen.add(node.path);
        await loadPage(page, origin + node.path);
        const {html, generated} = await page.evaluate(() => ({
          html: document.querySelector('article')?.innerHTML || '',
          generated: !!document.querySelector('[class*="generatedIndexPage"]'),
        }));
        if (generated) {
          // A category overview of cards: print a heading page instead.
          node.path = null;
        } else {
          if (!html) throw new Error(`No article on ${node.path}`);
          pages.push({path: node.path, html});
        }
      }
      await walk(node.children);
    }
  };
  await walk(tree);
  return pages;
}

async function printManual(browser, origin, locale) {
  const localePrefix = locale === defaultLocale ? '' : `${locale}/`;
  const sectionPath = `${baseUrl}${localePrefix}${manualPath}`;
  const page = await browser.newPage();
  await page.emulateMediaFeatures([{name: 'prefers-color-scheme', value: 'light'}]);
  const tree = await readSidebar(page, `${origin}${sectionPath}/election_management`, sectionPath);
  const pages = await collect(page, origin, tree);
  const t = titles[locale] || titles[defaultLocale];
  const label = version === 'main' ? 'Next' : version;
  let commit = '';
  try {
    commit = execFileSync('git', ['rev-parse', '--short', 'HEAD'], {cwd: siteDir})
      .toString()
      .trim();
  } catch {}

  // Rebuild the last visited page as the print document so the site's
  // stylesheets stay loaded.
  await page.evaluate(
    ({tree, pages, t, label, locale, commit, publicOrigin, baseUrl}) => {
      const index = new Map(pages.map((p, i) => [p.path, i]));
      const html = new Map(pages.map((p) => [p.path, p.html]));
      // The contents follow the sidebar: one numbered list item for each
      // page or category, nested as in the sidebar.
      let parts = 0;
      const setAnchors = (nodes) => nodes.forEach((n) => {
        n.anchor = n.path ? `p${index.get(n.path)}` : `part${parts++}`;
        setAnchors(n.children);
      });
      setAnchors(tree);
      const toc = (nodes) =>
        `<ol>${nodes.map((n) => `<li><a href="#${n.anchor}">${n.label}</a>${n.children.length ? toc(n.children) : ''}</li>`).join('')}</ol>`;
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
          ${toc(tree)}
        </section>`;
      const printed = new Set();
      // Move the headings of a page down by its depth in the sidebar, so the
      // bookmarks of the PDF have the structure of the sidebar. data-level
      // keeps the original size on paper.
      const shiftHeadings = (section, depth) => {
        if (!depth) return;
        for (const h of section.querySelectorAll('h1, h2, h3, h4, h5, h6')) {
          const level = Number(h.tagName[1]);
          const moved = document.createElement(`h${Math.min(6, level + depth)}`);
          for (const attr of h.attributes) moved.setAttribute(attr.name, attr.value);
          moved.dataset.level = level;
          moved.innerHTML = h.innerHTML;
          h.replaceWith(moved);
        }
      };
      const emit = (nodes, depth) => nodes.forEach((n) => {
        if (n.path && !printed.has(n.path)) {
          printed.add(n.path);
          emitPage({path: n.path, html: html.get(n.path)}, index.get(n.path), depth);
        } else if (!n.path) {
          const part = document.createElement('section');
          part.className = 'manual-print__part';
          part.id = n.anchor;
          part.innerHTML = `<h1>${n.label}</h1>`;
          shiftHeadings(part, depth);
          body.appendChild(part);
        }
        emit(n.children, depth + 1);
      });
      const emitPage = (p, i, depth) => {
        const section = document.createElement('section');
        section.className = 'manual-print__page markdown';
        section.id = `p${i}`;
        section.innerHTML = p.html;
        // Keep heading anchors unique across pages.
        // Diagrams are skipped: Mermaid styles its SVG through the SVG's id.
        for (const el of section.querySelectorAll('[id]'))
          if (!el.closest('svg')) el.id = `p${i}-${el.id}`;
        for (const a of section.querySelectorAll('a[href]')) {
          const url = new URL(a.getAttribute('href'), location.origin + p.path);
          const target = index.get(url.pathname.replace(/\/$/, ''));
          if (url.origin === location.origin && target !== undefined) {
            a.setAttribute('href', url.hash ? `#p${target}-${url.hash.slice(1)}` : `#p${target}`);
          } else if (url.origin === location.origin) {
            a.setAttribute('href', publicOrigin + url.pathname + url.hash);
          }
        }
        // Videos cannot play on paper: say that there is one, with a link.
        for (const iframe of section.querySelectorAll('iframe')) {
          const note = document.createElement('p');
          note.className = 'manual-print__video';
          note.innerHTML = `▶ ${t.video} <a href="${iframe.src}">${t.watch}</a>`;
          iframe.closest('.video-container, iframe').replaceWith(note);
        }
        for (const el of section.querySelectorAll(
          '.theme-doc-breadcrumbs, .theme-doc-version-badge, .theme-doc-footer, .hash-link, button',
        ))
          el.remove();
        shiftHeadings(section, depth);
        body.appendChild(section);
      };
      emit(tree, 0);
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
        .manual-print__toc, .manual-print__page, .manual-print__part { break-before: page; }
        .manual-print__part { padding-top: 80mm; }
        .manual-print__toc a { color: inherit; text-decoration: none; }
        .manual-print__toc ol { counter-reset: item; list-style: none; padding-left: 1.5em; }
        .manual-print__toc > ol { padding-left: 0; }
        .manual-print__toc li { counter-increment: item; margin: 0.15em 0; }
        .manual-print__toc li::before { content: counters(item, '.') '. '; }
        .manual-print__toc > ol > li { font-weight: 600; margin-top: 0.6em; }
        .manual-print__toc li li { font-weight: 400; }
        .manual-print [data-level='1'] { font-size: var(--ifm-h1-font-size); }
        .manual-print [data-level='2'] { font-size: var(--ifm-h2-font-size); }
        .manual-print [data-level='3'] { font-size: var(--ifm-h3-font-size); }
        .manual-print [data-level='4'] { font-size: var(--ifm-h4-font-size); }
        .manual-print pre, .manual-print tr, .manual-print img,
        .manual-print .admonition, .manual-print .theme-admonition { break-inside: avoid; }
        .manual-print td:first-child a { white-space: nowrap; }
        .manual-print h2, .manual-print h3 { break-after: avoid; }
        .manual-print img { max-width: 100%; }
        .manual-print__video { border: 1px solid #B7C2E2; border-radius: 4px; padding: 0.5em 0.8em; }
        .manual-print details { display: block; }
        .manual-print details > :not(summary) { display: block !important; }
      `;
      document.head.appendChild(style);
    },
    {tree, pages, t, label, locale, commit, publicOrigin, baseUrl},
  );
  // Expand collapsed <details> blocks so their content prints.
  await page.evaluate(() =>
    document.querySelectorAll('details').forEach((d) => (d.open = true)),
  );

  const outDir = path.join(buildDir, localePrefix, 'pdf');
  fs.mkdirSync(outDir, {recursive: true});
  const out = path.join(outDir, `sequent-admin-manual-${version}-${locale}.pdf`);
  const small = 'font-size:8px;width:100%;padding:0 15mm;color:#555;font-family:sans-serif;';
  await page.pdf({
    path: out,
    format: 'A4',
    printBackground: true,
    // PDF bookmarks from the headings, nested like the sidebar.
    outline: true,
    tagged: true,
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
      if (!fs.existsSync(path.join(localeBuild, manualPath))) {
        console.warn(`Skip locale ${locale}: not built`);
        continue;
      }
      await printManual(browser, origin, locale);
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
