# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""One lifecycle for local devcontainers and GitHub Actions."""
import argparse
import json
import os
import signal
import subprocess
import sys
import time
from pathlib import Path
from datetime import datetime, timezone
from .process import ROOT, docker, execute, save, source_digest
from .stack import Stack, TOOLS


def build_images():
    docker("buildx", "build", "--load", "-t", TOOLS, "-f", ".devcontainer/e2e/Dockerfile", ".devcontainer/e2e", timeout=3600)
    docker("buildx", "build", "--load", "-t", "step-e2e-keycloak:local", "--build-context", "beyond=./beyond",
           "-f", "packages/Dockerfile.keycloak", "packages", timeout=2400)


def summary(directory, stages, success, started):
    public = directory / "report"
    public.mkdir(exist_ok=True)
    lines = ["## E2E execution", "", f"Result: **{'passed' if success else 'failed'}**", "",
             f"Total runner wall time: **{time.monotonic() - started:.1f}s** (image/cache preparation is reported separately by Actions).", "",
             "| Stage | Seconds | Result |", "| --- | ---: | --- |"]
    for name, seconds, result in stages:
        lines.append(f"| {name} | {seconds:.1f} | {result} |")
    browser_report = directory / "private/playwright.json"
    if browser_report.exists():
        stats = json.loads(browser_report.read_text()).get("stats", {})
        lines += ["", f"Browser checks: **{stats.get('expected', 0)} passed, {stats.get('unexpected', 0)} failed, {stats.get('skipped', 0)} skipped**."]
    lines += ["", "Browser traces, session data and raw service logs remain private. Coverage excludes WASM and third-party code.", ""]
    (public / "summary.md").write_text("\n".join(lines))
    save(public / "timings.json", {"success": success, "wall_seconds": time.monotonic() - started, "stages": stages})


def run(args):
    if args.load_smoke and args.coverage != "none":
        raise ValueError("Run load smoke separately from coverage instrumentation")
    stack = Stack(args.run_id, args.coverage)
    claim = stack.directory / "claimed"
    with claim.open("x") as file:
        file.write(datetime.now(timezone.utc).isoformat())
    started, stages, success = time.monotonic(), [], False

    def stage(name, operation):
        before, result = time.monotonic(), "failed"
        print(f"E2E: {name}", flush=True)
        try:
            operation()
            result = "passed"
        finally:
            stages.append((name, time.monotonic() - before, result))

    def bootstrap(name):
        stack.exec("python3", "-m", "e2e.runner.bootstrap", name,
                   env={}, log=stack.directory / "private/bootstrap-driver.log")

    try:
        save(stack.directory / "manifest.json", {"sha": execute(["git", "rev-parse", "HEAD"], capture=True).strip(),
             "source_digest": source_digest(), "coverage": args.coverage, "run_id": args.run_id, "started": time.time()})
        stage("compose", stack.generate)
        stage("runner", lambda: stack.up("runner"))
        stage("service images", lambda: stack.compose("build", "postgres", "postgres-b4", "configure-minio",
                                                      log=stack.directory / "private/service-build.log", timeout=900))
        if not args.skip_build:
            mode = "normal" if args.coverage == "none" else "coverage"
            stage("application build", lambda: stack.exec("bash", "packages/e2e/runner/build.sh", mode,
                                                         log=stack.directory / "private/build.log", timeout=5400))
        stage("databases and queue", lambda: stack.up("postgres", "postgres-keycloak", "postgres-b4", "rabbitmq", "minio", "minio-proxy", "immudb"))
        stage("schema", lambda: bootstrap("migrate"))
        stage("storage", lambda: stack.compose("run", "--rm", "--no-deps", "configure-minio", log=stack.directory / "private/storage.log"))
        stage("authentication", lambda: stack.up("keycloak"))
        stage("signing keys", lambda: bootstrap("jwks"))
        stage("publish signing keys", lambda: stack.compose("run", "--rm", "--no-deps", "--entrypoint", "/bin/sh", "configure-minio", "-ec",
            'mc alias set fixture "$MINIO_PRIVATE_URI" "$MINIO_ROOT_USER" "$MINIO_ROOT_PASSWORD" >/dev/null; mc cp --attr "Cache-Control=max-age=1" /e2e/jwks.json fixture/public/certs.json',
            log=stack.directory / "private/storage.log"))
        stage("API", lambda: stack.up("graphql-engine"))
        stage("metadata", lambda: bootstrap("metadata"))
        stage("backend", lambda: stack.up("b4", "harvest", "windmill", "beat", "trustee1", "trustee2", "portals"))
        stage("election and census", lambda: bootstrap("prepare"))
        if args.coverage == "combined":
            stage("unit tests", lambda: stack.exec("bash", "packages/e2e/coverage/unit.sh", timeout=1800,
                                                   log=stack.directory / "private/unit.log"))
        stage("Chromium", lambda: stack.exec("bash", "-c", 'cd packages/e2e && yarn test --grep "$E2E_GREP"',
            # compose exec needs explicit -e, inherited host environment is not forwarded.
            log=stack.directory / "private/browser.log", timeout=900))
        if args.load_smoke:
            stage("live metrics", lambda: stack.up("pushgateway"))
            stage("two-worker load smoke", lambda: stack.exec("python3", "-m", "e2e.runner.load_smoke",
                log=stack.directory / "private/load-driver.log", timeout=1800))
        success = True
    finally:
        # SIGINT lets Rocket/Tokio finish requests. Native continuous profiles also
        # survive a service crash; coverage reporting requires each expected service.
        try:
            if stack.file.exists():
                try:
                    stage("stop services", lambda: stack.compose("stop", "-t", "45", "harvest", "windmill", "beat", "b4", timeout=200,
                                                                log=stack.directory / "private/shutdown.log"))
                    if args.coverage != "none":
                        stage("coverage report", lambda: stack.exec("python3", "-m", "e2e.runner.coverage", log=stack.directory / "private/coverage.log"))
                except (subprocess.SubprocessError, OSError):
                    success = False
                try:
                    stack.compose("logs", "--no-color", log=stack.directory / "private/services.log", timeout=30)
                finally:
                    try:
                        stage("artifact ownership", stack.reclaim_artifacts)
                    finally:
                        if not args.keep:
                            stage("cleanup", stack.down)
        except BaseException:
            success = False
            raise
        finally:
            summary(stack.directory, stages, success, started)
    if not success:
        raise RuntimeError(f"E2E run failed; inspect {stack.directory}/private")


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("images", help="Build the pinned reusable tool and Keycloak images")
    test = commands.add_parser("run", help="Build, start, test and clean a fresh local environment")
    test.add_argument("--run-id", default=datetime.now(timezone.utc).strftime("local-%Y%m%d-%H%M%S"))
    test.add_argument("--coverage", choices=["none", "e2e", "combined"], default="none")
    test.add_argument("--skip-build", action="store_true", help="Use already built artifacts from this checkout")
    test.add_argument("--keep", action="store_true", help="Retain this run's services for debugging")
    test.add_argument("--load-smoke", action="store_true", help="Also audit four voters per engine with two local workers")
    cleanup = commands.add_parser("cleanup", help="Remove only an explicitly identified run")
    cleanup.add_argument("run_id")
    for name in ("probe", "load"):
        remote = commands.add_parser(name, help="Use an explicitly enabled synthetic target registry entry")
        remote.add_argument("--registry", type=Path, default=ROOT / "packages/e2e/targets.json")
        remote.add_argument("--target", required=True)
        remote.add_argument("--engine", choices=["k6", "chromium"], default="chromium")
        remote.add_argument("--workers", type=int, choices=range(1, 5), default=1)
        remote.add_argument("--concurrency", type=int, choices=range(1, 51), default=1,
                            help="Concurrent voters per worker; the registered total cap also applies")
        remote.add_argument("--preset", choices=["smoke", "small", "medium"], default="smoke")
        remote.add_argument("--phase", choices=["full", "prepare", "worker", "report", "cleanup"], default="full")
        remote.add_argument("--run-id")
        remote.add_argument("--index", type=int, default=0)
    args = parser.parse_args()
    signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(KeyboardInterrupt()))
    try:
        if args.command == "images": build_images()
        elif args.command == "cleanup": Stack(args.run_id).down()
        elif args.command in ("probe", "load"):
            from .remote import run as remote_run
            remote_run(args)
        else: run(args)
    except (Exception, KeyboardInterrupt) as error:
        # Details remain in private files; don't echo subprocess arguments/secrets.
        print(f"E2E stopped ({type(error).__name__}); see the run's private logs.", file=sys.stderr)
        return 1
    return 0
