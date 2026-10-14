# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Fingerprint compatible compiler units without treating a cache hit as a build."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import subprocess
from collections.abc import Mapping
from pathlib import Path

ENV_PREFIXES = (
    "RUST",
    "CARGO_PROFILE_",
    "CARGO_TARGET_",
    "CC",
    "CXX",
    "CFLAGS",
    "CXXFLAGS",
    "OPENSSL_",
    "PKG_CONFIG_",
)
ENV_NAMES = ("CARGO_ENCODED_RUSTFLAGS",)


def digest(value: object) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()


def cache_identity(
    root: Path,
    *,
    name: str,
    lockfile: str,
    target_dir: str,
    compiler: str,
    system: str,
    architecture: str,
    profile: str,
    features: str,
    targets: str,
    environment: Mapping[str, str],
    epoch: str = "v1",
) -> dict[str, str]:
    root = root.resolve()
    if not re.fullmatch(r"[a-zA-Z0-9_-]+", name):
        raise ValueError("Cache name must contain only letters, numbers, '-' and '_'")
    lock = (root / lockfile).resolve()
    lock.relative_to(root)
    # A missing lockfile must not silently become a permanent, empty cache key.
    lock_hash = hashlib.sha256(lock.read_bytes()).hexdigest()
    target = (root / target_dir).resolve()
    target.relative_to(root)
    if target == root or target == lock.parent:
        raise ValueError("Cargo target must be an output directory inside the checkout")
    tracked = (
        subprocess.run(
            ["git", "ls-files", "-z"], cwd=root, check=True, capture_output=True
        )
        .stdout.decode()
        .split("\0")
    )
    configuration = {}
    for relative in sorted(filter(None, tracked)):
        path = Path(relative)
        if path.name in {"Cargo.toml", "rust-toolchain", "rust-toolchain.toml"} or (
            path.parent.name == ".cargo" and path.name in {"config", "config.toml"}
        ):
            configuration[relative] = hashlib.sha256(
                (root / path).read_bytes()
            ).hexdigest()
    flags = {
        key: value
        for key, value in environment.items()
        if (key.startswith(ENV_PREFIXES) or key in ENV_NAMES)
        and key not in {"RUSTUP_HOME", "CARGO_TARGET_DIR", "RUST_BACKTRACE"}
    }
    compatibility = digest(
        {
            "compiler": compiler,
            "system": system,
            "architecture": architecture,
            "profile": profile,
            "features": features,
            "targets": targets,
            "configuration": configuration,
            "flags": flags,
            "epoch": epoch,
        }
    )
    prefix = f"rust-compiler-v1-{system}-{architecture}-{name}-{compatibility}-"
    return {
        "key": prefix + lock_hash,
        "restore-prefix": prefix,
        "registry-key": f"cargo-registry-v2-{system}-{architecture}-{lock_hash}",
        "registry-prefix": f"cargo-registry-v2-{system}-{architecture}-",
        "target-dir": str(target),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--name", required=True)
    parser.add_argument("--lockfile", required=True)
    parser.add_argument("--target-dir", required=True)
    parser.add_argument("--profile", default="dev")
    parser.add_argument("--features", default="default")
    parser.add_argument("--targets", default="host")
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--epoch", default="v1")
    parser.add_argument("--github-output", type=Path)
    args = parser.parse_args()
    result = cache_identity(
        args.root,
        name=args.name,
        lockfile=args.lockfile,
        target_dir=args.target_dir,
        compiler=subprocess.check_output(["rustc", "-Vv"], text=True),
        system=os.environ.get("RUNNER_OS", platform.system()),
        architecture=os.environ.get("RUNNER_ARCH", platform.machine()),
        profile=args.profile,
        features=args.features,
        targets=args.targets,
        environment=os.environ,
        epoch=args.epoch,
    )
    if args.github_output:
        with args.github_output.open("a") as output:
            for key, value in result.items():
                output.write(f"{key}={value}\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
