// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The Alert scenario: a dishonest operator that signs a second version of the
// record. With the ballots log's own key (read from the stack's deploy/.state)
// it signs a checkpoint of the same size as the one in the voter's receipt but
// with a different root, as if the voter's ballot were not in the tree. The
// witnesses never signed it, and the real log, monitors and explorer never
// serve it. Only the voter's browser is shown it, through the
// route that playwright/tamper-route.mjs installs, so the portal's own
// receipt checks hold two log-signed checkpoints of one size with different
// roots: split-view evidence, with nothing faked in the portal.

import fs from "node:fs";
import path from "node:path";
import { readEntries } from "./events.mjs";
import { leafHash, noteSigner, sha256 } from "./vitrine.mjs";

const NODE_PREFIX = Buffer.of(1);

/** RFC 6962's Merkle tree hash of leaf hashes. */
export function merkleRoot(leaves) {
  if (leaves.length === 0) return sha256();
  if (leaves.length === 1) return leaves[0];
  let k = 1;
  while (k * 2 < leaves.length) k *= 2;
  return sha256(NODE_PREFIX, merkleRoot(leaves.slice(0, k)), merkleRoot(leaves.slice(k)));
}

const lineOf = (text, key) => text.split("\n").find((l) => l.startsWith(`${key} `))?.slice(key.length + 1);

export function createTamper({ live, log }) {
  const signer = noteSigner(fs.readFileSync(path.join(live.stateDir, "log", "ballots.key"), "utf8"));
  let on = false;
  let latest;
  const forged = new Map();

  /** The forged checkpoint for a cast: the receipt's size, a root without the voter's ballot. */
  async function forge(cast) {
    const leaf = leafHash(cast.entry).toString("hex");
    const base = live.ballots.url.endsWith("/") ? live.ballots.url : `${live.ballots.url}/`;
    const res = await fetch(`${base}receipt/${leaf}`, { signal: AbortSignal.timeout(8000) });
    if (res.status !== 200) throw new Error(`the log has no receipt for the cast yet (${res.status})`);
    const receipt = await res.text();
    // A tlog-proof: the entry, its index and proof, a blank line, then the checkpoint note.
    const checkpoint = receipt.slice(receipt.indexOf("\n\n") + 2);
    const size = Number(checkpoint.split("\n")[1]);
    if (!Number.isInteger(size) || size < 1) throw new Error("the receipt has no checkpoint");
    const index = Number(lineOf(receipt, "index"));
    const key = `${size}/${leaf}`;
    if (forged.has(key)) return forged.get(key);
    const { entries } = await readEntries(live.ballots.url);
    if (entries.length < size) throw new Error("the log's tiles don't cover the receipt's checkpoint");
    const leaves = entries.slice(0, size).map(leafHash);
    if (!Number.isInteger(index) || index >= size) throw new Error("the receipt has no usable index");
    leaves[index] = leafHash(Buffer.from("a different entry, not the voter's ballot"));
    const text = `${live.ballots.origin}\n${size}\n${merkleRoot(leaves).toString("base64")}\n`;
    const note = signer.sign(text);
    log(`tamper: signed a checkpoint of size ${size} without the ballot at index ${index}`);
    forged.set(key, { size, index, note });
    return forged.get(key);
  }

  return {
    remember: (cast) => {
      latest = cast;
    },
    status: () => ({ on, casts: latest ? 1 : 0, forged: [...forged.values()].map(({ size, index }) => ({ size, index })) }),
    async handle(req, res, url, readBody, send) {
      if (req.method === "POST") {
        on = JSON.parse(await readBody(req)).on === true;
        log(`tamper: ${on ? "on" : "off"}`);
        return send(res, 200, this.status());
      }
      if (url.pathname.endsWith("/checkpoint")) {
        if (!on) return send(res, 404, { error: "the switch is off" });
        if (!latest) return send(res, 404, { error: "nothing cast yet" });
        try {
          const { note } = await forge(latest);
          res.setHeader("Access-Control-Allow-Origin", "*");
          return send(res, 200, note, "text/plain; charset=utf-8");
        } catch (e) {
          log(`tamper: ${e.stack}`);
          return send(res, 503, { error: e.message });
        }
      }
      return send(res, 200, this.status());
    },
  };
}
