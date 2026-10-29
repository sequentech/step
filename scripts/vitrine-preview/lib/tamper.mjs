// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The Alert scenario's switch (filled in below).

export function createTamper({ log }) {
  let on = false;
  const casts = [];
  return {
    remember: (cast) => casts.push(cast),
    status: () => ({ on, casts: casts.length }),
    async handle(req, res, url, readBody, send) {
      if (req.method === "POST") on = JSON.parse(await readBody(req)).on === true;
      log(`tamper: ${on ? "on" : "off"}`);
      send(res, 200, { on, casts: casts.length });
    },
  };
}
