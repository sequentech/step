// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import { createSplitView, isLocal } from "./split-view.mjs";

const req = (peer, host, method = "POST") => ({ method, socket: { remoteAddress: peer }, headers: { host } });
const collect = () => {
  const sent = [];
  return { sent, send: (_res, code, body) => sent.push([code, body]) };
};

test("only a loopback peer asking a loopback host is local", () => {
  assert.equal(isLocal(req("127.0.0.1", "127.0.0.1:18851")), true);
  assert.equal(isLocal(req("::ffff:127.0.0.1", "localhost:18851")), true);
  assert.equal(isLocal(req("10.0.0.5", "127.0.0.1:18851")), false);
  assert.equal(isLocal(req("127.0.0.1", "evil.example:18851")), false);
});

test("it refuses other machines and other methods, and runs the script with the receipt", async () => {
  const calls = [];
  const runner = async (cmd, args) => {
    calls.push([cmd, args, fs.readFileSync(args[1], "utf8")]);
    return { code: 0, output: "409 evidence sha256:abc\n" };
  };
  const sv = createSplitView({
    vitrineDir: "/v",
    latest: () => ({ entry: Buffer.from("entry") }),
    fetchReceipt: async () => "c2sp.org/tlog-proof@v1\n",
    runner,
    log: () => {},
  });
  const out = collect();
  await sv.handle(req("10.0.0.5", "127.0.0.1"), null, null, async () => "{}", out.send);
  await sv.handle(req("127.0.0.1", "127.0.0.1", "GET"), null, null, async () => "{}", out.send);
  assert.deepEqual(out.sent.map(([c]) => c), [403, 405]);
  assert.equal(calls.length, 0);
  await sv.handle(req("127.0.0.1", "127.0.0.1"), null, null, async () => "{}", out.send);
  assert.equal(out.sent[2][0], 200);
  assert.equal(out.sent[2][1].output, "409 evidence sha256:abc\n");
  assert.equal(calls[0][0], "/v/deploy/split-view.sh");
  assert.equal(calls[0][1][0], "--receipt");
  assert.equal(calls[0][2], "c2sp.org/tlog-proof@v1\n");
});

test("with nothing cast it answers 409 and runs nothing", async () => {
  const sv = createSplitView({ vitrineDir: "/v", latest: () => undefined, fetchReceipt: async () => "", runner: async () => assert.fail("ran"), log: () => {} });
  const out = collect();
  await sv.handle(req("127.0.0.1", "localhost"), null, null, async () => "{}", out.send);
  assert.equal(out.sent[0][0], 409);
});
