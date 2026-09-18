# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Registered synthetic tenants only. Never starts, migrates or stops target services."""
import json
import os
import time
import uuid
import shutil
from pathlib import Path
import yaml
from .process import ROOT, execute, save
from .monitoring import Status, target
from .live import run as run_live
from .stack import project_name


def authenticate(cli, selected, env, log):
    execute([str(cli), "step", "config", "--tenant-id", selected["tenant_id"],
             "--endpoint-url", selected["graphql_url"], "--keycloak-url", selected["keycloak_url"],
             "--keycloak-user", os.environ["E2E_ADMIN_USERNAME"], "--keycloak-password", os.environ["E2E_ADMIN_PASSWORD"],
             "--keycloak-client-id", os.environ["E2E_CLIENT_ID"], "--keycloak-client-secret", os.environ["E2E_CLIENT_SECRET"]], env=env, log=log)


def owned_cleanup(cli, prepared, env, log):
    owned = prepared / "setup/setup-state.json"
    if not owned.exists(): owned = prepared / "inputs/config.json"
    if owned.exists():
        event = json.loads(owned.read_text()).get("election_event_id")
        if event:
            execute([str(cli), "step", "delete-election-event", "--election-event-id", event], env=env, log=log, timeout=360)


def phase(args, selected, artifacts, cli, env, status):
    """GitHub workers receive only frozen synthetic inputs, never admin sessions."""
    transfer = artifacts / "transfer"
    state = json.loads((transfer / "coordinator.json").read_text())
    if state["target"] != selected["name"] or state["tenant"] != selected["tenant_id"]:
        raise ValueError("Coordinator target mismatch")
    if state["engine"] != args.engine or state["workers"] != args.workers:
        raise ValueError("Worker topology does not match preparation")
    log = artifacts / "private/run.log"
    if args.phase == "worker":
        if not 0 <= args.index < args.workers: raise ValueError("Invalid worker index")
        metrics_url = os.environ["E2E_PUSHGATEWAY_URL"].rstrip("/") + (
            f"/metrics/job/step_load/environment/{selected['name']}/engine/{args.engine}/run/{args.run_id}/worker/{args.index}")
        run_live([str(cli), "load", "worker", str(transfer / "inputs"), "--index", str(args.index), "--workers", str(args.workers)],
                 directory=transfer / "inputs", engine=args.engine, metrics_url=metrics_url, env=env, log=log)
    elif args.phase == "report":
        success = False
        try:
            execute([str(cli), "load", "report", str(transfer), "--dsn-env", "E2E_AUDIT_DSN"], env=env, log=log)
            success = True
        finally:
            save(transfer / "outcome.json", {"success": success})
            status.send({"last_run_success": int(success)})
    elif args.phase == "cleanup":
        success = (transfer / "outcome.json").exists() and json.loads((transfer / "outcome.json").read_text())["success"]
        cleanup_ok = False
        try:
            authenticate(cli, selected, env, log)
            owned_cleanup(cli, transfer, env, log)
            cleanup_ok = True
        finally:
            status.finish(success and cleanup_ok, time.time() - state["started"], cleanup_ok)


def run(args):
    selected = target(args.registry, args.target)
    workers = args.workers
    count = 8 if args.command == "probe" else {"smoke": 8, "small": 100, "medium": 1000}[args.preset]
    if workers > selected["max_workers"] or count > selected["max_voters"]:
        raise ValueError("Requested workload exceeds the registered target limits")
    if args.command == "load" and not selected.get("allow_load", False):
        raise ValueError("Load generation is not enabled for this target")
    if args.command == "probe" and (args.phase != "full" or args.engine != "chromium"):
        raise ValueError("Browser probes use the full Chromium lifecycle")
    if args.phase != "full" and not args.run_id: raise ValueError("Distributed phases require an explicit run ID")
    run_id = args.run_id or "synthetic-" + uuid.uuid4().hex
    project_name(run_id)
    artifacts = ROOT / ".e2e/runs" / run_id
    private = artifacts / "private"
    private.mkdir(parents=True, mode=0o700, exist_ok=args.phase in ("worker", "report", "cleanup"))
    cli = ROOT / ".e2e/bin/normal/step-cli"
    env = {"STEP_CLI_CONFIG_DIR": str(private / "cli"), "E2E_ARTIFACTS": str(artifacts),
           "LOAD_PASSWORD": os.environ["E2E_VOTER_PASSWORD"], "E2E_COVERAGE": "none"}
    log = private / "run.log"
    status = Status(os.environ["E2E_PUSHGATEWAY_URL"], selected["name"], selected["tenant_id"],
                    "probe" if args.command == "probe" else "load", args.engine)
    if args.phase in ("worker", "report", "cleanup"):
        return phase(args, selected, artifacts, cli, env, status)
    status.start() # No traffic if telemetry cannot establish that a run started.
    started, passed, cleanup_ok, handed_off = time.monotonic(), False, True, False
    prepared = private / "prepared"
    try:
        authenticate(cli, selected, env, log)
        config = private / "workload.yaml"
        execute([str(cli), "load", "init", "--target", "remote", "--portal-url", selected["portal_url"],
                 "--storage-origin", selected["storage_origins"][0], "--output", str(config)], env=env, log=log)
        settings = yaml.safe_load(config.read_text())
        settings["target"]["storage_origins"] = selected["storage_origins"]
        settings["workload"].update(engine=args.engine, count=count, concurrency=1,
            shard_size=max(1, (count + workers - 1) // workers), username_prefix=run_id + "-",
            max_duration="10m", journey_timeout_ms=120000)
        settings["execution"].update(workers=workers, executor="local")
        settings["runtime"].update(playwright_dir=str(ROOT / "packages/voting-portal"), chromium=None)
        config.write_text(yaml.safe_dump(settings))
        execute([str(cli), "load", "prepare", str(config), "--output", str(prepared)], env=env, log=log, timeout=900)
        data = json.loads((prepared / "inputs/config.json").read_text())
        if data["tenant_id"] != selected["tenant_id"]:
            raise RuntimeError("Prepared fixture tenant does not match the registry")
        if args.phase == "prepare":
            transfer = artifacts / "transfer"
            transfer.mkdir(mode=0o700)
            (transfer / "inputs").mkdir(mode=0o700)
            (transfer / "setup").mkdir(mode=0o700)
            for name in ("inputs/config.json", "settings.yaml", "inputs/ready.json", "setup/setup-state.json"):
                shutil.copy2(prepared / name, transfer / name)
            for pattern in ("*.jsonl", "*.sha256"):
                for file in (prepared / "inputs").glob(pattern): shutil.copy2(file, transfer / "inputs" / file.name)
            save(transfer / "coordinator.json", {"target": selected["name"], "tenant": selected["tenant_id"],
                "engine": args.engine, "workers": args.workers, "started": time.time()})
            handed_off = True
            return
        if args.command == "probe":
            save(private / "fixture.json", {"tenantId": data["tenant_id"], "eventId": data["election_event_id"],
                "electionId": data["election_id"], "loginUrl": data["login_url"],
                "usernamePrefix": settings["workload"]["username_prefix"], "password": env["LOAD_PASSWORD"],
                "adminUrl": selected["admin_url"], "verifierUrl": selected["verifier_url"], "resultsUrl": selected["results_url"],
                "adminUsername": os.environ["E2E_ADMIN_USERNAME"], "adminPassword": os.environ["E2E_ADMIN_PASSWORD"],
                "auditDsn": os.environ["E2E_AUDIT_DSN"]})
            execute(["yarn", "test", "--grep", "@probe"], cwd=ROOT / "packages/e2e", env=env, log=log, timeout=900)
        else:
            metrics_url = os.environ["E2E_PUSHGATEWAY_URL"].rstrip("/") + (
                f"/metrics/job/step_load/environment/{selected['name']}/engine/{args.engine}/run/{run_id}/worker/coordinator")
            run_live([str(cli), "load", "run", str(prepared)], directory=prepared / "inputs", engine=args.engine,
                     metrics_url=metrics_url, env=env, log=log, timeout=1200)
            execute([str(cli), "load", "report", str(prepared), "--dsn-env", "E2E_AUDIT_DSN"], env=env, log=log, timeout=120)
        passed = True
    finally:
        # Provisioning writes ownership immediately after importing the event,
        # before key ceremony/census. Also clean a partially prepared fixture.
        if not handed_off:
            try:
                if (prepared / "setup/setup-state.json").exists():
                    authenticate(cli, selected, env, log)
                owned_cleanup(cli, prepared, env, log)
            except Exception: cleanup_ok = passed = False
            status.finish(passed, time.monotonic() - started, cleanup_ok)
    if not passed: raise RuntimeError("Synthetic run or owned fixture cleanup failed")
