# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Shared helpers for the load-test scripts in this directory.

Imported by setup_telephone_load_test.py, run_telephone_load_test.py and
run_online_load_test.py — not meant to be run directly. All three scripts
take no command-line arguments: every configurable knob lives in
telephone-load-test-inputs/layers.yaml, loaded here.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any

SCRIPTS_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPTS_DIR.parents[2]
CONFIG_PATH = SCRIPTS_DIR / "telephone-load-test-inputs" / "layers.yaml"

_ANSI_RE = re.compile(r"\x1b\[[0-9;]*[a-zA-Z]")


def log(message: str) -> None:
    print(f"==> {message}", file=sys.stderr, flush=True)


def die(message: str) -> None:
    print(f"Error: {message}", file=sys.stderr)
    sys.exit(1)


def load_config() -> dict[str, Any]:
    """Loads and validates layers.yaml, resolving relative paths against
    SCRIPTS_DIR (matching where the *.py scripts themselves live)."""
    import yaml  # local import: only needed once config loading actually runs

    if not CONFIG_PATH.is_file():
        die(f"no config file at {CONFIG_PATH}")
    with CONFIG_PATH.open() as f:
        config = yaml.safe_load(f) or {}
    if not isinstance(config, dict):
        die(f"{CONFIG_PATH} must be a YAML mapping")
    return config


def resolve_path(value: str) -> Path:
    """Resolves a YAML-supplied path relative to SCRIPTS_DIR, unless it is
    already absolute."""
    path = Path(value)
    return path if path.is_absolute() else (SCRIPTS_DIR / path).resolve()


def section(config: dict[str, Any], name: str) -> dict[str, Any]:
    value = config.get(name)
    if not isinstance(value, dict):
        die(f"{CONFIG_PATH} is missing the '{name}:' section")
    return value


def req_str(cfg: dict[str, Any], key: str, *, env: str | None = None) -> str:
    """A required string field: YAML value first, then $env, then a clear
    error naming both places the caller could have set it."""
    value = cfg.get(key)
    if value:
        return str(value)
    if env:
        env_value = os.environ.get(env)
        if env_value:
            return env_value
    where = f"layers.yaml's '{key}'" + (f" (or ${env})" if env else "")
    die(f"{where} is required but not set")
    raise AssertionError("unreachable")  # die() exits, but satisfies type checkers


def opt_str(cfg: dict[str, Any], key: str, *, env: str | None = None, default: str | None = None) -> str | None:
    value = cfg.get(key)
    if value:
        return str(value)
    if env:
        env_value = os.environ.get(env)
        if env_value:
            return env_value
    return default


# --- step-cli -----------------------------------------------------------------


def find_step_cli() -> str:
    from shutil import which

    binary = which("step-cli")
    if not binary:
        die(
            "step-cli not found on PATH. Build it: "
            "(cd packages/step-cli && cargo build --release -p step-cli)"
        )
    return binary  # type: ignore[return-value]


class StepCliError(RuntimeError):
    pass


def run_step(step_cli_bin: str, *args: str) -> str:
    """Runs `step-cli step <args>`. step-cli always exits 0 even on failure
    (commands eprintln "Error! ..." and return); detect failure by scanning
    the captured output instead of the exit code."""
    proc = subprocess.run(
        [step_cli_bin, "step", *args],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        env={**os.environ, "NO_COLOR": "1"},
    )
    out = _ANSI_RE.sub("", proc.stdout)
    print(out, file=sys.stderr)
    if re.search(r"^Error!", out, re.MULTILINE):
        raise StepCliError(f"step-cli step {' '.join(args)} failed")
    return out


def retry_step(step_cli_bin: str, attempts: int, delay: float, *args: str) -> None:
    """The trustee containers complete the key ceremony asynchronously
    (polling the bulletin board on their own schedule, running an actual DKG
    protocol round), so complete-key-ceremony can legitimately fail if called
    before a trustee has caught up to a just-started ceremony. Retry with
    backoff instead of treating the first failure as fatal."""
    import time

    for attempt in range(1, attempts + 1):
        try:
            run_step(step_cli_bin, *args)
            return
        except StepCliError:
            if attempt >= attempts:
                die(f"step-cli step {' '.join(args)} did not succeed after {attempts} attempts")
            log(f"Retrying in {delay}s (attempt {attempt + 1}/{attempts})...")
            time.sleep(delay)


_ID_RE = re.compile(r"ID:? +([A-Za-z0-9._-]+)")


def extract_id(output: str) -> str:
    """step-cli prints "Success! ... ID: <uuid>" (or, inconsistently,
    "ID <uuid>" with no colon) on the last line of a successful run; take the
    last ID-like token on the last such line."""
    matches = _ID_RE.findall(output)
    if not matches:
        die("could not parse an ID from step-cli output")
    return matches[-1]


# --- HTTP (stdlib only — no extra dependency for a couple of admin-API calls) --


def http_json(url: str, *, data: dict[str, str] | None = None, headers: dict[str, str] | None = None) -> Any:
    body = None
    req_headers = dict(headers or {})
    if data is not None:
        body = urllib.parse.urlencode(data).encode()
        req_headers.setdefault("Content-Type", "application/x-www-form-urlencoded")
    request = urllib.request.Request(url, data=body, headers=req_headers)
    with urllib.request.urlopen(request, timeout=10) as resp:  # noqa: S310
        return json.loads(resp.read())


def http_ok(url: str, *, timeout: float = 10) -> bool:
    try:
        with urllib.request.urlopen(url, timeout=timeout) as resp:  # noqa: S310
            return 200 <= resp.status < 400
    except (urllib.error.URLError, TimeoutError, ConnectionError):
        return False


def write_json(path: Path, data: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w") as f:
        json.dump(data, f, indent=2)
        f.write("\n")
