#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Exercise the shipped edge configuration with a disposable Nginx container."""
import http.client
import io
import json
import os
from pathlib import Path
import socket
import subprocess
import tarfile
import threading
import time
import unittest
import uuid
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = Path(__file__).resolve().parents[2]


class BallotEdgeTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.reads = 0
        cls.authorizations = 0

        class Upstream(BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_GET(self):
                cls.authorizations += 1
                token = self.headers.get("Authorization", "")
                if token == "Bearer denied":
                    self.send_response(403)
                else:
                    self.send_response(204)
                    self.send_header("X-Ballot-Scope", "area-b" if token == "Bearer b" else "area-a")
                self.end_headers()

            def do_POST(self):
                body = self.rfile.read(int(self.headers.get("Content-Length", "0")))
                cls.reads += 1
                payload = json.loads(body)
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                if self.path == "/ballot-box-log" and not payload.get("prefix") and not payload.get("cursor"):
                    self.send_header("X-Ballot-Cacheable", "1")
                self.end_headers()
                self.wfile.write(json.dumps({"reads": cls.reads, "request": payload}).encode())

        cls.backend = ThreadingHTTPServer(("127.0.0.1", 0), Upstream)
        threading.Thread(target=cls.backend.serve_forever, daemon=True).start()
        with socket.socket() as reserved:
            reserved.bind(("127.0.0.1", 0))
            cls.port = reserved.getsockname()[1]
        config = (ROOT / ".devcontainer/harvest-edge/default.conf.template").read_text()
        config = config.replace("${HARVEST_DOMAIN}", f"127.0.0.1:{cls.backend.server_port}")
        config = config.replace("listen 8080;", f"listen {cls.port};")
        cls.name = "vote-privacy-edge-test-" + uuid.uuid4().hex[:12]
        network = os.environ.get("BALLOT_EDGE_TEST_NETWORK")
        if not network:
            current = socket.gethostname()
            exists = subprocess.run(["docker", "inspect", current], capture_output=True).returncode == 0
            network = "container:" + current if exists else "host"
        subprocess.run(["docker", "run", "-d", "--name", cls.name, "--network", network,
                        "nginx:latest", "sleep", "300"], check=True, capture_output=True)
        cls.addClassCleanup(lambda: subprocess.run(["docker", "rm", "-f", cls.name], capture_output=True))
        archive = io.BytesIO()
        with tarfile.open(fileobj=archive, mode="w") as tar:
            data = config.encode()
            member = tarfile.TarInfo("default.conf")
            member.size = len(data)
            tar.addfile(member, io.BytesIO(data))
        subprocess.run(["docker", "cp", "-", cls.name + ":/etc/nginx/conf.d"],
                       input=archive.getvalue(), check=True, capture_output=True)
        cls.proxy = subprocess.Popen(["docker", "exec", cls.name, "nginx", "-g", "daemon off;"],
                                     stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        cls.addClassCleanup(cls.backend.server_close)
        cls.addClassCleanup(cls.backend.shutdown)
        for _ in range(100):
            if cls.proxy.poll() is not None:
                raise RuntimeError(cls.proxy.stderr.read().decode())
            try:
                with socket.create_connection(("127.0.0.1", cls.port), timeout=0.1):
                    break
            except OSError:
                time.sleep(0.05)
        else:
            raise RuntimeError("Nginx did not become ready")

    def request(self, token="a", body=None, path="/ballot-box-log", method="POST"):
        connection = http.client.HTTPConnection("127.0.0.1", self.port, timeout=5)
        connection.request(method, path, json.dumps(body or {"limit": 25}),
                           {"Authorization": "Bearer " + token, "Content-Type": "application/json"})
        response = connection.getresponse()
        result = response.status, response.getheader("X-Ballot-Cache"), response.read()
        connection.close()
        return result

    def test_authorized_shared_page_cache_and_scoped_isolation(self):
        auth = self.authorizations
        first = self.request(body={"limit": 25, "case": "shared"})
        second = self.request("same-area-other-voter", {"limit": 25, "case": "shared"})
        self.assertEqual(first[0], 200)
        self.assertEqual(first[1], "MISS")
        self.assertEqual(second[1], "HIT")
        self.assertEqual(first[2], second[2])
        self.assertGreaterEqual(self.authorizations - auth, 2)
        other = self.request("b", {"limit": 25, "case": "shared"})
        self.assertEqual(other[1], "MISS")
        self.assertNotEqual(first[2], other[2])
        denied = self.request("denied", {"limit": 25, "case": "shared"})
        self.assertEqual(denied[0], 403)
        changed = self.request(body={"limit": 100, "case": "shared"})
        self.assertEqual(changed[1], "MISS")
        self.assertNotEqual(first[2], changed[2])

    def test_a_cached_post_never_answers_another_http_method(self):
        body = {"limit": 25, "case": "method"}
        self.assertEqual(self.request(body=body)[0], 200)
        self.assertEqual(self.request(body=body, method="GET")[0], 204)

    def test_own_search_cursor_and_investigation_responses_are_never_shared(self):
        for path, body in [
            ("/ballot-box-own", {"case": "own"}),
            ("/forensic-network-summary", {"case": "private"}),
            ("/ballot-box-log", {"prefix": "AB"}),
            ("/ballot-box-log", {"cursor": "later"}),
        ]:
            with self.subTest(path=path, body=body):
                first = self.request(body=body, path=path)
                second = self.request(body=body, path=path)
                self.assertEqual(first[0], 200)
                self.assertEqual(second[0], 200)
                self.assertNotEqual(first[2], second[2])
                self.assertNotEqual(second[1], "HIT")


if __name__ == "__main__":
    unittest.main()
