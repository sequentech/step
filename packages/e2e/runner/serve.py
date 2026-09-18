# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Serve actual built portals with SPA routing and WASM isolation headers."""
import functools
import http.server
import json
import os
import threading
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


class Portal(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        self.send_header("Cross-Origin-Resource-Policy", "cross-origin")
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

    def do_GET(self):
        if self.path.split("?")[0] == "/global-settings.json":
            # Production supplies runtime settings separately from webpack output.
            portal = Path(self.directory).name
            settings = json.loads((ROOT / "packages" / portal / "public/global-settings.json").read_text())
            settings.update({"KEYCLOAK_URL": "http://keycloak:8090/",
                "HASURA_URL": "http://graphql-engine:8080/v1/graphql", "DISABLE_AUTH": False,
                "BALLOT_VERIFIER_URL": "http://portals:3001/", "VOTING_PORTAL_URL": "http://portals:3000/",
                "RESULTS_PORTAL_URL": "http://portals:3004/", "PUBLIC_BUCKET_URL": "http://minio-proxy:9002/public/"})
            body = json.dumps(settings).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return
        if not Path(self.translate_path(self.path.split("?")[0])).exists() and "." not in self.path.rsplit("/", 1)[-1]:
            self.path = "/index.html"
        super().do_GET()


if __name__ == "__main__":
    servers = []
    mode = "coverage" if os.environ.get("E2E_COVERAGE", "none") != "none" else "normal"
    for name, port in [("voting-portal", 3000), ("ballot-verifier", 3001), ("admin-portal", 3002), ("results-portal", 3004)]:
        root = ROOT / ".e2e/web" / mode / name
        if root.exists():
            server = http.server.ThreadingHTTPServer(("0.0.0.0", port), functools.partial(Portal, directory=str(root)))
            threading.Thread(target=server.serve_forever, daemon=True).start()
            servers.append(server)
    if not servers:
        raise SystemExit("No built portal artifacts")
    threading.Event().wait()
