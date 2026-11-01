// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "node:assert/strict";
import test from "node:test";
import { HIDE_PROBLEMS_SCRIPT, createDishonestPortal, injectHideProblems } from "./dishonest-portal.mjs";

test("the script goes right after <head>, before the app's scripts", () => {
  const page = '<!doctype html><html><head lang="en"><script src="/app.js"></script></head></html>';
  const out = injectHideProblems(page);
  assert.ok(out.startsWith('<!doctype html><html><head lang="en">' + HIDE_PROBLEMS_SCRIPT));
  assert.ok(out.indexOf(HIDE_PROBLEMS_SCRIPT) < out.indexOf("/app.js"));
});

test("a page without <head> gets the script first", () => {
  assert.equal(injectHideProblems("<p>x</p>"), `${HIDE_PROBLEMS_SCRIPT}<p>x</p>`);
});

test("the switch is off until it is turned on", async () => {
  const d = createDishonestPortal({ log: () => {} });
  assert.equal(d.isOn(), false);
  const sent = [];
  const send = (_res, code, body) => sent.push([code, body]);
  await d.handle({ method: "POST" }, null, null, async () => '{"on":true}', send);
  assert.equal(d.isOn(), true);
  await d.handle({ method: "POST" }, null, null, async () => '{"on":false}', send);
  assert.equal(d.isOn(), false);
  assert.deepEqual(sent.map(([c]) => c), [200, 200]);
});
