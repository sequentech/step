// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// Test-only controller: two real Chromium pages run the production WASM and
// browser service against a disposable backend. It never substitutes protocol
// messages. Start this only on the owned E2E compose network.
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { resolve, extname } from "node:path";
import { chromium } from "../../packages/node_modules/playwright/index.mjs";
import { verifyPublicTranscript } from "./public-transcripts.mjs";
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
const authentication = new Map();
const expiresAt = (token) =>
  JSON.parse(Buffer.from(token.split(".")[1], "base64url").toString()).exp *
  1000;
async function renewAuthentication(name) {
  const auth = authentication.get(name);
  if (!auth || auth.expiresAt > Date.now() + 60000) return;
  if (auth.pending) return auth.pending;
  auth.pending = (async () => {
    const response = await fetch(auth.endpoint, {
      method: "POST",
      body: new URLSearchParams({
        grant_type: "refresh_token",
        client_id: "api-key-client",
        client_secret: auth.clientSecret,
        refresh_token: auth.refreshToken,
      }),
    });
    if (!response.ok)
      throw new Error(`Fixture token renewal failed: HTTP ${response.status}`);
    const tokens = await response.json();
    if (!tokens.access_token)
      throw new Error("Fixture token renewal returned no access token");
    auth.refreshToken = tokens.refresh_token || auth.refreshToken;
    auth.expiresAt = expiresAt(tokens.access_token);
    const client = clients.get(name);
    if (client && !client.page.isClosed()) {
      await client.page.evaluate((token) => {
        window.token = token;
      }, tokens.access_token);
    }
  })();
  try {
    await auth.pending;
  } finally {
    auth.pending = null;
  }
}
const authenticationTimer = setInterval(() => {
  for (const name of authentication.keys()) {
    renewAuthentication(name).catch((error) => failures.push(error.message));
  }
}, 15000);
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
          window.token,
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
      await context.route(/^https?:\/\/harvest(?::\d+)?\//, (route) => {
        failures.push("Browser attempted to access internal Harvest");
        return route.abort();
      });
      const page = await context.newPage();
      page.on("pageerror", (error) => failures.push(error.message));
      await page.goto("http://localhost:9229/");
      await page.waitForFunction(() => Boolean(window.module));
      // Confidential synthetic fixture credentials remain in the Node relay.
      const { auth, ...browserInput } = input;
      await initialize(page, browserInput);
      clients.set(input.name, { page, context });
      if (auth)
        authentication.set(input.name, {
          ...auth,
          expiresAt: expiresAt(input.token),
          pending: null,
        });
    } else if (path === "/verify") {
      result = await verifyPublicTranscript(
        browser,
        "http://localhost:9229",
        input,
      );
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
  clearInterval(authenticationTimer);
  server.close();
  await browser.close();
  process.exit(0);
};
process.on("SIGTERM", stop);
process.on("SIGINT", stop);
