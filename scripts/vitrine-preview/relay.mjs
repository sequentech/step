// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Runs in the container vitrine-ui-harness-relay, which shares the Vitrine
// stack's network namespace: it relays a Unix socket in a mounted directory
// to the log server's internal listener, which only that namespace reaches.
// Usage: node relay.mjs SOCKET HOST PORT

import fs from "node:fs";
import net from "node:net";

const [socket, host, port] = process.argv.slice(2);
fs.rmSync(socket, { force: true });
const server = net.createServer((client) => {
  const upstream = net.connect(Number(port), host);
  client.pipe(upstream);
  upstream.pipe(client);
  client.on("error", () => upstream.destroy());
  upstream.on("error", () => client.destroy());
});
server.listen(socket, () => {
  fs.chmodSync(socket, 0o600);
  console.log(`relaying ${socket} to ${host}:${port}`);
});
for (const signal of ["SIGTERM", "SIGINT"]) process.on(signal, () => server.close(() => process.exit(0)));
