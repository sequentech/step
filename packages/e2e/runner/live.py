# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Bounded live histogram aggregation for native k6 and Chromium worker records."""
import json
import math
import os
import signal
import subprocess
import time
from urllib.request import Request, urlopen

BUCKETS = (0.1, 0.25, 0.5, 1, 2.5, 5, 10, 30, 60, 120, math.inf)


class Samples:
    def __init__(self):
        self.completed = self.failed = self.accepted = 0
        self.seconds = 0.0
        self.buckets = [0] * len(BUCKETS)
        self.positions = {}

    def add(self, sample):
        seconds = (float(sample["end"]) - float(sample["start"])) / 1000
        if not math.isfinite(seconds) or seconds < 0: raise ValueError("Invalid journey duration")
        self.completed += 1
        self.failed += not sample["passed"]
        self.accepted += bool(sample["passed"] and sample.get("receipt"))
        self.seconds += seconds
        for index, bound in enumerate(BUCKETS): self.buckets[index] += seconds <= bound

    def poll(self, directory, engine):
        # Read one streaming source per engine, never k6's later duplicate JSONL.
        filename = "worker.log" if engine == "k6" else "samples.jsonl"
        for file in directory.glob(f"results/*/{filename}"):
            position = self.positions.get(file, 0)
            with file.open() as source:
                source.seek(position)
                while line := source.readline():
                    if not line.endswith("\n"): break
                    position = source.tell()
                    if engine == "k6":
                        if not line.startswith("RESULT "): continue
                        line = line[7:]
                    self.add(json.loads(line))
            self.positions[file] = position

    def text(self):
        body = f"step_load_completed_total {self.completed}\nstep_load_failed_total {self.failed}\nstep_load_accepted_total {self.accepted}\n"
        body += "# TYPE step_load_journey_seconds histogram\n"
        for bound, count in zip(BUCKETS, self.buckets):
            label = "+Inf" if math.isinf(bound) else str(bound)
            body += f'step_load_journey_seconds_bucket{{le="{label}"}} {count}\n'
        return body + f"step_load_journey_seconds_count {self.completed}\nstep_load_journey_seconds_sum {self.seconds}\n"


def run(command, *, directory, engine, metrics_url, env, log, timeout=1200):
    samples = Samples()
    headers = {"Content-Type": "text/plain; version=0.0.4"}
    if token := os.environ.get("E2E_METRICS_TOKEN"): headers["Authorization"] = "Bearer " + token
    with open(log, "a") as output:
        process = subprocess.Popen(command, env=dict(os.environ, **env), stdout=output,
                                   stderr=subprocess.STDOUT, start_new_session=True)
        started = time.monotonic()
        try:
            while True:
                samples.poll(directory, engine)
                with urlopen(Request(metrics_url, data=samples.text().encode(), headers=headers, method="PUT"), timeout=5) as response:
                    if response.status // 100 != 2: raise RuntimeError("Live metrics delivery failed")
                if process.poll() is not None: break
                if time.monotonic() - started > timeout: raise TimeoutError("Load deadline exceeded")
                time.sleep(2)
            if process.returncode: raise RuntimeError("Load workers failed")
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGTERM)
                try: process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait()
            samples.poll(directory, engine)
            with urlopen(Request(metrics_url, data=samples.text().encode(), headers=headers, method="PUT"), timeout=5): pass
            # Allow two scrapes of the final cumulative histogram, even for an
            # eight-voter smoke. Configure the gateway scrape interval to 5s.
            time.sleep(10)
            # Prometheus retains historical samples; remove dynamic Pushgateway
            # groups so finished runs cannot accumulate cardinality indefinitely.
            with urlopen(Request(metrics_url, headers=headers, method="DELETE"), timeout=5): pass
