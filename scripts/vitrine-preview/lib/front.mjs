// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// One portal's origin. It serves the preview's own /global-settings.json (the
// checkout's public/ stays untouched) and either proxies everything else to
// the portal's webpack dev server (so edits in the Step checkout show up
// live, hot reload included) or, in static mode, serves a production build
// from a dist folder with the release bundle the logged-sites release script
// wrote at /.well-known/vitrine/release/<package>/<version>. Both modes send
// the headers the dev server sends: the portals' multi-threaded WASM needs
// cross-origin isolation (COOP same-origin, COEP require-corp).

import fs from "node:fs";
import http from "node:http";
import net from "node:net";
import path from "node:path";

export const ISOLATION_HEADERS = {
  "Cross-Origin-Opener-Policy": "same-origin",
  "Cross-Origin-Embedder-Policy": "require-corp",
  "Cross-Origin-Resource-Policy": "cross-origin",
};

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".map": "application/json; charset=utf-8",
  ".wasm": "application/wasm",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".gif": "image/gif",
  ".ico": "image/x-icon",
  ".webp": "image/webp",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".ttf": "font/ttf",
  ".txt": "text/plain; charset=utf-8",
  ".xml": "application/xml",
  ".pdf": "application/pdf",
  ".mp3": "audio/mpeg",
};

export const WELL_KNOWN_RELEASE = "/.well-known/vitrine/release/";

/** The package and version a release bundle file names, if it is JSON that
 * says so; the caller's own values otherwise. */
export function releaseIdentity(file, fallback) {
  try {
    const parsed = JSON.parse(fs.readFileSync(file, "utf8"));
    const pick = (o) => (o && typeof o.package === "string" && typeof o.version === "string" ? { package: o.package, version: o.version } : undefined);
    return pick(parsed) ?? pick(parsed.release) ?? pick(parsed.bundle) ?? fallback;
  } catch {
    return fallback;
  }
}

export function createFront({ settings, upstreamPort, dist, release, log }) {
  const settingsText = `${JSON.stringify(settings, null, 2)}\n`;

  const headers = (extra = {}) => ({ ...ISOLATION_HEADERS, "Cache-Control": "no-cache", ...extra });

  function serveFile(req, res, file) {
    const type = TYPES[path.extname(file).toLowerCase()] ?? "application/octet-stream";
    const stat = fs.statSync(file);
    res.writeHead(200, headers({ "Content-Type": type, "Content-Length": stat.size }));
    if (req.method === "HEAD") return res.end();
    fs.createReadStream(file).pipe(res);
  }

  function serveStatic(req, res, pathname) {
    const root = path.resolve(dist);
    let file = path.resolve(root, `.${decodeURIComponent(pathname)}`);
    if (file !== root && !file.startsWith(`${root}${path.sep}`)) return notFound(res);
    if (fs.existsSync(file) && fs.statSync(file).isDirectory()) file = path.join(file, "index.html");
    if (!fs.existsSync(file)) {
      // The single-page app's own routes: the dev server's historyApiFallback.
      if (path.extname(pathname) !== "") return notFound(res);
      file = path.join(root, "index.html");
    }
    serveFile(req, res, file);
  }

  const notFound = (res) => {
    res.writeHead(404, headers({ "Content-Type": "text/plain" }));
    res.end("not found\n");
  };

  function proxy(req, res) {
    const up = http.request(
      { host: "127.0.0.1", port: upstreamPort, path: req.url, method: req.method, headers: { ...req.headers, host: `127.0.0.1:${upstreamPort}` } },
      (answer) => {
        res.writeHead(answer.statusCode, { ...answer.headers, ...ISOLATION_HEADERS });
        answer.pipe(res);
      },
    );
    up.on("error", (e) => {
      res.writeHead(502, headers({ "Content-Type": "text/plain" }));
      res.end(`the portal's dev server isn't answering yet (${e.code ?? e.message}); it is still compiling\n`);
    });
    req.pipe(up);
  }

  const server = http.createServer((req, res) => {
    const url = new URL(req.url, "http://front.invalid");
    try {
      if (url.pathname === "/global-settings.json") {
        res.writeHead(200, headers({ "Content-Type": "application/json; charset=utf-8" }));
        return res.end(req.method === "HEAD" ? undefined : settingsText);
      }
      if (release && url.pathname.startsWith(WELL_KNOWN_RELEASE)) {
        if (url.pathname === `${WELL_KNOWN_RELEASE}${release.package}/${release.version}`) {
          const type = path.extname(release.file) === ".json" ? "application/json; charset=utf-8" : "application/octet-stream";
          res.writeHead(200, headers({ "Content-Type": type, "Access-Control-Allow-Origin": "*", "Cache-Control": "no-store" }));
          return res.end(req.method === "HEAD" ? undefined : fs.readFileSync(release.file));
        }
        return notFound(res);
      }
      if (dist) return serveStatic(req, res, url.pathname);
      return proxy(req, res);
    } catch (e) {
      log(`front: ${req.method} ${url.pathname}: ${e.message}`);
      res.writeHead(500, headers({ "Content-Type": "text/plain" }));
      res.end("error\n");
    }
  });

  // The dev server's hot-reload socket.
  server.on("upgrade", (req, socket, head) => {
    if (!upstreamPort) return socket.destroy();
    const up = net.connect(upstreamPort, "127.0.0.1", () => {
      const lines = Object.entries({ ...req.headers, host: `127.0.0.1:${upstreamPort}` }).map(([k, v]) => `${k}: ${v}`);
      up.write(`${req.method} ${req.url} HTTP/1.1\r\n${lines.join("\r\n")}\r\n\r\n`);
      if (head?.length) up.write(head);
      socket.pipe(up).pipe(socket);
    });
    up.on("error", () => socket.destroy());
    socket.on("error", () => up.destroy());
  });

  return { server, settingsText };
}
