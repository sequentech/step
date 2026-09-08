# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Local diagnostic action proxy. Records timings/status only, never credentials or bodies.

Point only the local Hasura get_ballot_files_urls action at this listener. The
upstream must be an explicit trusted local HTTP service. Restore metadata after use.
"""
import argparse
import http.client
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import threading
import time
from urllib.parse import urlsplit


def main():
    """Run the local recording proxy with explicit upstream and output paths."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--upstream", required=True)
    parser.add_argument("--log", type=Path, required=True)
    parser.add_argument(
        "--host",
        default="127.0.0.1",
        help="Listener address; select a container-reachable address explicitly when needed",
    )
    parser.add_argument("--port", type=int, default=3032)
    args = parser.parse_args()
    upstream = urlsplit(args.upstream)
    if (
        upstream.scheme != "http"
        or upstream.username
        or upstream.password
        or upstream.path not in ("", "/")
    ):
        parser.error("Use an HTTP origin without credentials or a path")
    os.umask(0o077)
    args.log.parent.mkdir(parents=True, exist_ok=True)
    lock = threading.Lock()

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_POST(self):
            if self.path != "/get-ballot-files-urls":
                self.send_error(404)
                return
            started = time.perf_counter()
            status, size = 502, 0
            connection = http.client.HTTPConnection(
                upstream.hostname, upstream.port, timeout=30
            )
            try:
                body = self.rfile.read(int(self.headers.get("Content-Length", 0)))
                headers = {
                    k: v
                    for k, v in self.headers.items()
                    if k.lower() not in {"host", "connection", "transfer-encoding"}
                }
                connection.request("POST", self.path, body=body, headers=headers)
                response = connection.getresponse()
                payload = response.read()
                status, size = response.status, len(payload)
                self.send_response(status)
                self.send_header(
                    "Content-Type",
                    response.getheader("Content-Type", "application/json"),
                )
                self.send_header("Content-Length", str(size))
                self.end_headers()
                self.wfile.write(payload)
            except Exception:
                self.send_error(502)
            finally:
                connection.close()
                record = dict(
                    method="POST",
                    path=self.path,
                    status=status,
                    response_bytes=size,
                    duration_ms=(time.perf_counter() - started) * 1000,
                    timestamp_ms=time.time() * 1000,
                )
                with lock, args.log.open("a") as stream:
                    stream.write(json.dumps(record) + "\n")

    ThreadingHTTPServer((args.host, args.port), Handler).serve_forever()


if __name__ == "__main__":
    main()
