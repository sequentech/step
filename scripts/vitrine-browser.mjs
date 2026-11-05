// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Run the built ballot verifier against an actual public Step queue receipt.
// Usage: node scripts/vitrine-browser.mjs BUILD_DIR PUBLIC_ARTIFACT_DIR OUTPUT_DIR
// WINDMILL_VITRINE_TEST_ARTIFACT_DIR produces the public input files. This
// checks the production receipt route; it does not deploy Hasura or Keycloak.
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { createRequire } from "node:module";

const [buildArg, artifactArg, outArg] = process.argv.slice(2);
assert.ok(
  buildArg && artifactArg && outArg,
  "provide build, public artifacts and output directories",
);
const build = path.resolve(buildArg);
const artifacts = path.resolve(artifactArg);
const out = path.resolve(outArg);
const require = createRequire(
  new URL("../packages/package.json", import.meta.url),
);
const { chromium } = require(
  process.env.STEP_VITRINE_PLAYWRIGHT_MODULE || "playwright-core",
);
const policy = await fs.readFile(path.join(artifacts, "policy.note"), "utf8");
const chain = await fs.readFile(path.join(artifacts, "ballot.chain"), "utf8");
const expected = JSON.parse(
  await fs.readFile(path.join(artifacts, "context.json"), "utf8"),
);
assert.equal(expected.source, "live-native-sidecar");
assert.ok(
  chain.includes("policy-ack "),
  "retain the log's actual acknowledgement",
);
assert.match(policy, /profile ballots vitrine\/election\/ballots\/v1 /);
assert.match(policy, /profile events vitrine\/election\/events\/v1 /);
const nativeOrigins = new Set(
  [...policy.matchAll(/https?:\/\/[^\s]+/g)].map(
    (match) => new URL(match[0]).origin,
  ),
);
await fs.mkdir(out, { recursive: true });
const settings = JSON.parse(
  await fs.readFile(path.join(build, "global-settings.json"), "utf8"),
);
settings.DISABLE_AUTH = true;
settings.HASURA_URL = "http://127.0.0.1:1/unused-receipt-route";
settings.KEYCLOAK_URL = "http://127.0.0.1:1/unused-receipt-route";
const mime = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".json": "application/json",
  ".css": "text/css",
  ".wasm": "application/wasm",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".ico": "image/x-icon",
};
const server = http.createServer(async (request, response) => {
  response.setHeader("Cross-Origin-Opener-Policy", "same-origin");
  response.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
  response.setHeader("Cross-Origin-Resource-Policy", "cross-origin");
  response.setHeader("Cache-Control", "no-store");
  try {
    const name = decodeURIComponent(
      new URL(request.url, "http://localhost").pathname,
    );
    if (name === "/global-settings.json") {
      response.setHeader("Content-Type", "application/json");
      response.end(JSON.stringify(settings));
      return;
    }
    let file = path.resolve(build, "." + name);
    assert.ok(
      file === build || file.startsWith(build + path.sep),
      "static path traversal",
    );
    if (name === "/receipt" || name.startsWith("/receipt/"))
      file = path.join(build, "index.html");
    const bytes = await fs.readFile(file);
    response.setHeader(
      "Content-Type",
      mime[path.extname(file)] || "application/octet-stream",
    );
    response.end(bytes);
  } catch {
    response.statusCode = 404;
    response.end("not found");
  }
});
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const address = server.address();
const base = `http://127.0.0.1:${address.port}`;
const browser = await chromium.launch({ headless: true });
const results = [];
const failures = [];
let nativeRequests = 0;
const errors = [];
const axe = require.resolve("axe-core/axe.min.js");
const screenshot = async (page, label) => {
  const wide = await page.evaluate(
    () => document.documentElement.scrollWidth > innerWidth + 1,
  );
  assert.equal(wide, false, `${label}: horizontal overflow`);
  await page.screenshot({
    path: path.join(out, `${label}.png`),
    fullPage: true,
  });
};
const open = async (context, fileInput = false) => {
  const page = await context.newPage();
  page.on("pageerror", (error) => errors.push(error.message));
  await page.route("**/*", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    if (url.origin === base) return route.continue();
    if (!nativeOrigins.has(url.origin)) {
      failures.push(`unexpected request origin ${url.origin}`);
      return route.abort();
    }
    nativeRequests++;
    const headers = await request.allHeaders();
    if (headers.cookie || headers.referer || headers.authorization)
      failures.push(
        `credentials/referrer on public proof request ${url.origin}`,
      );
    return route.continue();
  });
  await page.goto(`${base}/receipt`);
  await page.getByTestId("receipt-paste").waitFor({ timeout: 60000 });
  if (fileInput) {
    await page
      .locator('input[type="file"]')
      .first()
      .setInputFiles({
        name: "ballot.vitrine",
        mimeType: "text/plain",
        buffer: Buffer.from(chain),
      });
  } else {
    await page.getByTestId("receipt-paste").fill(chain);
    await page.getByTestId("receipt-open").click();
  }
  await page.getByTestId("policy-input").waitFor();
  return page;
};
try {
  for (const width of [1280, 390]) {
    const context = await browser.newContext({
      viewport: { width, height: 900 },
      locale: "en-US",
      ...(process.env.STEP_VITRINE_RECORD_VIDEO === "1"
        ? {
            recordVideo: {
              dir: path.join(out, "videos"),
              size: { width, height: 900 },
            },
          }
        : {}),
    });
    const page = await open(context, width === 390);
    await page.getByTestId("policy-input").fill(policy);
    await page.getByTestId("policy-load").click();
    await page.getByTestId("policy-offer").waitFor();
    assert.equal(
      (
        await page
          .getByTestId("policy-offer")
          .getByTestId("policy-id")
          .innerText()
      ).trim(),
      expected.policyId,
    );
    await page.getByTestId("policy-pin").click();
    await page.getByTestId("ballot-id").fill(expected.ballotId);
    assert.equal(
      await page.getByTestId("ballot-event").inputValue(),
      expected.eventId,
    );
    assert.equal(
      await page.getByTestId("ballot-election").inputValue(),
      expected.electionId,
    );
    await page.getByTestId("ballot-check").click();
    const receiptStatus = page.getByTestId("receipt-status");
    await page
      .locator('[data-testid="receipt-status"][data-status="verified"]')
      .waitFor({ timeout: 30000 });
    const steps = await receiptStatus.evaluate((element) =>
      Object.fromEntries(
        [...element.querySelectorAll('[data-testid^="status-row-"]')].map(
          (row) => [
            row.dataset.testid.slice("status-row-".length),
            row.dataset.state,
          ],
        ),
      ),
    );
    await fs.writeFile(
      path.join(out, `receipt-result-${width}.json`),
      JSON.stringify(steps, null, 2),
    );
    assert.deepEqual(
      steps,
      { record: "done", witnesses: "done", anchors: "done" },
      "every step of the receipt",
    );
    await receiptStatus.evaluate((element) =>
      element.scrollIntoView({ block: "start" }),
    );
    await screenshot(page, `native-receipt-${width}`);
    await page.addScriptTag({ path: axe });
    const accessibility = await page.evaluate(async () => {
      const result = await window.axe.run(
        document.querySelector('[data-testid="receipt-result"]'),
        { resultTypes: ["violations"] },
      );
      return result.violations.map((violation) => ({
        id: violation.id,
        targets: violation.nodes.map((node) => node.target),
      }));
    });
    assert.deepEqual(accessibility, [], "receipt result accessibility");
    await page.getByTestId("ballot-change").click();
    await page.getByTestId("ballot-id").fill("00".repeat(32));
    await page.getByTestId("ballot-check").click();
    await page
      .locator(
        '[data-testid="receipt-status"]:is([data-status="problem"], [data-status="late"])',
      )
      .waitFor({ timeout: 30000 });
    await screenshot(page, `wrong-ballot-${width}`);
    results.push({
      width,
      liveNativeReceipt: true,
      input: width === 390 ? "receipt file" : "pasted chain",
      steps,
      wrongBallotRefused: true,
      receiptAxeViolations: 0,
    });
    const recorded = page.video();
    await context.close();
    if (recorded)
      await fs.rename(
        await recorded.path(),
        path.join(out, "videos", `native-receipt-${width}.webm`),
      );
    const invalidContext = await browser.newContext({
      viewport: { width, height: 900 },
      locale: "en-US",
    });
    const invalid = await open(invalidContext);
    assert.ok(policy.includes("required on\n"));
    await invalid
      .getByTestId("policy-input")
      .fill(policy.replace("required on\n", "required off\n"));
    await invalid.getByTestId("policy-load").click();
    await invalid.getByTestId("policy-invalid").waitFor();
    assert.equal(await invalid.getByTestId("policy-pin").count(), 0);
    await screenshot(invalid, `tampered-policy-${width}`);
    results.push({ width, tamperedPolicyRefused: true });
    await invalidContext.close();
  }
  assert.deepEqual(failures, []);
  assert.deepEqual(errors, []);
  assert.ok(
    nativeRequests > 0,
    "the production verifier actually fetched its log proofs",
  );
  const summary = {
    source:
      "real Step queue and native sidecar; production ballot-verifier /receipt",
    policyId: expected.policyId,
    vitrineRevision: (
      await fs.readFile(
        new URL("../packages/vendor-npm/VITRINE_COMMIT", import.meta.url),
        "utf8",
      )
    ).trim(),
    results,
    nativeRequests,
    errors,
    failures,
    coverage:
      "No authenticated Step/Hasura/Keycloak deployment; cryptographic input is a synthetic accepted ballot.",
  };
  await fs.writeFile(
    path.join(out, "results.json"),
    JSON.stringify(summary, null, 2) + "\n",
  );
  console.log(JSON.stringify(summary, null, 2));
} finally {
  await browser.close();
  await new Promise((resolve) => server.close(resolve));
}
