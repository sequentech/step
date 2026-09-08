# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Internal coordinator for step-cli load; operators use the Rust CLI contract.

One run contains settings, private preparation logs, and immutable worker inputs.
Execution always produces a report, including after a worker exits unsuccessfully.
"""
from __future__ import annotations

from concurrent.futures import ThreadPoolExecutor
import json
import os
import re
import urllib.error
import urllib.request
from pathlib import Path
import shutil
import socket
import subprocess
import sys
from urllib.parse import urlsplit

import runner
import provision
import aggregate

HERE = Path(__file__).resolve().parent


def check(settings: dict) -> None:
    """Fail before provisioning when local preparation dependencies are unavailable."""
    runtime = settings["runtime"]
    for command in (runtime["python"], runtime["k6"]):
        if not shutil.which(command):
            raise ValueError(f"Missing executable: {command}; enter devenv shell")
    import matplotlib  # noqa: F401

    if settings["workload"]["engine"] == "chromium":
        subprocess.run(
            [
                runtime["node"],
                "-e",
                "const {chromium}=require('@playwright/test'); chromium.launch({headless:true,executablePath:process.argv[1]||undefined}).then(b=>b.close()).catch(e=>{console.error(e.message);process.exit(1)})",
                runtime.get("chromium") or "",
            ],
            cwd=runtime["playwright_dir"],
            check=True,
        )
    units = {"ms": 0.001, "s": 1, "m": 60, "h": 3600}
    timeout = sum(
        float(value) * units[unit]
        for value, unit in re.findall(
            r"([0-9.]+)(ms|s|m|h)", settings["workload"]["request_timeout"]
        )
    )
    target = settings["target"]
    endpoints = [
        target["portal_url"],
        target["keycloak_url"],
        target["graphql_url"],
        *target["storage_origins"],
    ]
    for endpoint in endpoints:
        try:
            with urllib.request.urlopen(endpoint, timeout=timeout):
                pass
        except urllib.error.HTTPError as error:
            # Protected APIs and buckets can reject anonymous probes while reachable.
            if error.code >= 500:
                raise ValueError(
                    f"Target service is unavailable: {endpoint} (HTTP {error.code})"
                ) from None
        except urllib.error.URLError:
            raise ValueError(
                f"Cannot reach target: {endpoint}; check DNS, TLS trust and network access"
            ) from None
    print("Configuration, target connectivity and preparation dependencies are valid.")


def path_from(value: str | None, base: Path) -> Path | None:
    """Resolve operator paths against the configuration, never a worker's current directory."""
    if value is None:
        return None
    path = Path(value)
    return (base / path).resolve() if not path.is_absolute() else path


def worker_config(settings: dict) -> dict:
    """Translate the public schema into the one shared engine protocol."""
    target, workload = settings["target"], settings["workload"]
    origins = {
        f"{urlsplit(url).scheme}://{urlsplit(url).netloc}"
        for url in [
            target["portal_url"],
            target["keycloak_url"],
            target["graphql_url"],
            *target["storage_origins"],
        ]
    }
    config = dict(target, **workload)
    config.update(
        vus=workload["concurrency"],
        allowed_origins=sorted(origins),
        goals=dict(settings["goals"]),
    )
    if settings["min_casts_per_second"]:
        config["goals"]["min_casts_per_second"] = settings["min_casts_per_second"]
    config.update(settings["preparation"])
    config["runtime"] = settings["runtime"]
    config["reporting"] = settings["reporting"]
    config["resources"] = settings["execution"]["resources"]
    config["native_cli"] = True
    return config


def prepare(config_path: Path, directory: Path, settings: dict) -> None:
    """Provision once, record the original configuration, then freeze worker inputs."""
    check(settings)
    runner.password(worker_config(settings))
    directory.mkdir(parents=True, exist_ok=False)
    (directory / "settings.yaml").write_text(json.dumps(settings, indent=2) + "\n")
    config = worker_config(settings)
    preparation = settings["preparation"]
    base = config_path.parent
    cli = Path(os.environ["STEP_LOAD_CLI"])
    existing = path_from(preparation["existing_event"], base)
    if existing:
        event = runner.read(existing)
        if event["tenant_id"] != config["tenant_id"]:
            raise ValueError("Existing event belongs to a different tenant")
        config.update(
            {
                key: event[key]
                for key in (
                    "election_event_id",
                    "election_id",
                    "realm",
                    "area_name",
                    "login_url",
                )
            }
        )
        runner.validate(config)
        runner.census(config, directory / "census")
        runner.import_census(
            config,
            directory / "census",
            cli,
            settings["target"]["upload_mode"] == "local",
        )
    else:
        provision.provision(
            config,
            path_from(preparation["template"], base) or HERE / "fixtures/election.json",
            directory / "setup",
            cli,
            settings["target"]["upload_mode"] == "local",
            preparation["threshold"],
            path_from(preparation["publication_preparer"], base),
        )
        config = runner.read(directory / "setup/config.json")
    runner.prepare(
        config,
        directory / "inputs",
        cli,
        path_from(preparation["choices"], base),
        settings["execution"]["workers"],
    )
    print(f"Prepared {config['count']:,} {config['engine']} journeys in {directory}")
    print(f"Next: step-cli load run {directory}")


def command(arguments: list[str], **kwargs) -> None:
    """Execute an argument vector and propagate failure without shell expansion."""
    subprocess.run(arguments, check=True, **kwargs)


def kubernetes(directory: Path, settings: dict, workers: int) -> None:
    """Provision one run volume, transfer inputs, execute indexed workers, collect results.

    Resources are retained after failure for reconciliation. No pod retry may reclaim
    attempted voter shards. Only the synthetic password enters the Kubernetes Secret.
    """
    import hashlib

    execution = settings["execution"]
    if not execution["storage_class"]:
        raise ValueError("execution.storage_class must support ReadWriteMany")
    name = "voting-load-" + hashlib.sha256(str(directory).encode()).hexdigest()[:12]
    kubectl = ["kubectl", "--namespace", execution["namespace"]]
    secret = {
        "apiVersion": "v1",
        "kind": "Secret",
        "metadata": {"name": name},
        "stringData": {
            "password": runner.password(runner.read(directory / "inputs/config.json"))
        },
    }
    command([*kubectl, "create", "-f", "-"], input=json.dumps(secret), text=True)
    volume = {
        "apiVersion": "v1",
        "kind": "PersistentVolumeClaim",
        "metadata": {"name": name},
        "spec": {
            "accessModes": ["ReadWriteMany"],
            "storageClassName": execution["storage_class"],
            "resources": {"requests": {"storage": execution["storage_size"]}},
        },
    }
    transfer = {
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": {"name": name},
        "spec": {
            "securityContext": {
                "runAsUser": os.getuid(),
                "runAsGroup": os.getgid(),
                "fsGroup": os.getgid(),
            },
            "containers": [
                {
                    "name": "transfer",
                    "image": execution["image"],
                    "command": ["sleep", "infinity"],
                    "volumeMounts": [{"name": "load", "mountPath": "/load"}],
                }
            ],
            "volumes": [{"name": "load", "persistentVolumeClaim": {"claimName": name}}],
        },
    }
    for resource in (volume, transfer):
        command([*kubectl, "create", "-f", "-"], input=json.dumps(resource), text=True)
    command(
        [
            *kubectl,
            "wait",
            f"pod/{name}",
            "--for=condition=Ready",
            "--timeout",
            execution["wait_timeout"],
        ]
    )
    command([*kubectl, "cp", str(directory / "inputs") + "/.", f"{name}:/load"])
    output = subprocess.check_output(
        [
            sys.executable,
            str(HERE / "runner.py"),
            "pods",
            str(directory / "inputs"),
            "--nodes",
            str(workers),
            "--image",
            execution["image"],
            "--pvc",
            name,
            "--secret",
            name,
        ],
        text=True,
    )
    job = json.loads(output)
    job["metadata"]["name"] = name
    runner.save(directory / "job.json", job)
    command([*kubectl, "create", "-f", str(directory / "job.json")])
    try:
        command(
            [
                *kubectl,
                "wait",
                f"job/{name}",
                "--for=condition=Complete",
                "--timeout",
                execution["wait_timeout"],
            ]
        )
    finally:
        command([*kubectl, "cp", f"{name}:/load/.", str(directory / "inputs")])
    command([*kubectl, "delete", "pod", name])
    print(
        f"Collected results. Retained Job, PVC and Secret: {execution['namespace']}/{name}"
    )


def report(directory: Path, dsn_env: str | None = None) -> None:
    """Expose portable reports at the run root while private samples remain under inputs."""
    try:
        aggregate.report(directory / "inputs", dsn_env)
    finally:
        for name in (
            "report.html",
            "performance.svg",
            "performance.md",
            "results.json",
        ):
            source = directory / "inputs" / name
            if source.exists():
                shutil.copyfile(source, directory / name)
    print(f"Report: {directory / 'report.html'}")


def mapped_mount(directory: Path, mounts: list[dict]) -> Path:
    """Map a coordinator path to the daemon host using its longest enclosing bind mount."""
    for mount in sorted(
        mounts, key=lambda item: len(item["Destination"]), reverse=True
    ):
        try:
            relative = directory.relative_to(mount["Destination"])
        except ValueError:
            continue
        return Path(mount["Source"]) / relative
    raise ValueError(
        "Prepared run is outside shared Docker mounts; set execution.docker_mount_source"
    )


def docker_mount(directory: Path, execution: dict) -> Path:
    """Resolve bind mounts correctly for both host coordinators and devcontainers."""
    if execution.get("docker_mount_source"):
        return Path(execution["docker_mount_source"])
    if not Path("/.dockerenv").exists():
        return directory
    mounts = subprocess.check_output(
        ["docker", "inspect", socket.gethostname(), "--format", "{{json .Mounts}}"],
        text=True,
    )
    return mapped_mount(directory, json.loads(mounts))


def run(directory: Path, workers: int, executor: str) -> None:
    """Run the selected executor and retain a readable report even after partial failure."""
    settings = runner.read(directory / "settings.yaml")
    inputs = directory / "inputs"
    (directory / "execution.json").write_text(
        json.dumps(dict(workers=workers, executor=executor))
    )
    try:
        if executor == "local":
            with ThreadPoolExecutor(max_workers=workers) as pool:
                futures = [
                    pool.submit(runner.node, inputs, index, workers)
                    for index in range(workers)
                ]
                failures = [str(f.exception()) for f in futures if f.exception()]
            if failures:
                raise RuntimeError("; ".join(failures))
        elif executor == "docker":
            execution = settings["execution"]
            command(
                [
                    sys.executable,
                    str(HERE / "runner.py"),
                    "containers",
                    str(inputs),
                    "--nodes",
                    str(workers),
                    "--image",
                    execution["image"],
                    "--network",
                    execution["network"],
                    "--mount-source",
                    str(docker_mount(inputs, execution)),
                ]
            )
        elif executor == "kubernetes":
            kubernetes(directory, settings, workers)
        else:
            raise ValueError(f"Unsupported executor: {executor}")
    finally:
        report(directory)


def screenshot(directory: Path, destination: Path) -> None:
    """Capture the actual standalone report, with fonts and SVG layout fully loaded."""
    settings = runner.read(directory / "settings.yaml")
    runtime = settings["runtime"]
    destination.parent.mkdir(parents=True, exist_ok=True)
    script = """const {createRequire}=require('module');
const requireFrom=createRequire(process.argv[1]+'/package.json');
const {chromium}=requireFrom('@playwright/test');
(async()=>{const browser=await chromium.launch({headless:true,executablePath:process.argv[4]||undefined});
try{const page=await browser.newPage({viewport:{width:Number(process.argv[5]),height:Number(process.argv[6])},deviceScaleFactor:1});
await page.goto(require('url').pathToFileURL(process.argv[2]).href);
await page.evaluate(()=>document.fonts.ready);await page.screenshot({path:process.argv[3],fullPage:true});}
finally{await browser.close();}})().catch(e=>{console.error(e);process.exit(1)});"""
    command(
        [
            runtime["node"],
            "-e",
            script,
            runtime["playwright_dir"],
            str((directory / "report.html").resolve()),
            str(destination.resolve()),
            runtime.get("chromium") or "",
            str(settings["reporting"]["screenshot_width"]),
            str(settings["reporting"]["screenshot_height"]),
        ]
    )


def main() -> None:
    """The Rust CLI supplies validated JSON and positional internal arguments."""
    os.umask(0o077)
    action, *args = sys.argv[1:]
    if action == "check":
        check(json.loads(args[0]))
    elif action == "prepare":
        prepare(Path(args[0]), Path(args[1]).resolve(), json.loads(args[2]))
    elif action == "run":
        run(Path(args[0]), int(args[1]), args[2])
    elif action == "report":
        directory = Path(args[0])
        report(directory, args[1] or None)
        if len(args) > 2 and args[2]:
            screenshot(directory, Path(args[2]))
    else:
        raise ValueError("Unsupported internal operation")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"Load failed: {error}", file=sys.stderr)
        sys.exit(1)
