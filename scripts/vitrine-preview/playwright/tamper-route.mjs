// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The Alert scenario in a Playwright browser. While the preview's switch is on
// (`preview.mjs tamper on`), the voter's browser is shown the checkpoint a
// dishonest operator signed with the ballots log's own key (a record of the
// same size as the voter's receipt without the voter's ballot) instead of
// the log's real one. Everything else goes to the real stack, and the
// real log, witnesses, monitors and explorer never serve the forged view.
//
//   import { installTamperRoute } from "./tamper-route.mjs";
//   await installTamperRoute(context);   // before navigating
//   // ... later: `node preview.mjs tamper on`, or setTamper(true) below
//
// The ballots log's public URL comes from the stack's policy; override it
// with {log: "http://localhost:18710/demo/ballots/"} if the stack differs.

const STANDIN = "http://127.0.0.1:18851";

export async function setTamper(on, standin = STANDIN) {
  const res = await fetch(`${standin}/__preview/tamper`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ on }) });
  return res.json();
}

export async function installTamperRoute(context, { log = "http://localhost:18710/demo/ballots/", standin = STANDIN } = {}) {
  await context.route(`${log}checkpoint`, async (route) => {
    const answer = await fetch(`${standin}/__preview/tamper/checkpoint`).catch(() => undefined);
    if (!answer || answer.status !== 200) return route.continue();
    await route.fulfill({
      status: 200,
      contentType: "text/plain; charset=utf-8",
      headers: { "access-control-allow-origin": "*", "cross-origin-resource-policy": "cross-origin" },
      body: await answer.text(),
    });
  });
}
