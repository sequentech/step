# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Durable batch status for Prometheus; Alertmanager consumes alerting rules."""
import json
import os
import re
import time
from urllib.parse import quote, urlsplit
from urllib.request import Request, urlopen

LABEL = re.compile(r"[a-zA-Z0-9_-]{1,100}")


def target(registry, name):
    entries = json.loads(registry.read_text())["targets"]
    found = [entry for entry in entries if entry["name"] == name]
    if len(found) != 1 or not found[0].get("enabled"):
        raise ValueError("Target is missing, ambiguous or disabled")
    value = found[0]
    if value.get("synthetic_tenant") is not True:
        raise ValueError("A designated synthetic tenant is required")
    for key in ("name", "tenant_id"):
        if not LABEL.fullmatch(value[key]): raise ValueError(f"Invalid {key}")
    for key in ("portal_url", "keycloak_url", "graphql_url", "admin_url", "verifier_url", "results_url"):
        endpoint = urlsplit(value[key])
        if endpoint.scheme != "https" or not endpoint.hostname or endpoint.username or endpoint.password:
            raise ValueError(f"Expected a credential-free HTTPS {key}")
    if not value.get("storage_origins") or any(urlsplit(url).scheme != "https" for url in value["storage_origins"]):
        raise ValueError("Explicit HTTPS publication storage origins are required")
    if not 1 <= value.get("max_workers", 0) <= 4 or not 1 <= value.get("max_voters", 0) <= 10000:
        raise ValueError("Target needs bounded worker and voter limits")
    if not 1 <= value.get("max_concurrency", 1) <= 200:
        raise ValueError("Target needs a total concurrency limit between 1 and 200")
    return value


def validate_workload(selected, *, kind, engine, preset, workers, concurrency):
    """Apply the same finite workload budget in dispatch and each runner phase."""
    count = 8 if kind == "probe" else {"smoke": 8, "small": 100, "medium": 1000}[preset]
    if not 1 <= workers <= selected["max_workers"] or count > selected["max_voters"]:
        raise ValueError("Requested workload exceeds the registered worker/voter limits")
    if not 1 <= concurrency <= 50 or workers * concurrency > selected.get("max_concurrency", 1):
        raise ValueError("Requested workload exceeds the registered total concurrency limit")
    if engine == "chromium" and concurrency > 2:
        raise ValueError("Chromium is limited to two concurrent voters per bounded runner")
    if kind == "probe" and (engine != "chromium" or workers != 1 or concurrency != 1):
        raise ValueError("Browser probes use one worker and one concurrent voter")
    if kind == "load" and not selected.get("allow_load", False):
        raise ValueError("Load generation is not enabled for this target")
    return count


class Status:
    def __init__(self, url, environment, tenant, kind="probe", engine="chromium"):
        labels = {"environment": environment, "tenant": tenant, "kind": kind, "engine": engine}
        if not all(LABEL.fullmatch(value) for value in labels.values()):
            raise ValueError("Invalid monitoring labels")
        self.url = url.rstrip("/") + "/metrics/job/step_synthetic/" + "/".join(
            f"{key}/{quote(value, safe='')}" for key, value in labels.items())

    def send(self, metrics):
        body = "".join(f"# TYPE step_synthetic_{name} gauge\nstep_synthetic_{name} {float(value)}\n"
                       for name, value in metrics.items()).encode()
        headers = {"Content-Type": "text/plain; version=0.0.4"}
        if token := os.environ.get("E2E_METRICS_TOKEN"):
            headers["Authorization"] = "Bearer " + token
        # POST preserves last-success on failures and the prior result at start.
        request = Request(self.url, data=body, headers=headers, method="POST")
        with urlopen(request, timeout=10) as response:
            if response.status // 100 != 2: raise RuntimeError("Metrics delivery failed")

    def start(self): self.send({"last_start_seconds": time.time(), "active": 1})

    def finish(self, success, duration, cleanup_ok=True):
        values = {"last_finish_seconds": time.time(), "last_run_success": int(success),
                  "duration_seconds": duration, "active": 0, "cleanup_success": int(cleanup_ok)}
        if success: values["last_success_seconds"] = time.time()
        self.send(values)
