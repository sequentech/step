# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""The checkout a command acts on, as described by its ``.devcontainer/.env``."""

from __future__ import annotations

import subprocess
from dataclasses import dataclass
from pathlib import Path

REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
ENV_FILE = Path(".devcontainer/.env")
INITIALIZE_COMMAND = Path(".devcontainer/scripts/initialize-command.sh")
CACHE_VOLUME_KEYS = (
    "DEVCONTAINER_NIX_VOLUME",
    "DEVCONTAINER_CACHE_VOLUME",
    "DEVCONTAINER_CARGO_VOLUME",
)
# Files a container runtime creates; see running_in_container.
CONTAINER_MARKERS = (Path("/.dockerenv"), Path("/run/.containerenv"))


class CheckoutError(RuntimeError):
    """The checkout lacks the devcontainer settings a command needs."""


def running_in_container() -> bool:
    return any(marker.exists() for marker in CONTAINER_MARKERS)


def parse_dotenv(text: str) -> dict[str, str]:
    """``KEY=value`` lines as Compose reads them, without interpolation."""
    values: dict[str, str] = {}
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("export "):
            line = line[len("export ") :].lstrip()
        key, separator, value = line.partition("=")
        key = key.strip()
        if not separator or not key:
            continue
        value = value.strip()
        if len(value) >= 2 and value[0] == value[-1] and value[0] in "'\"":
            value = value[1:-1]
        else:
            comment = value.find(" #")
            if comment >= 0:
                value = value[:comment].rstrip()
        values[key] = value
    return values


@dataclass(frozen=True)
class Checkout:
    # The repository root as this process sees it.
    root: Path
    env: dict[str, str]

    @property
    def project(self) -> str:
        project = self.env.get("COMPOSE_PROJECT_NAME", "")
        if not project:
            raise CheckoutError(f"{self.root / ENV_FILE} sets no COMPOSE_PROJECT_NAME")
        return project

    @property
    def name_prefix(self) -> str:
        return self.env.get("DEVCONTAINER_NAME_PREFIX", "")

    @property
    def host_root(self) -> Path:
        """The checkout's path on the Docker host."""
        host = self.env.get("LOCAL_WORKSPACE_FOLDER", "")
        return Path(host) if host else self.root

    @property
    def container_root(self) -> str:
        """The checkout's path inside its devcontainer."""
        return (
            self.env.get("DEVCONTAINER_WORKSPACE_FOLDER")
            or f"/workspaces/{self.root.name}"
        )

    @property
    def compose_dir(self) -> Path:
        """``.devcontainer`` at its host path when reachable from here.

        Compose resolves relative bind mount sources against this directory, and
        the Docker daemon reads them on the host. The devcontainer also mounts
        the checkout at its host path, so this holds inside it as well.
        """
        host = self.host_root / ".devcontainer"
        try:
            if (host / ".env").samefile(self.root / ENV_FILE):
                return host
        except OSError:
            pass
        return self.root / ".devcontainer"

    @property
    def binds_resolve_on_host(self) -> bool:
        return self.compose_dir == self.host_root / ".devcontainer"

    def cache_volumes(self) -> list[str]:
        missing = [key for key in CACHE_VOLUME_KEYS if not self.env.get(key)]
        if missing:
            raise CheckoutError(
                f"{self.root / ENV_FILE} lacks {', '.join(missing)}; "
                f"run {INITIALIZE_COMMAND}"
            )
        return [self.env[key] for key in CACHE_VOLUME_KEYS]


def load_checkout(root: Path = REPOSITORY_ROOT) -> Checkout:
    """Reads ``.devcontainer/.env``, writing it first on the host if missing.

    Inside a container the host path of the checkout is unknown, so a missing
    file is an error there instead.
    """
    path = root / ENV_FILE
    if not path.is_file():
        if running_in_container():
            raise CheckoutError(
                f"{path} is missing; run {INITIALIZE_COMMAND} on the host"
            )
        completed = subprocess.run(
            [str(root / INITIALIZE_COMMAND)], cwd=root, check=False
        )
        if completed.returncode != 0 or not path.is_file():
            raise CheckoutError(f"{INITIALIZE_COMMAND} failed")
    return Checkout(root, parse_dotenv(path.read_text(encoding="utf-8")))
