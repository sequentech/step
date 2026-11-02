// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The Vitrine log server, as Step's backend reaches it: its internal
// listener (`add`, admin routes) through the relay container's Unix socket,
// since only the stack's own network namespace reaches it, and its public
// routes over plain HTTP.

import http from "node:http";

const MAX_ANSWER = 1 << 20;

function request({ socketPath, url, method = "GET", headers = {}, body, timeoutMs = 10000 }) {
  return new Promise((resolve, reject) => {
    const target = new URL(url);
    const req = http.request(
      {
        socketPath,
        host: socketPath ? undefined : target.hostname,
        port: socketPath ? undefined : target.port,
        path: target.pathname + target.search,
        method,
        headers: { ...headers, ...(body ? { "Content-Length": body.length } : {}) },
        timeout: timeoutMs,
      },
      (res) => {
        const chunks = [];
        let size = 0;
        res.on("data", (c) => {
          size += c.length;
          if (size > MAX_ANSWER) req.destroy(new Error("answer too large"));
          else chunks.push(c);
        });
        res.on("end", () => resolve({ status: res.statusCode, body: Buffer.concat(chunks).toString("utf8") }));
        res.on("error", reject);
      },
    );
    req.on("timeout", () => req.destroy(Object.assign(new Error("timeout"), { code: "ETIMEDOUT" })));
    req.on("error", reject);
    if (body) req.write(body);
    req.end();
  });
}

export class Sidecar {
  constructor(live, socketPath) {
    this.live = live;
    this.socketPath = socketPath;
  }

  internal(path, opts = {}) {
    return request({ ...opts, socketPath: this.socketPath, url: `http://internal/${path}` });
  }

  /** `POST <log>/add?timeout=<ms>` with the log's submission token, as the
   * demo booth's server submits (the stack's logs take HTTP submissions). */
  add(log, entry, timeoutMs, budgetMs) {
    const head = Buffer.from(`entry ${entry.length}\n`, "ascii");
    return this.internal(`${log.sidecar}/add?timeout=${Math.floor(timeoutMs)}`, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${log.token}`,
        "Content-Type": "application/vnd.vitrine.submission",
      },
      body: Buffer.concat([head, entry, Buffer.from("\n")]),
      timeoutMs: budgetMs,
    });
  }

  /** The admin listing of the logs (`GET /admin/logs`). */
  async adminLogs() {
    const r = await this.internal("admin/logs", { headers: { Authorization: `Bearer ${this.live.adminToken}` } });
    if (r.status !== 200) throw new Error(`GET /admin/logs answered ${r.status}`);
    const parsed = JSON.parse(r.body);
    return Array.isArray(parsed) ? parsed : (parsed.logs ?? []);
  }

  /** A public route of a log, by the policy's URL for it. */
  publicGet(log, route, timeoutMs = 5000) {
    return request({ url: new URL(route, log.url).toString(), timeoutMs });
  }

  async status(log) {
    const r = await this.publicGet(log, "status");
    if (r.status !== 200) throw new Error(`${log.name}/status answered ${r.status}`);
    return JSON.parse(r.body);
  }

  /** The receipt the log serves for a leaf, or undefined while it has none. */
  async receipt(log, leafHex) {
    const r = await this.publicGet(log, `receipt/${leafHex}`);
    return r.status === 200 ? r.body : undefined;
  }
}
