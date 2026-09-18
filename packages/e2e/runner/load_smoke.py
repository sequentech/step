# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Four voters per engine against the owned local stack, including live telemetry."""
import json
import os
import shutil
import time
import requests
import yaml
from .bootstrap import ARTIFACTS, authenticate, client_origins, wait_http
from .live import run as run_live
from .monitoring import Status
from .process import execute
from .remote import owned_cleanup


def main():
    if os.environ.get("E2E_COVERAGE", "none") != "none":
        raise RuntimeError("Load measurements require normal binaries")
    wait_http("http://pushgateway:9091/-/ready")
    for engine in ("k6", "chromium"):
        cli, _ = authenticate()
        log = ARTIFACTS / f"private/load-{engine}.log"
        settings = yaml.safe_load((ARTIFACTS / "private/workload.yaml").read_text())
        settings["workload"].update(engine=engine, count=4, shard_size=2,
                                    username_prefix=f"smoke-{engine}-", concurrency=1)
        settings["execution"]["workers"] = 2
        config = ARTIFACTS / f"private/load-{engine}.yaml"
        config.write_text(yaml.safe_dump(settings))
        prepared = ARTIFACTS / f"private/load-{engine}"
        env = {"LOAD_PASSWORD": "E2e-synthetic-2026!", "E2E_LOCAL_STACK": "1",
               "E2E_AUDIT_DSN": "postgres://e2e_audit:e2e-audit@postgres:5432/postgres"}
        status = Status("http://pushgateway:9091", "local", settings["target"]["tenant_id"], "load", engine)
        status.start()
        started = time.monotonic()
        passed = cleanup_ok = False
        try:
            execute([cli, "load", "prepare", str(config), "--output", str(prepared)], env=env, log=log, timeout=900)
            data = json.loads((prepared / "inputs/config.json").read_text())
            client_origins(data["realm"], "voting-portal", ["http://portals:3000"])
            run_live([cli, "load", "run", str(prepared)], directory=prepared / "inputs", engine=engine,
                     metrics_url=f"http://pushgateway:9091/metrics/job/step_load/environment/local/engine/{engine}/run/smoke/worker/coordinator",
                     env=env, log=log, timeout=600)
            execute([cli, "load", "report", str(prepared), "--dsn-env", "E2E_AUDIT_DSN"], env=env, log=log, timeout=120)
            public = ARTIFACTS / "report" / f"load-{engine}"
            public.mkdir(parents=True, exist_ok=True)
            for name in ("report.html", "performance.md", "performance.svg", "results.json"):
                shutil.copy2(prepared / name, public / name)
            # Real Pushgateway ingestion, not a mocked metrics client.
            requests.get("http://pushgateway:9091/metrics", timeout=5).raise_for_status()
            passed = True
        finally:
            try:
                authenticate()
                owned_cleanup(cli, prepared, {}, log)
                cleanup_ok = True
            finally:
                status.finish(passed and cleanup_ok, time.monotonic() - started, cleanup_ok)


if __name__ == "__main__": main()
