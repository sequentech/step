# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Prepare, fetch and select a source-free, compatible devenv image."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import re
import shutil
import subprocess
from pathlib import Path

BASE_IMAGE = "ghcr.io/cachix/devenv/devcontainer:v1.10"
IMAGE_REPOSITORY = "ghcr.io/sequentech/step-devenv"
IMAGE_LABEL = "io.sequent.devenv-key"
INPUTS = (
    "devenv.nix",
    "devenv.lock",
    "devenv.yaml",
    ".devcontainer/devcontainer.json",
    ".devcontainer/devcontainer-lock.json",
    ".devcontainer/prebuild/Dockerfile",
    ".devcontainer/prebuild/warm-env.sh",
    "scripts/dev/prebuild.py",
)


def read_jsonc(path: Path) -> dict:
    # Match whole strings first, so URL slashes and escaped quotes remain intact.
    value = re.sub(
        r'("(?:\\.|[^"\\])*")|//[^\n]*|/\*[\s\S]*?\*/',
        lambda match: match.group(1) or "",
        path.read_text(),
    )
    value = re.sub(
        r'("(?:\\.|[^"\\])*")|,(?=\s*[}\]])',
        lambda match: match.group(1) or "",
        value,
    )
    return json.loads(value)


def fingerprint(root: Path) -> str:
    hashes = {
        path: hashlib.sha256((root / path).read_bytes()).hexdigest() for path in INPUTS
    }
    return hashlib.sha256(json.dumps(hashes, sort_keys=True).encode()).hexdigest()[:32]


def image_reference(key: str, repository: str = IMAGE_REPOSITORY) -> str:
    if not re.fullmatch(r"[a-z0-9][a-z0-9./_-]*", repository):
        raise ValueError(
            "Image repository must be a lowercase registry/repository path"
        )
    return f"{repository}:env-{key}"


def prepare_context(root: Path, destination: Path, key: str) -> None:
    if destination.exists() and any(destination.iterdir()):
        raise ValueError(f"Build context must be empty: {destination}")
    config_dir = destination / ".devcontainer"
    config_dir.mkdir(parents=True, exist_ok=True)
    for name in ("devenv.nix", "devenv.lock", "devenv.yaml"):
        shutil.copyfile(root / name, destination / name)
    for name in ("Dockerfile", "warm-env.sh"):
        shutil.copyfile(root / ".devcontainer/prebuild" / name, config_dir / name)
    shutil.copyfile(
        root / ".devcontainer/devcontainer-lock.json",
        config_dir / "devcontainer-lock.json",
    )
    config = {
        "name": "Step development tools",
        "build": {
            "dockerfile": "Dockerfile",
            "context": "..",
            "args": {"STEP_ENV_FINGERPRINT": key},
        },
        "features": read_jsonc(root / ".devcontainer/devcontainer.json")["features"],
        "remoteUser": "vscode",
    }
    (config_dir / "devcontainer.json").write_text(json.dumps(config, indent=2) + "\n")


def compatible_image(image: str, key: str) -> bool:
    try:
        result = subprocess.run(
            ["docker", "image", "inspect", image],
            capture_output=True,
            text=True,
            check=False,
            timeout=15,
        )
    except (OSError, subprocess.TimeoutExpired):
        return False
    if result.returncode:
        return False
    try:
        entry = json.loads(result.stdout)[0]
        labels = entry["Config"]["Labels"]
        if not isinstance(labels, dict):
            return False
        architecture = {
            "aarch64": "arm64",
            "arm64": "arm64",
            "x86_64": "amd64",
            "AMD64": "amd64",
        }.get(platform.machine())
        return (
            labels.get(IMAGE_LABEL) == key
            and entry["Architecture"] == architecture
            and entry["Os"] == "linux"
        )
    except (KeyError, IndexError, TypeError, json.JSONDecodeError):
        return False


def selection(image: str, key: str, available: bool) -> dict[str, str]:
    if available:
        return {
            "DEVCONTAINER_IMAGE": image,
            "DEVCONTAINER_NIX_VOLUME": f"step-devcontainer-nix-env-{key}",
        }
    return {
        "DEVCONTAINER_IMAGE": BASE_IMAGE,
        "DEVCONTAINER_NIX_VOLUME": "step-devcontainer-nix-v1.10",
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "command", choices=["status", "pull", "resolve", "context", "fingerprint"]
    )
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[2]
    )
    parser.add_argument("--repository", default=IMAGE_REPOSITORY)
    parser.add_argument("--destination", type=Path)
    parser.add_argument("--env-file", type=Path)
    parser.add_argument("--github-output", type=Path)
    args = parser.parse_args()
    root = args.root.resolve()
    key = fingerprint(root)
    image = image_reference(key, args.repository)
    if args.command == "context":
        if not args.destination:
            parser.error("context requires --destination")
        prepare_context(root, args.destination, key)
    elif args.command in {"pull", "resolve", "status"}:
        docker = shutil.which("docker")
        if docker and args.command == "pull":
            subprocess.run([docker, "pull", image], check=False)
        available = bool(docker) and compatible_image(image, key)
        selected = selection(image, key, available)
        if args.command == "resolve":
            if not args.env_file:
                parser.error("resolve requires --env-file")
            with args.env_file.open("a") as output:
                output.write(
                    "\n# Compatible toolchain image selected by step-dev prebuild\n"
                )
                for name, value in selected.items():
                    output.write(f"{name}={value}\n")
        print(json.dumps({"key": key, "available": available, **selected}, indent=2))
    else:
        print(key)
    if args.github_output:
        with args.github_output.open("a") as output:
            output.write(f"key={key}\nimage={image}\n")


if __name__ == "__main__":
    main()
