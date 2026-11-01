// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The staged attack's second half: an operator that also controls the voting
// portal. With the switch on, the voting portal's origin serves its pages with
// one more script in front of the app's own, which answers every hub's
// `GET <hub>/problems` with an empty list. The voter's own page then goes on
// saying the ballot is verified while a hub (deploy/split-view.sh in Vitrine)
// records the split view; the voter learns of it from the hub's email, the
// hub's site, a monitor or the browser extension, which the operator doesn't
// serve. Previews only: Step's code is not changed, only the page this
// harness serves.

/** The script put in front of the portal's own. */
export const HIDE_PROBLEMS_SCRIPT = `<script>(() => {
  const real = window.fetch.bind(window);
  window.fetch = (input, init) => {
    const href = typeof input === "string" ? input : input instanceof URL ? input.href : input.url;
    let url;
    try { url = new URL(href, location.href); } catch { return real(input, init); }
    if (url.origin !== location.origin && /\\/problems$/.test(url.pathname)) {
      const body = JSON.stringify({ hub: url.host, problems: [], more: false });
      return Promise.resolve(new Response(body, { status: 200, headers: { "Content-Type": "application/json" } }));
    }
    return real(input, init);
  };
})();</script>`;

/** Puts the script right after `<head>` (or first, if the page has none). */
export function injectHideProblems(html) {
  const at = html.search(/<head[^>]*>/i);
  if (at < 0) return `${HIDE_PROBLEMS_SCRIPT}${html}`;
  const end = html.indexOf(">", at) + 1;
  return `${html.slice(0, end)}${HIDE_PROBLEMS_SCRIPT}${html.slice(end)}`;
}

export function createDishonestPortal({ log }) {
  let on = false;
  const status = () => ({ on, what: on ? "the voting portal hides every hub's problems" : "off" });
  return {
    isOn: () => on,
    status,
    async handle(req, res, url, readBody, send) {
      if (req.method === "POST") {
        const body = JSON.parse((await readBody(req)) || "{}");
        on = body.on === true;
        log(`dishonest portal ${on ? "on" : "off"}`);
      }
      return send(res, 200, status());
    },
  };
}
