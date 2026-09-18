# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
import http.server
import json
import tempfile
import threading
import unittest
from pathlib import Path
from unittest.mock import patch
from runner.monitoring import Status, target
from runner.live import Samples
from runner.stack import project_name, host_path
from runner.process import ROOT


class MonitoringTests(unittest.TestCase):
    def test_post_preserves_previous_success_and_start_does_not_clear_failure(self):
        received = []
        class Receiver(http.server.BaseHTTPRequestHandler):
            def do_POST(self):
                received.append(self.rfile.read(int(self.headers["Content-Length"])).decode())
                self.send_response(202)
                self.end_headers()
            def log_message(self, *_): pass
        server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Receiver)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            status = Status(f"http://127.0.0.1:{server.server_port}", "test", "tenant")
            status.finish(True, 3)
            status.start()
            status.finish(False, 2)
            status.finish(True, 1)
            self.assertIn("last_success_seconds", received[0])
            self.assertNotIn("last_run_success", received[1])
            self.assertNotIn("last_success_seconds", received[2])
            self.assertIn("last_run_success 0.0", received[2])
            self.assertIn("last_run_success 1.0", received[3])
        finally:
            server.shutdown()
            server.server_close()
            thread.join()

    def test_disabled_target_fails_before_network_access(self):
        with tempfile.TemporaryDirectory() as temp:
            registry = Path(temp) / "targets.json"
            registry.write_text(json.dumps({"targets": [{"name": "prod", "enabled": False}]}))
            with self.assertRaises(ValueError): target(registry, "prod")

    def test_cleanup_cannot_escape_owned_project_names(self):
        for value in ("../other", "other_project", "", "x" * 57):
            with self.assertRaises(ValueError): project_name(value)
        self.assertEqual(project_name("test-12"), "step-e2e-test-12")

    def test_devcontainer_bind_sources_use_daemon_host_paths(self):
        with patch.dict("os.environ", {"LOCAL_WORKSPACE_FOLDER": "/host/checkout"}):
            self.assertEqual(host_path(ROOT / ".e2e/runs/one"), "/host/checkout/.e2e/runs/one")

    def test_streaming_samples_do_not_duplicate_partial_records_or_k6_jsonl(self):
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp)
            shard = directory / "results/000000"
            shard.mkdir(parents=True)
            sample = json.dumps({"start": 0, "end": 1200, "passed": True, "receipt": "receipt"})
            log = shard / "worker.log"
            log.write_text("noise\nRESULT " + sample[:20])
            metrics = Samples()
            metrics.poll(directory, "k6")
            self.assertEqual(metrics.completed, 0)
            with log.open("a") as output: output.write(sample[20:] + "\n")
            (shard / "samples.jsonl").write_text(sample + "\n")
            metrics.poll(directory, "k6")
            metrics.poll(directory, "k6")
            self.assertEqual((metrics.completed, metrics.accepted), (1, 1))
            self.assertIn('bucket{le="1"} 0', metrics.text())
            self.assertIn('bucket{le="2.5"} 1', metrics.text())
