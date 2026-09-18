# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
import json
import os
import subprocess
import time
import hashlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source_digest():
    """Include local patches/new sources; a matching commit alone is insufficient."""
    digest = hashlib.sha256()
    for command in (["git", "rev-parse", "HEAD"], ["git", "diff", "HEAD", "--binary"]):
        digest.update(execute(command, capture=True).encode())
    names = execute(["git", "ls-files", "--others", "--exclude-standard", "-z"], capture=True)
    for name in sorted(filter(None, names.split("\0"))):
        file = ROOT / name
        if file.is_file():
            digest.update(name.encode())
            digest.update(file.read_bytes())
    return digest.hexdigest()


def execute(args, *, cwd=ROOT, timeout=1800, capture=False, env=None, log=None):
    started = time.monotonic()
    options = {"cwd": cwd, "timeout": timeout, "text": True, "check": True,
               "env": dict(os.environ, **(env or {}))}
    if log:
        Path(log).parent.mkdir(parents=True, exist_ok=True)
        with open(log, "a", encoding="utf8") as output:
            result = subprocess.run(args, stdout=output, stderr=subprocess.STDOUT, **options)
    else:
        result = subprocess.run(args, capture_output=capture, **options)
    return result.stdout if capture else time.monotonic() - started


def docker(*args, **kwargs):
    prefix = ["docker"]
    if os.environ.get("E2E_DOCKER_SUDO") == "1":
        prefix = ["sudo", "-n", "docker"]
    return execute([*prefix, *map(str, args)], **kwargs)


def save(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2) + "\n")
    temporary.chmod(0o600)
    temporary.replace(path)
