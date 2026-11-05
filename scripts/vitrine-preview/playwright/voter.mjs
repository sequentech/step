// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The voter's flow in Step's voting portal: log in, open the ballot, choose,
// review, cast, then watch the confirmation screen's receipt fill. Writes a
// screenshot per step to /out. Arguments: [origin] [width] [height] [seconds].
//   run.sh voter playwright/voter.mjs OUT
const mod = await import("/pw/playwright-core/index.mjs");
const { chromium } = mod.default ?? mod;
const [origin = "http://127.0.0.1:18850", width = "390", height = "844", watch = "60"] = process.argv.slice(2);
const TENANT = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
const EVENT = process.env.EVENT ?? "demo-2026";

const browser = await chromium.launch();
const context = await browser.newContext({ viewport: { width: Number(width), height: Number(height) } });
const page = await context.newPage();
const problems = [];
page.on("pageerror", (e) => problems.push(`pageerror: ${String(e).slice(0, 240)}`));
page.on("console", (m) => { if (/wasm interface|Failed to|Error loading/.test(m.text())) problems.push(`console.${m.type()}: ${m.text().slice(0, 500)}`); if (m.type() === "error" && !/Selector|customCss|empty string|%o/.test(m.text())) problems.push(`console.error: ${m.text().slice(0, 240)}`); });
page.on("response", (r) => { if (r.status() >= 400 && !r.url().includes("favicon")) problems.push(`http ${r.status()}: ${r.url().slice(0, 140)}`); });

page.on("framenavigated", (f) => { if (f === page.mainFrame()) console.log(`  navigated: ${f.url().replace(origin, "").slice(0, 110)}`); });
page.on("console", (m) => { if (/webpack-dev-server|\[HMR\]/.test(m.text())) console.log(`  dev: ${m.text().slice(0, 160)}`); });
const text = async () => (await page.locator("body").innerText()).replace(/\s+/g, " ").slice(0, 420);
const shot = async (name) => { await page.screenshot({ path: `/out/Step-${name}.png` }); console.log(`[${name}] ${page.url().replace(origin, "")}\n   ${await text()}`); };
const click = async (name, pattern) => {
  const target = page.getByRole("button", { name: pattern }).or(page.getByRole("link", { name: pattern })).first();
  await target.waitFor({ state: "visible", timeout: 30000 });
  await target.click();
  console.log(`click ${name}`);
};

try {
await page.goto(`${origin}/tenant/${TENANT}/event/${EVENT}/login`);
await click("vote", /click to vote|vote/i);
await page.waitForTimeout(1500);
await shot("1-start");
await click("start", /start voting|start/i);
await page.waitForTimeout(2000);
await shot("2-voting");
await page.getByText(process.env.CHOICE ?? "Maria Santos", { exact: false }).first().click();
await page.waitForTimeout(500);
await click("next", /next|review/i);
await page.waitForTimeout(3000);
await shot("3-review");
await click("cast", /cast/i);
await page.waitForTimeout(4000);
await shot("4-confirmation");
const seconds = Number(watch);
for (let t = 0; t < seconds; t += 15) {
  await page.waitForTimeout(15000);
  await shot(`5-receipt-${String(t + 15).padStart(3, "0")}s`);
}
} catch (e) {
  console.log(`STOPPED: ${String(e.message).split("\n")[0]}`);
  await page.screenshot({ path: "/out/Step-stopped.png" });
  console.log(`   ${await text()}`);
}
console.log(problems.slice(0, 25).join("\n"));
await context.storageState({ path: "/out/storage-state.json" });
await browser.close();
