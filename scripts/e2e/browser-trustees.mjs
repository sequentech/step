// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// Test-only controller: two real Chromium pages run the production WASM and
// browser service against a disposable backend. It never substitutes protocol
// messages. Start this only on the owned E2E compose network.
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { resolve, extname } from "node:path";
import { chromium } from "../../packages/node_modules/playwright/index.mjs";
import ts from "../../packages/node_modules/typescript/lib/typescript.js";

const root = resolve("packages/admin-portal/public");
const service = ts.transpileModule(
  await readFile(
    "packages/admin-portal/src/services/BrowserTrustee.ts",
    "utf8",
  ),
  {
    compilerOptions: {
      target: ts.ScriptTarget.ES2022,
      module: ts.ModuleKind.ES2022,
    },
  },
).outputText;
const browser = await chromium.launch({ headless: true });
const clients = new Map();
const failures = [];
async function initialize(page, input) {
  await page.waitForFunction(() => Boolean(window.module));
  await page.evaluate(async (input) => {
    const { name, keys, boardUrl, platform, token } = input;
    window.engine = new window.module.BrowserTrustee(name, keys, boardUrl);
    if (input.backup)
      window.boards = JSON.parse(
        await window.engine.restoreBackup(input.backup),
      );
    window.keys = keys;
    window.boardUrl = boardUrl;
    window.name = name;
    window.platform = platform;
    window.token = token;
    if (!input.backup) window.boards = input.boards || [];
    window.cycles = 0;
    window.paused = false;
    window.busy = false;
    window.run = async () => {
      if (window.paused || window.busy) return;
      window.busy = true;
      try {
        await window.runBrowserCycle(
          window.engine,
          platform,
          token,
          (boards) => {
            window.boards = Array.from(
              new Set([
                ...window.boards,
                ...boards.flatMap((b) =>
                  b.parent ? [b.name, b.parent] : [b.name],
                ),
              ]),
            );
          },
        );
        window.cycles++;
      } finally {
        window.busy = false;
      }
      if (!window.paused) window.timer = setTimeout(window.run, 500);
    };
    void window.run();
  }, input);
}
const server = createServer(async (request, response) => {
  const path = new URL(request.url, "http://localhost").pathname;
  try {
    if (request.method === "GET" && path === "/health") {
      response
        .writeHead(200, { "Content-Type": "application/json" })
        .end(JSON.stringify({ clients: [...clients.keys()], failures }));
      return;
    }
    if (request.method === "GET") {
      response.setHeader("Cross-Origin-Opener-Policy", "same-origin");
      response.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
      if (path === "/") {
        response.setHeader("Content-Type", "text/html");
        response.end(
          '<!doctype html><title>Native browser trustees integration</title><script type="module">import {loadBrowserTrustee,runBrowserCycle} from "/BrowserTrustee.js"; window.module = await loadBrowserTrustee(); window.runBrowserCycle = runBrowserCycle;</script>',
        );
      } else if (path === "/BrowserTrustee.js") {
        response.setHeader("Content-Type", "text/javascript");
        response.end(service);
      } else if (path.startsWith("/wbraid-trustee/") && !path.includes("..")) {
        response.setHeader(
          "Content-Type",
          extname(path) === ".wasm" ? "application/wasm" : "text/javascript",
        );
        response.end(await readFile(resolve(root, `.${path}`)));
      } else response.writeHead(404).end();
      return;
    }
    if (request.method !== "POST") {
      response.writeHead(405).end();
      return;
    }
    let body = "";
    for await (const chunk of request) {
      body += chunk;
      if (body.length > 8_000_000) throw new Error("test request too large");
    }
    const input = JSON.parse(body);
    let result = {};
    if (path === "/start") {
      if (clients.has(input.name)) throw new Error("duplicate trustee");
      const context = await browser.newContext();
      const page = await context.newPage();
      page.on("pageerror", (error) => failures.push(error.message));
      await page.goto("http://localhost:9229/");
      await page.waitForFunction(() => Boolean(window.module));
      await initialize(page, input);
      clients.set(input.name, { page, context });
    } else if (path === "/verify") {
      // A separate public observer page has no trustee identity or keys.
      const context = await browser.newContext();
      const page = await context.newPage();
      try {
        await page.goto("http://localhost:9229/");
        await page.waitForFunction(() => Boolean(window.module));
        result = await page.evaluate(async (input) => {
          const verifier = window.module.WasmVerifier;
          const anchors = {
            ...input.anchors,
            configuration_hash: verifier.configurationHash(
              input.configurationMessage,
            ),
            ballots_hash: input.ballotsMessage
              ? verifier.ballotsHash(input.ballotsMessage)
              : null,
          };
          const transcript = await verifier.fetchTranscript(
            input.boardUrl,
            input.dkgBoard,
            input.tallyBoard,
          );
          const report = JSON.parse(
            verifier.verifyTranscript(transcript, JSON.stringify(anchors)),
          );
          const wrongAnchor = JSON.parse(
            verifier.verifyTranscript(
              transcript,
              JSON.stringify({
                ...anchors,
                configuration_hash: "00".repeat(64),
              }),
            ),
          );
          const incomplete = JSON.parse(transcript);
          if (incomplete.tally_messages) incomplete.tally_messages = [];
          else incomplete.dkg_messages = [];
          const truncated = JSON.parse(
            verifier.verifyTranscript(
              JSON.stringify(incomplete),
              JSON.stringify(anchors),
            ),
          );
          return {
            transcript: JSON.parse(transcript),
            anchors,
            report,
            wrongAnchor,
            truncated,
          };
        }, input);
      } finally {
        await context.close();
      }
    } else {
      const client = clients.get(input.name);
      if (!client) throw new Error("trustee not started");
      const page = client.page;
      await page.evaluate(() => {
        window.paused = true;
        clearTimeout(window.timer);
      });
      await page.waitForFunction(() => !window.busy);
      try {
        if (path === "/attest") {
          result = {
            proof: await page.evaluate(
              (request) => window.engine.attest(request),
              input.request,
            ),
          };
        } else if (path === "/restart") {
          const saved = await page.evaluate(async () => {
            const backup = await window.engine.backup(
              JSON.stringify(window.boards),
            );
            const stores = JSON.parse(backup).stores;
            if (!stores.some((s) => s.predicates.length && s.own_posts.length))
              throw new Error(
                "no persisted DKG commitments and mailbox records",
              );
            return {
              name: window.name,
              keys: window.keys,
              boardUrl: window.boardUrl,
              platform: window.platform,
              token: window.token,
              boards: window.boards,
              stores: stores.length,
              cycles: window.cycles,
            };
          });
          await page.reload();
          await initialize(page, saved);
          result = { stores: saved.stores, cycles: saved.cycles };
        } else if (path === "/restore-in-new-browser") {
          const saved = await page.evaluate(async () => ({
            backup: await window.engine.backup(JSON.stringify(window.boards)),
            name: window.name,
            keys: window.keys,
            boardUrl: window.boardUrl,
            platform: window.platform,
            token: window.token,
          }));
          const context = await browser.newContext(),
            replacement = await context.newPage();
          replacement.on("pageerror", (error) => failures.push(error.message));
          await replacement.goto("http://localhost:9229/");
          await initialize(replacement, saved);
          clients.set(input.name, { page: replacement, context });
          await client.context.close();
          result = { restored: true };
          response
            .writeHead(200, { "Content-Type": "application/json" })
            .end(JSON.stringify(result));
          return;
        } else if (path === "/status") {
          result = await page.evaluate(() => ({
            cycles: window.cycles,
            boards: window.boards,
          }));
        } else throw new Error("unknown operation");
      } finally {
        if (!page.isClosed())
          await page.evaluate(() => {
            window.paused = false;
            void window.run();
          });
      }
    }
    response
      .writeHead(200, { "Content-Type": "application/json" })
      .end(JSON.stringify(result));
  } catch (error) {
    response
      .writeHead(500, { "Content-Type": "application/json" })
      .end(JSON.stringify({ error: error.message }));
  }
});
server.listen(9229, "0.0.0.0");
const stop = async () => {
  server.close();
  await browser.close();
  process.exit(0);
};
process.on("SIGTERM", stop);
process.on("SIGINT", stop);
