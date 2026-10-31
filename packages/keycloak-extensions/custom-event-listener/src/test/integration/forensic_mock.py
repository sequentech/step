# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Synthetic policy/token/forensic endpoints; never contacts a real Harvest service."""

import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

POLICIES = {}
CAPTURED = []


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def reply(self, status, body):
        raw = b"" if status == 204 else json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(raw)))
        self.end_headers()
        self.wfile.write(raw)

    def do_GET(self):
        if self.path == "/captured":
            return self.reply(200, CAPTURED)
        if self.path.startswith("/privacy/sign-in-network/policy?"):
            from urllib.parse import parse_qs, urlsplit

            event = parse_qs(urlsplit(self.path).query)["election_event_id"][0]
            return self.reply(200, {"network_data_policy": POLICIES[event]})
        self.reply(404, {})

    def do_POST(self):
        raw = self.rfile.read(int(self.headers.get("Content-Length", "0")))
        if self.path.endswith("/protocol/openid-connect/token"):
            return self.reply(200, {"access_token": "fixture-service-token", "expires_in": 300})
        if self.path == "/policies":
            POLICIES.update(json.loads(raw))
            return self.reply(200, {})
        if self.path == "/privacy/sign-in-network":
            CAPTURED.append(json.loads(raw))
            return self.reply(204, {})
        self.reply(404, {})


ThreadingHTTPServer(("0.0.0.0", 8081), Handler).serve_forever()
