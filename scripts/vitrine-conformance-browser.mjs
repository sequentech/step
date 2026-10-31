// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Exercise the installed browser SDK and the compiled native counterpart on
// immutable upstream receipt-chain vectors. No live services are contacted.
// Usage: node scripts/vitrine-conformance-browser.mjs VITRINE_CHECKOUT OUTPUT_DIR
// Build that checkout's @vitrine/verify-wasm before running; its production
// source must match packages/vendor-npm/VITRINE_COMMIT.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs/promises";
import http from "node:http";
import { createRequire } from "node:module";
import path from "node:path";
import { pathToFileURL } from "node:url";

const [checkoutArg, outputArg] = process.argv.slice(2);
assert.ok(
  checkoutArg && outputArg,
  "provide Vitrine checkout and new output directory",
);
const checkout = path.resolve(checkoutArg);
const output = path.resolve(outputArg);
const require = createRequire(
  new URL("../packages/package.json", import.meta.url),
);
const { build } = require("esbuild");
const { chromium } = require(
  process.env.STEP_VITRINE_PLAYWRIGHT_MODULE || "playwright-core",
);
const revision = (
  await fs.readFile(
    new URL("../packages/vendor-npm/VITRINE_COMMIT", import.meta.url),
    "utf8",
  )
).trim();
const git = (...args) =>
  execFileSync(
    "git",
    ["-c", `safe.directory=${checkout}`, "-C", checkout, ...args],
    { encoding: "utf8" },
  );
git(
  "diff",
  "--exit-code",
  revision,
  "--",
  "crates/vitrine-tlog/src",
  "crates/vitrine-wasm/src",
  "packages/verify-wasm/src",
);
const fixture = JSON.parse(
  git("show", `${revision}:testdata/receipt-chain/waict-normalization.json`),
);
assert.equal(fixture.vectors.length, 10);
const pureFile = require.resolve("@vitrine/verify/pure");
const sdk = await import(pathToFileURL(pureFile));
const check = (v, result) => {
  const tiers = Object.fromEntries(result.tiers.map((t) => [t.tier, t.status]));
  assert.deepEqual(
    tiers,
    Object.fromEntries(
      ["record", "witnesses", "hubs", "browser"].map((t) => [t, v.expected[t]]),
    ),
    v.name,
  );
};
for (const v of fixture.vectors) {
  check(
    v,
    sdk.verifyChain(v.chain, sdk.pinPolicy(v.policy), {
      expect: { entry: Uint8Array.from(Buffer.from(v.entry, "base64")) },
      now: v.now,
    }),
  );
}
const moduleFile = path.join(
  checkout,
  "packages/verify-wasm/dist/wasm/vitrine_wasm_bg.wasm",
);
const moduleBytes = await fs.readFile(moduleFile);
const wasmSource = path.join(checkout, "packages/verify-wasm/src/index.ts");
const bundled = await build({
  stdin: {
    contents: `import { pinPolicy, verifyChain } from "@vitrine/verify/pure";
      import { loadWasmBackend } from ${JSON.stringify(wasmSource)};
      window.runConformance = async vectors => {
        const compiled = await loadWasmBackend({wasm:new URL('/core.wasm',location.href)});
        return vectors.map(v => {
          const policy=pinPolicy(v.policy);
          const options={expect:{entry:Uint8Array.from(atob(v.entry),c=>c.charCodeAt(0))},now:v.now};
          const result = backend => { const r=backend(v.chain,policy,options);return {status:r.status,tiers:Object.fromEntries(r.tiers.map(t=>[t.tier,t.status]))}; };
          return {name:v.name,pure:result(verifyChain),compiled:result(compiled.verifyChain)};
        });
      };`,
    resolveDir: path.resolve(new URL("../packages", import.meta.url).pathname),
    sourcefile: "vitrine-conformance-entry.mjs",
  },
  bundle: true,
  format: "esm",
  platform: "browser",
  write: false,
  plugins: [
    {
      name: "installed-vitrine",
      setup(plugin) {
        plugin.onResolve(
          { filter: /^@vitrine\/verify(?:\/pure)?$/ },
          (args) => ({ path: require.resolve(args.path) }),
        );
      },
    },
  ],
});
const html =
  "<!doctype html><html lang=en><meta charset=utf-8><title>Vitrine verifier conformance</title><h1>WAICT receipt-chain conformance</h1><pre id=result>Checking compiled core and installed browser SDK…</pre><script type=module src=/bundle.js></script></html>";
const server = http.createServer((request, response) => {
  response.setHeader("Cache-Control", "no-store");
  if (request.url === "/") {
    response.setHeader("Content-Type", "text/html");
    response.end(html);
  } else if (request.url === "/bundle.js") {
    response.setHeader("Content-Type", "text/javascript");
    response.end(bundled.outputFiles[0].contents);
  } else if (request.url === "/core.wasm") {
    response.setHeader("Content-Type", "application/wasm");
    response.end(moduleBytes);
  } else {
    response.statusCode = 404;
    response.end("not found");
  }
});
await fs.mkdir(output);
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
let browser;
try {
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({
    viewport: { width: 1280, height: 900 },
  });
  const errors = [];
  const requests = [];
  const origin = `http://127.0.0.1:${server.address().port}`;
  page.on("pageerror", (error) => errors.push(error.message));
  await page.route("**/*", async (route) => {
    const request = route.request();
    const headers = await request.allHeaders();
    assert.equal(
      new URL(request.url()).origin,
      origin,
      "no service or third-party request",
    );
    assert.ok(!headers.cookie && !headers.authorization, "no credentials");
    if (new URL(request.url()).pathname === "/core.wasm")
      assert.ok(!headers.referer, "no WASM referrer");
    requests.push(new URL(request.url()).pathname);
    await route.continue();
  });
  await page.goto(origin);
  await page.waitForFunction(() => typeof window.runConformance === "function");
  const results = await page.evaluate(
    (vectors) => window.runConformance(vectors),
    fixture.vectors,
  );
  for (const [i, result] of results.entries()) {
    for (const backend of ["pure", "compiled"]) {
      const v = fixture.vectors[i];
      assert.equal(
        result[backend].status,
        v.expected.browser === "verified" ? "verified" : "invalid",
        `${v.name} ${backend}`,
      );
      assert.deepEqual(
        result[backend].tiers,
        Object.fromEntries(
          ["record", "witnesses", "hubs", "browser"].map((t) => [
            t,
            v.expected[t],
          ]),
        ),
        `${v.name} ${backend}`,
      );
    }
    assert.deepEqual(result.pure, result.compiled, result.name);
  }
  assert.deepEqual(errors, []);
  assert.equal(
    requests.filter((request) => request === "/core.wasm").length,
    1,
  );
  await page.locator("#result").evaluate((element, results) => {
    element.textContent = results
      .map(
        (r) =>
          `${r.name}: pure=${r.pure.status}; compiled=${r.compiled.status}`,
      )
      .join("\n");
  }, results);
  await page.screenshot({
    path: path.join(output, "waict-conformance.png"),
    fullPage: true,
  });
  const report = {
    revision,
    source:
      "immutable simulated receipt-chain vectors; installed Step SDK and compiled native WASM",
    browser: await browser.version(),
    wasmSha256: createHash("sha256").update(moduleBytes).digest("hex"),
    nodeCases: fixture.vectors.length,
    browserCases: results.length,
    requests,
    pageErrors: errors,
    results,
  };
  await fs.writeFile(
    path.join(output, "results.json"),
    `${JSON.stringify(report, null, 2)}\n`,
  );
  console.log(JSON.stringify(report, null, 2));
} finally {
  if (browser) await browser.close();
  await new Promise((resolve) => server.close(resolve));
}
