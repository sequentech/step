// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// Actual production WASM, with public data and no trustee identity or store.
import assert from "node:assert/strict";
import { before, after, test } from "node:test";
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { chromium } from "../../../node_modules/playwright/index.mjs";

const fixture = JSON.parse(
  await readFile(
    new URL("../data/public-transcript-v1.json", import.meta.url),
    "utf8",
  ),
);
let browser, page, server;
before(async () => {
  server = createServer(async (request, response) => {
    response.setHeader("Cross-Origin-Opener-Policy", "same-origin");
    response.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
    try {
      if (request.url === "/") {
        response.setHeader("Content-Type", "text/html");
        response.end(
          '<!doctype html><title>Public transcript verifier regression</title><script type="module">import init, {initThreadPool, WasmVerifier} from "/wbraid-trustee/index.js"; await init({module_or_path: "/wbraid-trustee/index_bg.wasm"}); await initThreadPool(2); window.verifier = WasmVerifier;</script>',
        );
      } else if (
        request.url.startsWith("/wbraid-trustee/") &&
        !request.url.includes("..")
      ) {
        response.setHeader(
          "Content-Type",
          request.url.endsWith(".wasm")
            ? "application/wasm"
            : "text/javascript",
        );
        response.end(
          await readFile(
            new URL(
              `../../../admin-portal/public${request.url}`,
              import.meta.url,
            ),
          ),
        );
      } else response.writeHead(404).end();
    } catch {
      response.writeHead(500).end();
    }
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  browser = await chromium.launch({
    headless: true,
    ...(process.env.CHROMIUM_EXECUTABLE_PATH
      ? { executablePath: process.env.CHROMIUM_EXECUTABLE_PATH }
      : {}),
  });
  page = await browser.newPage();
  await page.goto(`http://127.0.0.1:${server.address().port}/`);
  await page.waitForFunction(() => Boolean(window.verifier));
});
after(async () => {
  if (browser) await browser.close();
  if (server) await new Promise((resolve) => server.close(resolve));
});

async function verify(
  transcript = fixture.transcript,
  request = fixture.request,
) {
  return page.evaluate(
    ({ transcript, request }) =>
      JSON.parse(
        window.verifier.verifyTranscript(
          JSON.stringify(transcript),
          JSON.stringify(request),
        ),
      ),
    { transcript, request },
  );
}

test("public browser verifier reproduces the native report without private keys", async () => {
  assert.deepEqual(await verify(), fixture.report);
});
test("public browser verifier refuses another election or sibling tally", async () => {
  for (const request of [
    { ...fixture.request, configuration_hash: "00".repeat(64) },
    {
      ...fixture.request,
      tally_id: String(BigInt(fixture.request.tally_id) + 1n),
    },
  ]) {
    const report = await verify(fixture.transcript, request);
    assert.equal(report.status, "invalid");
    assert.deepEqual(report.plaintexts, []);
  }
});
test("public browser verifier keeps incomplete evidence distinct from success", async () => {
  const report = await verify({ ...fixture.transcript, tally_messages: [] });
  assert.equal(report.status, "incomplete");
  assert.deepEqual(report.plaintexts, []);
});
test("public browser verifier checks canonical bytes and trusted batch pins", async () => {
  const evidence = structuredClone(fixture.transcript);
  evidence.dkg_messages[0] += "AAAA";
  assert.equal((await verify(evidence)).status, "invalid");
  assert.equal(
    (
      await verify(fixture.transcript, {
        ...fixture.request,
        ballots_hash: "00".repeat(64),
      })
    ).status,
    "invalid",
  );
});
test("public browser verifier refuses ciphertexts absent from the verified batch", async () => {
  const first = Buffer.from(fixture.request.ballot_ciphertexts[0], "base64");
  const second = Buffer.from(fixture.request.ballot_ciphertexts[1], "base64");
  // Keep canonical points while changing one ciphertext point. The first
  // 23 bytes are the one-contest vector's length and its 'contest' string.
  second.copy(first, 23, 23, 55);
  const report = await verify(fixture.transcript, {
    ...fixture.request,
    ballot_ciphertexts: [first.toString("base64")],
  });
  assert.equal(report.status, "invalid");
  assert.match(report.detail, /absent/);
  assert.deepEqual(report.ballot_inclusions, []);
});
