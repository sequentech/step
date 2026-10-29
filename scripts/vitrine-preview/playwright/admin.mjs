// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The admin portal's Transparency tab for the election event: logs in through
// the fake Keycloak, opens the event, selects the tab and screenshots it.
//   run.sh admin playwright/admin.mjs OUT [origin] [width] [height]
const mod = await import("/pw/playwright-core/index.mjs");
const { chromium } = mod.default ?? mod;
const [origin = "http://127.0.0.1:18853", width = "1440", height = "1440"] = process.argv.slice(2);
const EVENT = process.env.EVENT ?? "demo-2026";

const browser = await chromium.launch();
const context = await browser.newContext({ viewport: { width: Number(width), height: Number(height) } });
const page = await context.newPage();
const problems = [];
page.on("pageerror", (e) => problems.push(`pageerror: ${String(e).slice(0, 240)}`));
page.on("console", (m) => { if (m.type() === "error" && !/Selector|%o|empty string|customCss/.test(m.text())) problems.push(`console.error: ${m.text().slice(0, 240)}`); });
const text = async () => (await page.locator("body").innerText()).replace(/\s+/g, " ").slice(0, 500);
try {
  await page.goto(`${origin}/sequent_backend_election_event/${EVENT}`);
  const tab = page.getByRole("tab", { name: /transparency/i }).first();
  await tab.waitFor({ state: "visible", timeout: 60000 });
  await tab.click();
  await page.waitForTimeout(Number(process.env.WAIT_MS ?? 6000));
  await page.screenshot({ path: "/out/Step-Admin.png" });
  console.log(await text());
} catch (e) {
  console.log(`STOPPED: ${String(e.message).split("\n")[0]}`);
  await page.screenshot({ path: "/out/Step-Admin-stopped.png" });
  console.log(await text());
  console.log("tabs:", (await page.getByRole("tab").allInnerTexts()).join(" | "));
}
console.log(problems.slice(0, 12).join("\n"));
await browser.close();
