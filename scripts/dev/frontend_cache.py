# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Content identities and output verification for CI frontend builds."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import subprocess
from pathlib import Path

from scripts.dev.affected.model import load_model

ROOT = Path(__file__).resolve().parents[2]
SHARED_PACKAGES = ("ui-core", "ui-essentials")
PORTALS = ("voting-portal", "admin-portal", "results-portal", "ballot-verifier")
SCHEMA = 1
MANIFEST_DIRECTORY = Path(".cache/feedback-builds")
ENVIRONMENT_KEYS = (
    "NODE_ENV",
    "BABEL_ENV",
    "BROWSERSLIST_ENV",
    "BROWSERSLIST",
    "NODE_OPTIONS",
)
RECIPE_FILES = {
    "scripts/dev/frontend_cache.py",
    "scripts/dev/affected.toml",
    ".github/workflows/frontend-ui-tests.yml",
    ".github/actions/setup-frontend/action.yml",
    ".github/actions/restore-frontend-build/action.yml",
}
ROOT_INPUTS = {
    "package.json",
    "yarn.lock",
    ".npmrc",
    ".yarnrc",
    ".node-version",
    ".nvmrc",
    "packages/package.json",
    "packages/yarn.lock",
    "packages/.npmrc",
    "packages/.yarnrc",
}


def run(root: Path, *arguments: str) -> str:
    return subprocess.check_output(arguments, cwd=root, text=True).strip()


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def fingerprint(records: dict[str, object]) -> str:
    return hashlib.sha256(
        json.dumps(records, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()


def input_paths(root: Path) -> list[str]:
    model = load_model(root)
    closure = set(SHARED_PACKAGES)
    pending = list(closure)
    while pending:
        for dependency in model.units[pending.pop()].depends - closure:
            closure.add(dependency)
            pending.append(dependency)
    prefixes = tuple(
        model.units[unit].path + "/"
        for unit in sorted(closure)
        if model.units[unit].path
    )
    tracked = run(
        root, "git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"
    )
    return sorted(
        {
            path
            for path in tracked.split("\0")
            if path
            and (
                path.startswith(prefixes)
                or path in ROOT_INPUTS
                or path in RECIPE_FILES
                or (path.startswith("packages/") and path.endswith("/package.json"))
                or (
                    path.startswith("packages/")
                    and "/rust/" in path
                    and path.endswith(".tgz")
                )
            )
        }
    )


def identity(root: Path) -> dict[str, object]:
    return {
        "schema": SCHEMA,
        "platform": [platform.system(), platform.machine(), platform.libc_ver()],
        "node": run(root, "node", "--version"),
        "yarn": run(root, "yarn", "--version"),
        "environment": {key: os.environ.get(key, "") for key in ENVIRONMENT_KEYS},
        "recipe": "webpack production: ui-core then ui-essentials",
        "inputs": {path: digest(root / path) for path in input_paths(root)},
    }


def output_files(root: Path, package: str) -> dict[str, str]:
    packages = SHARED_PACKAGES if package == "shared-ui" else (package,)
    records = {}
    for name in packages:
        directory = root / "packages" / name / "dist"
        required = "index.js" if package == "shared-ui" else "index.html"
        if not (directory / required).is_file():
            raise ValueError(f"missing build output {directory / required}")
        for path in sorted(directory.rglob("*")):
            if path.is_symlink():
                raise ValueError(f"unexpected symlink in build output: {path}")
            if path.is_file():
                records[path.relative_to(root).as_posix()] = digest(path)
    return records


def stamp(root: Path, package: str, key: str) -> None:
    manifest = root / MANIFEST_DIRECTORY / f"{package}.json"
    manifest.parent.mkdir(parents=True, exist_ok=True)
    manifest.write_text(
        json.dumps(
            {
                "schema": SCHEMA,
                "key": key,
                "outputs": output_files(root, package),
            },
            sort_keys=True,
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )


def verify(root: Path, package: str, key: str) -> bool:
    manifest = root / MANIFEST_DIRECTORY / f"{package}.json"
    try:
        recorded = json.loads(manifest.read_text(encoding="utf-8"))
        return (
            isinstance(recorded, dict)
            and recorded.get("schema") == SCHEMA
            and recorded.get("key") == key
            and recorded.get("outputs") == output_files(root, package)
        )
    except (OSError, ValueError):
        return False


def artifact_name(package: str, run_id: str) -> str:
    # A partial rerun consumes successful producers from an earlier attempt.
    return f"frontend-{package}-{run_id}"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("key", "stamp", "verify", "artifact"))
    parser.add_argument(
        "--package", choices=("shared-ui", *PORTALS), default="shared-ui"
    )
    parser.add_argument("--key")
    args = parser.parse_args()
    if args.command == "artifact":
        name = artifact_name(args.package, os.environ["GITHUB_RUN_ID"])
        with Path(os.environ["GITHUB_OUTPUT"]).open("a", encoding="utf-8") as stream:
            stream.write(f"name={name}\n")
        print(name)
        return 0
    if args.command == "key":
        records = identity(ROOT)
        key = fingerprint(records)
        if destination := os.environ.get("GITHUB_OUTPUT"):
            with Path(destination).open("a", encoding="utf-8") as stream:
                stream.write(f"key={key}\n")
        print(key)
        return 0
    key = args.key or (
        fingerprint(identity(ROOT))
        if args.package == "shared-ui"
        else run(ROOT, "git", "rev-parse", "HEAD")
    )
    if args.command == "stamp":
        stamp(ROOT, args.package, key)
        return 0
    valid = verify(ROOT, args.package, key)
    status = (
        "current outputs verified" if valid else "missing, stale or invalid outputs"
    )
    print(f"{args.package}: {status}")
    return 0 if valid else 1


if __name__ == "__main__":
    raise SystemExit(main())
