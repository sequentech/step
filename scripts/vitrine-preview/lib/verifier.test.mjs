// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The hub's ballot verifier:  node --test scripts/vitrine-preview/lib/
import assert from "node:assert/strict";
import test from "node:test";
import { auditedLink, policyBallotVerifier, verifierInPolicy, verifierSettings, VERIFIER_HUB } from "./verifier.mjs";

const POLICY = [
  "vitrine.localhost/policy",
  "hub-ballot-verifier hub-b http://127.0.0.1:9999/",
  "hub hub-a hub-a.example+1+A http://localhost:18720/",
  "hub-explorer hub-a http://localhost:18706/",
  "hub hub-b hub-b.example+2+B http://localhost:18721/",
  "hub-explorer hub-b http://localhost:18704/",
  "hub-ballot-verifier hub-b http://127.0.0.1:18856/",
  "hub-ballot-verifier hub-b http://127.0.0.1:18999/",
  "",
  "— vitrine.localhost/policy abc=",
].join("\n");

test("the policy's ballot verifier of a hub is its first line after the hub's", () => {
  assert.equal(policyBallotVerifier(POLICY, "hub-b"), "http://127.0.0.1:18856/");
  assert.equal(policyBallotVerifier(POLICY, "hub-a"), undefined);
  assert.equal(policyBallotVerifier(undefined, "hub-b"), undefined);
});

test("the status says whether the policy names the preview's verifier", () => {
  assert.deepEqual(verifierInPolicy(POLICY, "http://127.0.0.1:18856"), { ok: true, url: "http://127.0.0.1:18856/" });
  const other = verifierInPolicy(POLICY, "http://127.0.0.1:18857");
  assert.equal(other.ok, false);
  assert.match(other.hint, /names http:\/\/127\.0\.0\.1:18856\/ for hub-b/);
  assert.match(other.hint, /VITRINE_HUB_B_BALLOT_VERIFIER=http:\/\/127\.0\.0\.1:18857\//);
  assert.match(verifierInPolicy("x\nhub hub-b k u", "http://127.0.0.1:18856").hint, /names no ballot verifier for hub-b/);
});

test("the audited ballot link carries the IDs in its path and fragment", () => {
  const ids = { tenantId: "t-1", eventId: "demo 2026", electionId: "council", trackingId: "FTBE-MHRX", policyId: "sha256:ab" };
  assert.equal(auditedLink("http://127.0.0.1:18856", ids), "http://127.0.0.1:18856/tenant/t-1/event/demo%202026/election/council/audited#id=FTBE-MHRX&policy=sha256:ab");
  assert.equal(auditedLink("http://127.0.0.1:18856/", ids), auditedLink("http://127.0.0.1:18856", ids));
});

test("the verifier is hub-b's, public, and reaches the stand-in through its own origin", () => {
  const origins = { voting: "http://127.0.0.1:18850", standin: "http://127.0.0.1:18851", verifier: "http://127.0.0.1:18856" };
  const settings = verifierSettings({ origins, tenantId: "t-1", eventId: "e-1", release: { version: "dev" } });
  assert.equal(settings.HUB_NAME, VERIFIER_HUB);
  assert.equal(settings.DISABLE_AUTH, true);
  assert.equal(settings.HASURA_URL, "http://127.0.0.1:18856/v1/graphql");
  assert.equal(settings.PUBLIC_BUCKET_URL, "http://127.0.0.1:18851/public/");
  assert.equal(settings.VOTING_PORTAL_URL, "http://127.0.0.1:18850");
  assert.equal(settings.DEFAULT_EVENT_ID, "e-1");
});
