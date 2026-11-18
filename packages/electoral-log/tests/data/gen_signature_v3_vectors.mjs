// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
// Run: node gen_signature_v3_vectors.mjs > signature_v3_vectors.json
// Independent generator of the log signature scheme v3 vectors (Node's
// Ed25519, OpenSSL). Synthetic seeds only.
import crypto from "node:crypto";
const seedHex = "07".repeat(32);
const key = crypto.createPrivateKey({
  key: Buffer.concat([Buffer.from("302e020100300506032b657004220420", "hex"), Buffer.from(seedHex, "hex")]),
  format: "der", type: "pkcs8",
});
const pub = crypto.createPublicKey(key).export({ format: "der", type: "spki" }).subarray(12);
const signed = (domain, canonical) => Buffer.concat([Buffer.from(domain, "ascii"), Buffer.from([0]), crypto.createHash("sha256").update(canonical).digest()]);
const vector = (name, domain, canonical) => {
  const message = signed(domain, canonical);
  return { name, domain, canonical_hex: canonical.toString("hex"), signed_message_hex: message.toString("hex"), signature_b64: crypto.sign(null, message, key).toString("base64") };
};
const checkpoint = Buffer.from(JSON.stringify(["sequent-electoral-log-checkpoint-v3", "board", "00000000-0000-0000-0000-000000000003", 9, "05".repeat(32), "TALLY_COMPLETED"]));
const out = {
  about: "Electoral log signature scheme v3: Ed25519 (pure) over domain || 0x00 || SHA-256(canonical bytes). Synthetic key; generated independently with Node's Ed25519 (tests/data/gen_signature_v3_vectors.mjs).",
  signing_key_seed_hex: seedHex,
  public_key_b64: Buffer.from(pub).toString("base64"),
  vectors: [
    vector("statement", "sequent-electoral-log/statement-signature/v3", Buffer.from("statement record bytes")),
    vector("statement-empty", "sequent-electoral-log/statement-signature/v3", Buffer.alloc(0)),
    // A real log statement's record bytes (electoral-log's TallyOpen for
    // "election-1" in "event-1" at 1700000000), as the Rust test pins them.
    vector("statement-real", "sequent-electoral-log/statement-signature/v3", Buffer.from("070000006576656e742d310d00f153650000000001001600000054616c6c792073657373696f6e206f70656e6e65642e0c010a000000656c656374696f6e2d31", "hex")),
    vector("checkpoint", "sequent-electoral-log/checkpoint-signature/v3", checkpoint),
  ],
};
process.stdout.write(JSON.stringify(out, null, 2) + "\n");
