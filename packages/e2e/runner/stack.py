# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Derive an isolated runtime from the maintained development service definitions."""
import copy
import json
import os
import re
import time
from pathlib import Path
from .process import ROOT, docker, save

TOOLS = "step-e2e-tools:local"


def host_path(path):
    """Docker outside Docker needs daemon-host paths, not container paths."""
    relative = Path(path).relative_to(ROOT)
    return str(Path(os.environ.get("LOCAL_WORKSPACE_FOLDER", str(ROOT))) / relative)

SERVICES = {"postgres", "postgres-keycloak", "postgres-b4", "rabbitmq", "minio", "minio-proxy",
            "configure-minio", "graphql-engine", "keycloak", "harvest", "windmill",
            "beat", "b4", "trustee1", "trustee2", "immudb"}


def project_name(run_id):
    if not re.fullmatch(r"[a-z0-9][a-z0-9-]{0,55}", run_id):
        raise ValueError("run ID must contain 1–56 lowercase letters, digits or hyphens")
    return "step-e2e-" + run_id


class Stack:
    def __init__(self, run_id, coverage="none"):
        self.name = project_name(run_id)
        self.run_id = run_id
        self.directory = ROOT / ".e2e/runs" / run_id
        self.directory.mkdir(parents=True, exist_ok=True, mode=0o700)
        self.file = self.directory / "compose.json"
        self.coverage = coverage

    def compose(self, *args, **kwargs):
        return docker("compose", "--project-name", self.name, "-f", self.file,
                      *args, **kwargs)

    def generate(self):
        # Compose include.env_file is resolved even with --env-file. Give it a
        # deterministic fixture, without replacing a developer's local settings.
        local_env = ROOT / ".devcontainer/.env"
        if not local_env.exists():
            local_env.write_bytes((ROOT / ".devcontainer/.env.development").read_bytes())
            local_env.chmod(0o600)
        raw = docker("compose", "--env-file", ROOT / ".devcontainer/.env.development",
                     "--profile", "full", "-f", ROOT / ".devcontainer/docker-compose.yml",
                     "config", "--format", "json", capture=True)
        original = json.loads(raw)
        selected = {key: copy.deepcopy(value) for key, value in original["services"].items()
                    if key in SERVICES}
        for key, service in selected.items():
            for field in ("container_name", "profiles", "ports", "depends_on", "restart",
                          "volumes_from", "stdin_open", "healthcheck", "networks"):
                service.pop(field, None)
            service["restart"] = "no"
            service["cpus"] = 1
            service["mem_limit"] = "1g"
            service.setdefault("environment", {})
            service["environment"].update({"AWS_S3_PUBLIC_URI": "http://minio-proxy:9002",
                "KEYCLOAK_PUBLIC_URL": "http://keycloak:8090",
                "VOTING_PORTAL_URL": "http://portals:3000",
                "BALLOT_VERIFIER_URL": "http://portals:3001",
                "AWS_S3_JWKS_CACHE_POLICY": "max-age=1",
                "AWS_EC2_METADATA_DISABLED": "true", "RAYON_NUM_THREADS": "2"})
            for volume in service.get("volumes", []):
                if isinstance(volume, dict) and volume.get("type") == "bind":
                    source = Path(volume["source"])
                    if source.is_relative_to(ROOT): volume["source"] = host_path(source)
        # The SQL schema's initial migrations need the same extension-enabled Postgres.
        for name in ("postgres", "postgres-keycloak", "postgres-b4"):
            selected[name]["mem_limit"] = "768m"
            selected[name]["command"] = ["postgres", "-c", "wal_level=logical", "-c", "shared_preload_libraries=pgaudit"]
        selected["postgres"]["image"] = selected["postgres-keycloak"]["image"] = "step-e2e-postgres:local"
        selected["postgres-b4"]["image"] = "step-e2e-postgres-b4:local"
        selected["graphql-engine"]["entrypoint"] = ["/bin/graphql-engine"]
        selected["graphql-engine"]["command"] = ["serve"]
        selected["keycloak"]["image"] = "step-e2e-keycloak:local"
        selected["keycloak"].pop("build", None)
        selected["keycloak"]["mem_limit"] = "1536m"
        selected["keycloak"]["environment"]["JAVA_OPTS_KC_HEAP"] = "-Xms128m -Xmx768m"
        selected["keycloak"]["environment"]["KC_HOSTNAME"] = "http://keycloak:8090"
        selected["configure-minio"]["volumes"] = [
            f"{host_path(ROOT / '.devcontainer/keycloak/import')}:/realm-configs:ro",
            f"{host_path(self.directory)}:/e2e:ro"]
        selected["minio"]["image"] = "quay.io/minio/minio:RELEASE.2025-04-22T22-12-26Z@sha256:a1ea29fa28355559ef137d71fc570e508a214ec84ff8083e39bc5428980b015e"
        selected["minio"]["environment"]["MINIO_BROWSER"] = "off"
        selected["immudb"] = {"image": "codenotary/immudb:1.9.6", "mem_limit": "512m",
                              "cpus": 0.5, "volumes": ["immudb_data:/var/lib/immudb"]}
        runtime = {"harvest": ["harvest"], "windmill": ["windmill", "consume", "-q",
                   "beat", "short_queue", "tally_queue", "reports_queue", "communication_queue",
                   "import_export_queue", "electoral_log_batch_queue", "electoral_log_beat_queue",
                   "--prefetch-count", "1", "--worker-threads", "2"], "beat": ["beat"],
                   "b4": ["b4", "--host", "postgres-b4", "--port", "5432", "--username", "postgres",
                          "--password", "postgrespassword", "--database", "b4", "--bind", "0.0.0.0:50051"]}
        for name, command in runtime.items():
            service = selected[name]
            service.pop("build", None)
            service["image"] = TOOLS
            service["entrypoint"] = [f"/workspaces/step/.e2e/bin/{'coverage' if self.coverage != 'none' else 'normal'}/{command[0]}"]
            service["command"] = command[1:]
            service["working_dir"] = "/workspaces/step/packages"
            service["volumes"] = [f"{host_path(ROOT)}:/workspaces/step"]
            service["init"] = True
            service["stop_grace_period"] = "45s"
            service["environment"].update({"PATH": "/usr/local/cargo/bin:/usr/local/bin:/usr/bin:/bin",
                "RUST_BACKTRACE": "1", "ROCKET_WORKERS": "2"})
            if self.coverage != "none":
                directory = self.directory / "coverage/rust/e2e" / name
                directory.mkdir(parents=True, exist_ok=True)
                service["environment"]["LLVM_PROFILE_FILE"] = f"/workspaces/step/{directory.relative_to(ROOT)}/%m-%p%c.profraw"
        selected["windmill"]["mem_limit"] = "2g"
        selected["pushgateway"] = {"image": "prom/pushgateway:v1.11.2", "cpus": 0.25,
                                   "mem_limit": "128m"}
        for i in (1, 2):
            key = f"trustee{i}"
            config = f"/workspaces/step/.devcontainer/trustees-data/{key}/{key}.toml"
            selected[key] = {"image": TOOLS, "init": True, "cpus": 0.5, "mem_limit": "512m",
                "entrypoint": ["/workspaces/step/.e2e/bin/normal/trustee"],
                "command": ["--b4-url", "http://b4:50051", "--trustee-config", config],
                "working_dir": f"/tmp/{key}", "volumes": [f"{host_path(ROOT)}:/workspaces/step:ro"],
                "environment": {"TRUSTEE_NAME": key, "RAYON_NUM_THREADS": "1"}}
        selected["runner"] = {"image": TOOLS, "init": True, "cpus": 2, "mem_limit": "8g",
            "shm_size": "1g", "working_dir": "/workspaces/step", "command": ["sleep", "infinity"],
            "volumes": [f"{host_path(ROOT)}:/workspaces/step", "cargo-cache:/usr/local/cargo/registry"],
            "environment": {"E2E_RUN_ID": self.run_id, "E2E_COVERAGE": self.coverage,
                "PYTHONPATH": "/workspaces/step/packages", "E2E_GREP": "", "E2E_LOCAL_STACK": "1",
                "GIT_CONFIG_COUNT": "1", "GIT_CONFIG_KEY_0": "safe.directory", "GIT_CONFIG_VALUE_0": "/workspaces/step",
                "E2E_ARTIFACTS": f"/workspaces/step/{self.directory.relative_to(ROOT)}",
                "STEP_CLI_CONFIG_DIR": f"/workspaces/step/{self.directory.relative_to(ROOT)}/private/cli",
                "CARGO_BUILD_JOBS": "2", "CARGO_PROFILE_DEV_DEBUG": "0", "CARGO_INCREMENTAL": "0"}}
        selected["portals"] = {"image": TOOLS, "init": True, "cpus": 1, "mem_limit": "512m",
            "command": ["python3", "packages/e2e/runner/serve.py"],
            "working_dir": "/workspaces/step", "volumes": [f"{host_path(ROOT)}:/workspaces/step:ro"],
            "environment": {"E2E_COVERAGE": self.coverage}}
        if self.coverage != "none":
            cli_profiles = self.directory / "coverage/rust/e2e/step-cli"
            cli_profiles.mkdir(parents=True, exist_ok=True)
            selected["runner"]["environment"]["LLVM_PROFILE_FILE"] = f"/workspaces/step/{cli_profiles.relative_to(ROOT)}/%m-%p%c.profraw"
        # Compose's normalized output contains explicit old project volume names.
        volumes = {key: {} for key in original.get("volumes", {})}
        docker("volume", "create", "step-e2e-cargo-cache", capture=True)
        volumes["cargo-cache"] = {"name": "step-e2e-cargo-cache", "external": True}
        save(self.file, {"services": selected, "volumes": volumes})
        return selected

    def exec(self, *args, **kwargs):
        return self.compose("exec", "-T", "runner", *args, **kwargs)

    def up(self, *services):
        self.compose("up", "-d", "--no-deps", *services, timeout=1200,
                     log=self.directory / "startup.log")

    def down(self):
        if self.file.exists():
            self.compose("down", "--volumes", "--remove-orphans", timeout=180,
                         log=self.directory / "cleanup.log")

    def reclaim_artifacts(self):
        # Tool containers run as root; reports must remain usable by the checkout
        # owner, including when the host entry point writes the final summary.
        owner = ROOT.stat()
        self.compose("run", "--rm", "--no-deps", "--entrypoint", "/bin/chown", "runner",
                     "-R", f"{owner.st_uid}:{owner.st_gid}",
                     f"/workspaces/step/{self.directory.relative_to(ROOT)}",
                     timeout=120, log=self.directory / "private/ownership.log")
