// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Opens a URL, waits, and reports what the page shows: a screenshot, the
// visible text, console errors and failed requests. A debugging aid.
//   run.sh explore playwright/explore.mjs OUT URL [width height seconds]
const { chromium } = (await import("/pw/playwright-core/index.mjs")).default ?? (await import("/pw/playwright-core/index.mjs"));
const [url, width = "1280", height = "900", seconds = "8"] = process.argv.slice(2);
const browser = await chromium.launch();
const context = await browser.newContext({ viewport: { width: Number(width), height: Number(height) } });
const page = await context.newPage();
const lines = [];
page.on("console", (m) => { if (["error", "warning"].includes(m.type())) lines.push(`console.${m.type()}: ${m.text().slice(0, 300)}`); });
page.on("pageerror", (e) => lines.push(`pageerror: ${String(e).slice(0, 300)}`));
page.on("requestfailed", (r) => lines.push(`requestfailed: ${r.url().slice(0, 160)} ${r.failure()?.errorText}`));
if (process.env.TRACE) {
  page.on("response", async (r) => {
    if (!r.url().includes(":18851/")) return;
    let op = "";
    try { op = JSON.parse(r.request().postData() ?? "{}").operationName ?? ""; } catch { /* not JSON */ }
    let err = "";
    if (op) { try { const j = await r.json(); if (j.errors) err = ` ERRORS: ${j.errors.map((e) => e.message).join("; ").slice(0, 200)}`; } catch { /* no body */ } }
    lines.push(`${r.request().method()} ${r.url().replace("http://127.0.0.1:18851", "").slice(0, 100)} ${op} -> ${r.status()}${err}`);
  });
}
page.on("response", (r) => { if (r.status() >= 400) lines.push(`http ${r.status()}: ${r.url().slice(0, 160)}`); });
await page.goto(url, { waitUntil: "domcontentloaded" });
await page.waitForTimeout(Number(seconds) * 1000);
await page.screenshot({ path: "/out/explore.png" });
console.log("url:", page.url());
console.log("title:", await page.title());
console.log("text:", (await page.locator("body").innerText()).replace(/\s+/g, " ").slice(0, 700));
console.log(lines.slice(0, 40).join("\n"));
await browser.close();
