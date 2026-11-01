// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The staged attack's other half, for recordings: `POST /__preview/split-view`
// runs Vitrine's `deploy/split-view.sh` on this machine with the receipt of the
// voter's latest cast (the one the stand-in's ballot box wrote), so a recording
// can trigger the split view from its board with one request. The script signs
// a second checkpoint of the receipt's size, without the ballot, with the dev
// log key, and gossips it to hub-b, which proves the split view itself.
//
// Development only: it answers requests from this machine only (a loopback
// peer and a loopback Host), like the script, which refuses any other URL.
// Nothing here is Step.

import { spawn } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { leafHash } from "./vitrine.mjs";

const TIMEOUT_MS = 150_000;
const LOOPBACK_PEER = new Set(["127.0.0.1", "::1", "::ffff:127.0.0.1"]);
const LOOPBACK_HOST = /^(localhost|127\.0\.0\.1|\[::1\])(:\d+)?$/;

/** Whether a request came from this machine, to this machine's own address. */
export const isLocal = (req) => LOOPBACK_PEER.has(req.socket?.remoteAddress ?? "") && LOOPBACK_HOST.test(req.headers?.host ?? "");

/** Runs a command, answering its exit code and its output (stdout and stderr, in order). */
export function run(command, args, { timeoutMs = TIMEOUT_MS } = {}) {
  return new Promise((resolve) => {
    const child = spawn(command, args, { stdio: ["ignore", "pipe", "pipe"] });
    let output = "";
    const take = (chunk) => {
      output += chunk;
    };
    child.stdout.on("data", take);
    child.stderr.on("data", take);
    const timer = setTimeout(() => child.kill("SIGTERM"), timeoutMs);
    child.on("error", (e) => {
      clearTimeout(timer);
      resolve({ code: -1, output: `${output}${e.message}` });
    });
    child.on("close", (code) => {
      clearTimeout(timer);
      resolve({ code: code ?? -1, output });
    });
  });
}

/** `latest()` answers the voter's latest cast (`{ entry }`, as the tamper switch
 * remembers it); `fetchReceipt(leafHex)` the log's receipt for a leaf. */
export function createSplitView({ vitrineDir, latest, fetchReceipt, runner = run, log }) {
  const script = path.join(vitrineDir, "deploy", "split-view.sh");
  let running;

  async function stage(extra = []) {
    const cast = latest();
    if (!cast) return { status: 409, body: { ok: false, error: "nothing cast yet: the voter's receipt is what the split view starts from" } };
    const receipt = await fetchReceipt(leafHash(cast.entry).toString("hex"));
    if (!receipt) return { status: 503, body: { ok: false, error: "the log has no receipt for the cast yet" } };
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "vitrine-split-view-"));
    const file = path.join(dir, "receipt");
    fs.writeFileSync(file, receipt);
    try {
      log(`split view: running ${path.basename(script)} --receipt`);
      const { code, output } = await runner(script, ["--receipt", file, ...extra]);
      log(`split view: the script ended with ${code}`);
      return { status: code === 0 ? 200 : 502, body: { ok: code === 0, code, output } };
    } finally {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  }

  return {
    stage,
    async handle(req, res, url, readBody, send) {
      if (!isLocal(req)) return send(res, 403, { ok: false, error: "the split view can be staged from this machine only" });
      if (req.method !== "POST") return send(res, 405, { ok: false, error: "POST to stage the split view" });
      const body = JSON.parse((await readBody(req)) || "{}");
      if (running) return send(res, 409, { ok: false, error: "a split view is being staged already" });
      running = stage(body.dryRun === true ? ["--dry-run"] : []);
      try {
        const { status, body: answer } = await running;
        return send(res, status, answer);
      } finally {
        running = undefined;
      }
    },
  };
}
