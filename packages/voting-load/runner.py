# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Prepare and run finite voting workloads with memory bounded by shard size.

The coordinator holds administrator credentials; workers only need the shared
synthetic voter password. A shard is attempted once, including after a crash.
Use a new census range for a new run rather than silently retrying cast requests.
"""
from __future__ import annotations

import argparse
import base64
from concurrent.futures import ThreadPoolExecutor
import csv
import hashlib
import json
import math
import os
import re
from pathlib import Path
import subprocess
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
STATUS_QUERY = """query GetVoterStatus($electionEventId: String!) {
  get_ballot_files_urls(election_event_id: $electionEventId)
  sequent_backend_cast_vote { id tenant_id election_id election_event_id status }
}"""
CAST_QUERY = """mutation InsertCastVote($electionId: uuid!, $ballotId: String!, $content: String!) {
  insert_cast_vote(election_id: $electionId, ballot_id: $ballotId, content: $content) {
    id ballot_id election_id election_event_id tenant_id
  }
}"""


def read(path: Path) -> dict:
    """Read a private configuration or an aggregate artifact."""
    return json.loads(path.read_text())


def save(path: Path, value: dict) -> None:
    """Atomically publish complete metadata; partial files never count as ready."""
    temporary = path.with_suffix(path.suffix + ".partial")
    temporary.write_text(json.dumps(value, indent=2) + "\n")
    temporary.replace(path)


def digest(path: Path) -> str:
    """Hash an input incrementally so integrity checks stay bounded by buffer size."""
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def shard_bounds(config: dict, shard: int) -> tuple[int, int]:
    """Return a disjoint, half-open global voter range, including the short tail."""
    first = shard * config["shard_size"]
    if first < 0 or first >= config["count"]:
        raise ValueError("Shard outside census")
    return config["start"] + first, min(config["shard_size"], config["count"] - first)


def shard_count(config: dict) -> int:
    """Keep scheduler metadata proportional to chunks, never to voters."""
    return math.ceil(config["count"] / config["shard_size"])


def password(config: dict) -> str:
    """Read secrets from the environment, avoiding command-line process listings."""
    value = os.environ[config.get("password_env", "LOAD_PASSWORD")]
    if not value:
        raise ValueError("Synthetic password cannot be empty")
    return value


def portal_query(name: str, fallback: str) -> str:
    """Read the portal's GraphQL selection without compiling or launching a browser.

    The status adapter also has a standalone protocol definition for callers
    that install the runner without the portal source tree.
    """
    source = ROOT / "packages/voting-portal/src/queries" / f"{name}.ts"
    if not source.exists():
        return fallback
    match = re.search(r"gql`([\s\S]+?)`", source.read_text())
    if not match:
        raise ValueError(f"Cannot read portal GraphQL operation {name}")
    return match.group(1).strip()


def protocol(config: dict, *, cast: bool) -> dict:
    """Describe the API journey directly; a Chromium capture is optional evidence."""
    oidc = (
        config["keycloak_url"].rstrip("/")
        + "/realms/"
        + config["realm"]
        + "/protocol/openid-connect/"
    )
    steps = [
        dict(
            kind="auth",
            url=oidc + "auth",
            parameters=dict(
                client_id=config.get("client_id", "voting-portal"),
                redirect_uri=config["login_url"],
                response_type="code",
                response_mode="fragment",
                scope="openid",
                ui_locales=config.get("locale", "en"),
            ),
        ),
        dict(kind="login"),
        dict(kind="token", url=oidc + "token"),
        dict(
            kind="status",
            url=config["graphql_url"],
            payload=dict(
                operationName="GetVoterStatus",
                query=portal_query("GetVoterStatus", STATUS_QUERY),
                variables=dict(electionEventId=config["election_event_id"]),
            ),
        ),
    ]
    if config.get("mode", "vote") != "status":
        steps.extend(
            dict(kind="publication", binding=name)
            for name in ("event_url", "election_url", "summary_url", "style_url")
        )
        if cast:
            steps.append(dict(kind="cast"))
    return {"steps": [dict(step, offset_ms=0) for step in steps]}


def validate(config: dict) -> None:
    """Reject impossible allocations before creating a census or sending traffic."""
    for key in ("count", "shard_size", "vus"):
        if not isinstance(config[key], int) or config[key] < 1:
            raise ValueError(f"{key} must be a positive integer")
    if config["start"] < 0:
        raise ValueError("Use a nonnegative start")
    if config["engine"] not in ("k6", "chromium"):
        raise ValueError("engine must be k6 or chromium")
    if config["engine"] == "chromium" and config.get("mode") == "status":
        raise ValueError("Use k6 for the isolated status API workload")
    if config.get("mode", "vote") not in ("vote", "status"):
        raise ValueError("mode must be vote or status")
    from urllib.parse import urlsplit

    for name in ("graphql_url", "keycloak_url", "login_url"):
        url = urlsplit(config[name])
        if f"{url.scheme}://{url.netloc}" not in config["allowed_origins"]:
            raise ValueError(f"{name} is outside allowed_origins")
    if config["graphql_url"].split("?")[0] != config["graphql_url"]:
        raise ValueError("GraphQL URL must not contain credentials or query parameters")


def census(config: dict, output: Path) -> None:
    """Stream CSV batches using one PBKDF2 computation for the entire census.

    Do not add a password column: the importer would override the supplied hash.
    Every account gets an exact username lookup; shared attribute-only login is
    unsuitable because it would make authentication search the entire census.
    """
    output.mkdir(parents=True, exist_ok=False)
    started = time.monotonic()
    salt = os.urandom(16)
    iterations = config.get("hash_iterations", 27500)
    if iterations < 1:
        raise ValueError("hash_iterations must be positive")
    hashed = hashlib.pbkdf2_hmac(
        "sha256", password(config).encode(), salt, iterations, dklen=32
    )
    attributes = config.get("census_attributes", {})
    if set(attributes) & {
        "password",
        "username",
        "hashed_password",
        "password_salt",
        "num_of_iterations",
    }:
        raise ValueError("census_attributes cannot override credentials")
    fields = [
        "username",
        "area_name",
        "email",
        "email_verified",
        "authorized-election-ids",
        "hashed_password",
        "password_salt",
        "num_of_iterations",
        *attributes,
    ]
    if len(fields) != len(set(fields)):
        raise ValueError("Duplicate census attribute")
    for shard in range(shard_count(config)):
        start, count = shard_bounds(config, shard)
        with (output / f"{shard:06d}.csv").open("w", newline="") as stream:
            writer = csv.DictWriter(stream, fieldnames=fields)
            writer.writeheader()
            for index in range(start, start + count):
                username = config["username_prefix"] + str(index)
                writer.writerow(
                    dict(
                        attributes,
                        username=username,
                        area_name=config["area_name"],
                        email=f"{username}@example.invalid",
                        email_verified="true",
                        **{"authorized-election-ids": config["election_id"]},
                        hashed_password=base64.b64encode(hashed).decode(),
                        password_salt=base64.b64encode(salt).decode(),
                        num_of_iterations=iterations,
                    )
                )
    save(
        output / "census.json",
        dict(
            count=config["count"],
            shards=shard_count(config),
            password_hash_computations=1,
            elapsed_seconds=time.monotonic() - started,
            tenant_id=config["tenant_id"],
            election_event_id=config["election_event_id"],
        ),
    )


def import_census(config: dict, directory: Path, cli: Path, local: bool) -> None:
    """Import bounded CSVs through step-cli, with a durable per-batch checkpoint."""
    for shard in range(shard_count(config)):
        marker = directory / f"{shard:06d}.imported"
        if marker.exists():
            continue
        command = [
            str(cli.resolve()),
            "step",
            "import-voters",
            "--election-event-id",
            config["election_event_id"],
            "--file-path",
            str((directory / f"{shard:06d}.csv").resolve()),
        ]
        if local:
            command.append("--is-local")
        log_path = directory / f"{shard:06d}.import.log"
        with log_path.open("w") as log:
            subprocess.run(
                [str(cli.resolve()), "step", "refresh-token"],
                stdout=log,
                stderr=log,
                check=True,
            )
            subprocess.run(command, stdout=log, stderr=log, check=True)
        if "Voters imported successfully" not in log_path.read_text():
            raise RuntimeError(
                f"Import {shard} did not confirm success; inspect its private log"
            )
        marker.touch(exist_ok=False)


def k6_env(config: dict, config_path: Path) -> dict:
    """Only voter credentials enter the workload process; no administrator token."""
    environment = {
        key: value
        for key, value in os.environ.items()
        if key
        in {
            "PATH",
            "HOME",
            "TMPDIR",
            "LD_LIBRARY_PATH",
            "NIX_LD",
            "NIX_LD_LIBRARY_PATH",
            "SSL_CERT_FILE",
            "SSL_CERT_DIR",
            "NODE_EXTRA_CA_CERTS",
            "FONTCONFIG_FILE",
            "FONTCONFIG_PATH",
            "PLAYWRIGHT_BROWSERS_PATH",
            "CHROMIUM_EXECUTABLE_PATH",
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "NO_PROXY",
            "http_proxy",
            "https_proxy",
            "no_proxy",
        }
    }
    return dict(
        environment,
        LOAD_CONFIG=str(config_path.resolve()),
        LOAD_PASSWORD=password(config),
    )


def bootstrap(config: dict, output: Path) -> dict:
    """Authenticate one voter and obtain public encryption inputs without casting."""
    private = dict(config, profile=protocol(config, cast=False))
    save(output / "bootstrap.json", private)
    log_path = output / "bootstrap.log"
    with log_path.open("w") as log:
        subprocess.run(
            [
                config.get("runtime", {}).get("k6", "k6"),
                "run",
                "--log-format",
                "raw",
                str(HERE / "bootstrap.k6.js"),
            ],
            env=k6_env(config, output / "bootstrap.json"),
            stdout=log,
            stderr=log,
            check=True,
        )
    with log_path.open() as log:
        for line in log:
            if line.startswith("PUBLICATION "):
                return json.loads(line.removeprefix("PUBLICATION "))
    raise RuntimeError("No publication returned; inspect private bootstrap.log")


def prepare(
    config: dict, output: Path, native: Path | None, choices: Path | None, nodes: int
) -> None:
    """Pre-encrypt bounded shards in parallel; the ready marker follows every shard."""
    output.mkdir(parents=True, exist_ok=False)
    started = time.monotonic()
    result = bootstrap(config, output)
    config.update(
        style_id=result["file"]["id"],
        publication_version=result["file"]["version"],
        profile=protocol(config, cast=True),
        cast_query=portal_query("InsertCastVote", CAST_QUERY),
    )
    save(output / "config.json", config)
    if config["engine"] == "k6" and config.get("mode", "vote") != "status":
        if not native:
            raise ValueError("k6 vote preparation requires --native")
        wire = result["publications"]["style_url"]
        if "ballot_eml_prefix" in wire:
            eml = (
                wire["ballot_eml_prefix"]
                + result["publications"]["event_url"]["ballot_eml_presentation"]
                + wire["ballot_eml_suffix"]
            )
        else:
            eml = wire["ballot_eml"]
        style = json.loads(eml) if isinstance(eml, str) else eml
        save(output / "style.json", style)
        if choices is None:
            # A simple plurality fixture gets the same valid selection for every voter.
            # Other ballot rules should supply an explicit decoded choice document.
            decoded = []
            for contest in style["contests"]:
                if contest.get("is_acclaimed"):
                    continue
                selected = max(1, contest["min_votes"])
                if selected > contest["max_votes"] or selected > len(
                    contest["candidates"]
                ):
                    raise ValueError(
                        "Cannot choose a valid default ballot; supply --choices"
                    )
                decoded.append(
                    dict(
                        contest_id=contest["id"],
                        is_explicit_invalid=False,
                        is_decline_to_vote=False,
                        is_blank_ballot=False,
                        invalid_errors=[],
                        invalid_alerts=[],
                        choices=[
                            dict(
                                id=candidate["id"],
                                selected=0 if index < selected else -1,
                                write_in_text=None,
                            )
                            for index, candidate in enumerate(contest["candidates"])
                        ],
                    )
                )
            choices = output / "choices.json"
            choices.write_text(json.dumps(decoded))

        def encrypt_node(node: int) -> None:
            """One process handles one chunk; independent nodes share no voter state."""
            for shard in range(node, shard_count(config), nodes):
                _, count = shard_bounds(config, shard)
                destination = output / f"{shard:06d}.jsonl"
                with destination.with_suffix(".partial").open("w") as stream:
                    subprocess.run(
                        [
                            str(native.resolve()),
                            *(["load", "encrypt"] if config.get("native_cli") else []),
                            str(output / "style.json"),
                            str(choices.resolve()),
                            str(count),
                        ],
                        stdout=stream,
                        check=True,
                    )
                destination.with_suffix(".partial").replace(destination)
                destination.with_suffix(".sha256").write_text(digest(destination))

        with ThreadPoolExecutor(max_workers=nodes) as pool:
            list(pool.map(encrypt_node, range(nodes)))
    save(
        output / "ready.json",
        dict(
            shards=shard_count(config),
            count=config["count"],
            config_sha256=digest(output / "config.json"),
            elapsed_seconds=time.monotonic() - started,
        ),
    )


def worker(directory: Path, shard: int) -> None:
    """Attempt one shard once. A crash leaves a claim for explicit reconciliation."""
    config = read(directory / "config.json")
    shard_bounds(config, shard)
    if not (directory / "ready.json").exists():
        raise ValueError("Preparation is incomplete")
    if read(directory / "ready.json")["config_sha256"] != digest(
        directory / "config.json"
    ):
        raise ValueError("Prepared configuration changed; prepare a new run")
    if config["engine"] == "k6" and config.get("mode", "vote") == "vote":
        ballot_file = directory / f"{shard:06d}.jsonl"
        if digest(ballot_file) != ballot_file.with_suffix(".sha256").read_text():
            raise ValueError("Prepared ciphertext shard changed")
    out = directory / "results" / f"{shard:06d}"
    out.mkdir(parents=True, exist_ok=True)
    (out / "attempted").touch(exist_ok=False)
    env = k6_env(config, directory / "config.json")
    env.update(
        LOAD_SHARD=str(shard),
        LOAD_BALLOTS=str(directory / f"{shard:06d}.jsonl"),
        LOAD_SUMMARY=str(out / "summary.json"),
        LOAD_RESULTS=str(out / "samples.jsonl"),
        LOAD_ARTIFACTS=str(out / "browser"),
        LOAD_ACTION_TIMEOUT_MS=str(config.get("action_timeout_ms", 15000)),
    )
    runtime = config.get("runtime", {})
    if os.environ.get("STEP_LOAD_CONTAINER"):
        env.pop("CHROMIUM_EXECUTABLE_PATH", None)
        runtime = dict(
            runtime, node="node", k6="k6", playwright_dir="/runner", chromium=None
        )
    if runtime.get("chromium"):
        env["CHROMIUM_EXECUTABLE_PATH"] = runtime["chromium"]
    env["NODE_PATH"] = str(
        Path(runtime.get("playwright_dir", ROOT / "packages/voting-portal"))
        / "node_modules"
    )
    if config["engine"] == "k6":
        command = [
            runtime.get("k6", "k6"),
            "run",
            "--log-format",
            "raw",
            str(HERE / "scale.k6.js"),
        ]
    else:
        package = subprocess.check_output(
            [
                runtime.get("node", "node"),
                "-p",
                "require.resolve('@playwright/test/package.json')",
            ],
            cwd=runtime.get("playwright_dir", ROOT / "packages/voting-portal"),
            text=True,
        ).strip()
        env["NODE_PATH"] = str(Path(package).parents[2])
        command = [
            runtime.get("node", "node"),
            str(Path(package).parent / "cli.js"),
            "test",
            "--config",
            str(ROOT / "packages/voting-portal/playwright.scale.config.ts"),
        ]

    with (out / "worker.log").open("w") as log:
        result = subprocess.run(command, env=env, stdout=log, stderr=log)
    if config["engine"] == "k6":
        with (out / "worker.log").open() as log, (out / "samples.jsonl").open(
            "w"
        ) as samples:
            for line in log:
                if line.startswith("RESULT "):
                    samples.write(line.removeprefix("RESULT "))
    save(out / "exit.json", dict(code=result.returncode))
    if result.returncode:
        raise RuntimeError(f"Shard {shard} failed; inspect its private worker.log")


def node(directory: Path, index: int, nodes: int) -> None:
    """An indexed node owns disjoint shards and keeps at most one shard in memory."""
    if not 0 <= index < nodes:
        raise ValueError("node index must be in [0, nodes)")
    errors = []
    for shard in range(index, shard_count(read(directory / "config.json")), nodes):
        try:
            worker(directory, shard)
        except Exception as error:
            errors.append(str(error))
    if errors:
        raise RuntimeError("; ".join(errors))


def main() -> None:
    """Expose the same finite workload to local processes and indexed cluster jobs."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "command",
        choices=[
            "census",
            "import",
            "prepare",
            "run",
            "node",
            "report",
            "pods",
            "containers",
        ],
    )
    parser.add_argument("directory", type=Path)
    parser.add_argument("--config", type=Path)
    parser.add_argument(
        "--dsn-env",
        help="Optional coordinator-only PostgreSQL audit DSN environment variable",
    )
    parser.add_argument("--cli", type=Path)
    parser.add_argument("--is-local", action="store_true")
    parser.add_argument("--native", type=Path)
    parser.add_argument("--choices", type=Path)
    parser.add_argument("--nodes", type=int, default=1)
    parser.add_argument(
        "--index", type=int, default=int(os.environ.get("JOB_COMPLETION_INDEX", "0"))
    )
    parser.add_argument("--image")
    parser.add_argument("--pvc")
    parser.add_argument("--network", default="bridge")
    parser.add_argument("--mount-source", type=Path)
    parser.add_argument("--uid", type=int, default=os.getuid())
    parser.add_argument("--gid", type=int, default=os.getgid())
    parser.add_argument("--secret", default="voting-load-password")
    args = parser.parse_args()
    os.umask(0o077)
    args.directory = args.directory.resolve()
    if args.nodes < 1:
        parser.error("nodes must be positive")
    config = read(args.config or args.directory / "config.json")
    validate(config)
    if args.command == "census":
        census(config, args.directory)
    elif args.command == "import":
        if not args.cli:
            parser.error("import requires --cli")
        import_census(config, args.directory, args.cli, args.is_local)
    elif args.command == "prepare":
        prepare(config, args.directory, args.native, args.choices, args.nodes)
    elif args.command == "node":
        node(args.directory, args.index, args.nodes)
    elif args.command == "run":
        with ThreadPoolExecutor(max_workers=args.nodes) as pool:
            futures = [
                pool.submit(node, args.directory, index, args.nodes)
                for index in range(args.nodes)
            ]
            failures = [str(f.exception()) for f in futures if f.exception()]
        from aggregate import report

        report(args.directory, args.dsn_env)
        if failures:
            raise RuntimeError("; ".join(failures))
    elif args.command == "containers":
        if not args.image:
            parser.error("containers requires --image")

        def container(index: int) -> int:
            """Docker receives the secret through its environment, never argv values."""
            return subprocess.run(
                [
                    "docker",
                    "run",
                    "--rm",
                    "--user",
                    f"{os.getuid()}:{os.getgid()}",
                    "--network",
                    args.network,
                    "-e",
                    config.get("password_env", "LOAD_PASSWORD"),
                    "-v",
                    f"{args.mount_source or args.directory}:/load",
                    args.image,
                    "node",
                    "/load",
                    "--nodes",
                    str(args.nodes),
                    "--index",
                    str(index),
                ]
            ).returncode

        with ThreadPoolExecutor(max_workers=args.nodes) as pool:
            codes = list(pool.map(container, range(args.nodes)))
        if any(codes):
            raise RuntimeError(
                "Container workers failed; merge their results with report"
            )
    elif args.command == "report":
        from aggregate import report

        report(args.directory, args.dsn_env)
    else:
        if not args.image or not args.pvc:
            parser.error("pods requires --image and --pvc")
        # Mount the prepared run at /load; no per-voter ConfigMaps or API objects.
        job = {
            "apiVersion": "batch/v1",
            "kind": "Job",
            "metadata": {"generateName": "voting-load-"},
            "spec": {
                "completionMode": "Indexed",
                "completions": args.nodes,
                "parallelism": args.nodes,
                "backoffLimit": 0,
                "template": {
                    "spec": {
                        "restartPolicy": "Never",
                        "securityContext": {
                            "runAsUser": args.uid,
                            "runAsGroup": args.gid,
                            "fsGroup": args.gid,
                        },
                        "containers": [
                            {
                                "name": "worker",
                                "image": args.image,
                                "command": [
                                    "python3",
                                    "/runner/packages/voting-load/runner.py",
                                    "node",
                                    "/load",
                                    "--nodes",
                                    str(args.nodes),
                                ],
                                "env": [
                                    {
                                        "name": "JOB_COMPLETION_INDEX",
                                        "valueFrom": {
                                            "fieldRef": {
                                                "fieldPath": "metadata.annotations['batch.kubernetes.io/job-completion-index']"
                                            }
                                        },
                                    },
                                    {
                                        "name": config.get(
                                            "password_env", "LOAD_PASSWORD"
                                        ),
                                        "valueFrom": {
                                            "secretKeyRef": {
                                                "name": args.secret,
                                                "key": "password",
                                            }
                                        },
                                    },
                                ],
                                "resources": config.get(
                                    "resources",
                                    {
                                        "requests": {"cpu": "1", "memory": "1Gi"},
                                        "limits": {"cpu": "2", "memory": "4Gi"},
                                    },
                                ),
                                "volumeMounts": [
                                    {"name": "load", "mountPath": "/load"}
                                ],
                            }
                        ],
                        "volumes": [
                            {
                                "name": "load",
                                "persistentVolumeClaim": {"claimName": args.pvc},
                            }
                        ],
                    }
                },
            },
        }
        print(json.dumps(job, indent=2))


if __name__ == "__main__":
    main()
