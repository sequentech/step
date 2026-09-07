# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Shared helpers for the load-test scripts in this directory.

Imported by setup_telephone_load_test.py, run_telephone_load_test.py and
run_online_load_test.py — not meant to be run directly. All three scripts
take no command-line arguments: every configurable knob lives in
telephone-load-test-inputs/config/layers.yaml, loaded here.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

SCRIPTS_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPTS_DIR.parents[2]
INPUTS_DIR = SCRIPTS_DIR / "telephone-load-test-inputs"
# config/ is gitignored (holds real per-server credentials) — copy
# layers.yaml.example there before first use.
CONFIG_PATH = INPUTS_DIR / "config" / "layers.yaml"
CONFIG_EXAMPLE_PATH = INPUTS_DIR / "layers.yaml.example"

_ANSI_RE = re.compile(r"\x1b\[[0-9;]*[a-zA-Z]")


def log(message: str) -> None:
    print(f"==> {message}", file=sys.stderr, flush=True)


def die(message: str) -> None:
    print(f"Error: {message}", file=sys.stderr)
    sys.exit(1)


# layers.yaml.example marks every required credential with this literal
# placeholder (never a real secret) instead of `null`, so a diff shows at a
# glance which fields need a value. Treated identically to `null` wherever a
# config value is read, so req_str()'s env-var fallback still kicks in —
# copying the example unedited into config/layers.yaml keeps working inside
# the devcontainer, where those env vars are already set.
MASKED_PLACEHOLDER = "****"


def _unmask(value: Any) -> Any:
    if isinstance(value, dict):
        return {k: _unmask(v) for k, v in value.items()}
    if isinstance(value, list):
        return [_unmask(v) for v in value]
    return None if value == MASKED_PLACEHOLDER else value


def load_config() -> dict[str, Any]:
    """Loads and validates config/layers.yaml, resolving relative paths
    against SCRIPTS_DIR (matching where the *.py scripts themselves live)."""
    import yaml  # local import: only needed once config loading actually runs

    if not CONFIG_PATH.is_file():
        die(
            f"no config file at {CONFIG_PATH}. Copy the tracked template and fill in your own "
            f"credentials:\n    mkdir -p {CONFIG_PATH.parent}\n    cp {CONFIG_EXAMPLE_PATH} {CONFIG_PATH}"
        )
    with CONFIG_PATH.open() as f:
        config = yaml.safe_load(f) or {}
    if not isinstance(config, dict):
        die(f"{CONFIG_PATH} must be a YAML mapping")
    return _unmask(config)


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


def retry_step(step_cli_bin: str, attempts: int, delay: float, *args: str) -> str:
    """The trustee containers complete the key ceremony asynchronously
    (polling the bulletin board on their own schedule, running an actual DKG
    protocol round), so complete-key-ceremony can legitimately fail if called
    before a trustee has caught up to a just-started ceremony. Retry with
    backoff instead of treating the first failure as fatal. Also useful right
    after authenticating against a freshly created tenant, whose realm JWKS
    may not have propagated to every backend instance yet."""
    import time

    for attempt in range(1, attempts + 1):
        try:
            return run_step(step_cli_bin, *args)
        except StepCliError:
            if attempt >= attempts:
                die(f"step-cli step {' '.join(args)} did not succeed after {attempts} attempts")
            log(f"Retrying in {delay}s (attempt {attempt + 1}/{attempts})...")
            time.sleep(delay)
    raise AssertionError("unreachable")  # die() exits, but satisfies type checkers


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

# urllib's default "Python-urllib/x.y" User-Agent trips Cloudflare's bot-fight
# mode (error 1010) on Cloudflare-fronted deployments (e.g. the *.sequent.vote
# test servers) even for legitimate authenticated calls — send a normal
# browser-like one instead.
_USER_AGENT = "Mozilla/5.0 (X11; Linux x86_64) step-cli-load-test"


def http_json(url: str, *, data: dict[str, str] | None = None, headers: dict[str, str] | None = None) -> Any:
    body = None
    req_headers = {"User-Agent": _USER_AGENT, **(headers or {})}
    if data is not None:
        body = urllib.parse.urlencode(data).encode()
        req_headers.setdefault("Content-Type", "application/x-www-form-urlencoded")
    request = urllib.request.Request(url, data=body, headers=req_headers)
    with urllib.request.urlopen(request, timeout=10) as resp:  # noqa: S310
        return json.loads(resp.read())


def http_ok(url: str, *, timeout: float = 10) -> bool:
    try:
        request = urllib.request.Request(url, headers={"User-Agent": _USER_AGENT})
        with urllib.request.urlopen(request, timeout=timeout) as resp:  # noqa: S310
            return 200 <= resp.status < 400
    except (urllib.error.URLError, TimeoutError, ConnectionError):
        return False


def lookup_client_secret(keycloak_url: str, admin_user: str, admin_password: str, tenant_id: str, client_id: str) -> str:
    """Looks up client_id's secret in tenant_id's realm via Keycloak's
    master-realm admin API. Only needed for a tenant whose client secret
    isn't already known — e.g. a freshly auto-created tenant, whose clients
    get a randomly regenerated secret unless the server has a fixed one
    configured for that specific client id."""
    log(f"Looking up {client_id}'s client secret from Keycloak (master realm admin API)")
    token_resp = http_json(
        f"{keycloak_url}/realms/master/protocol/openid-connect/token",
        data={
            "grant_type": "password",
            "client_id": "admin-cli",
            "username": admin_user,
            "password": admin_password,
        },
    )
    master_token = token_resp.get("access_token")
    if not master_token:
        die(
            f"could not obtain a master-realm admin token to look up {client_id}'s secret "
            "(check setup.keycloak_admin_user/keycloak_admin_password)"
        )
    clients = http_json(
        f"{keycloak_url}/admin/realms/tenant-{tenant_id}/clients?clientId={urllib.parse.quote(client_id)}",
        headers={"Authorization": f"Bearer {master_token}"},
    )
    secret = clients[0].get("secret") if clients else None
    if not secret:
        die(f"could not look up {client_id}'s secret in tenant-{tenant_id}")
    return secret  # type: ignore[return-value]


TIMESTAMP_FORMAT = "%Y-%m-%dT%H:%M:%SZ"


def format_timestamp(value: datetime) -> str:
    return value.astimezone(timezone.utc).strftime(TIMESTAMP_FORMAT)


def parse_timestamp(value: str) -> datetime:
    return datetime.strptime(value, TIMESTAMP_FORMAT).replace(tzinfo=timezone.utc)


# --- Synchronised / ramped start (distributed runs) ----------------------------


def parse_start_at(value: str) -> datetime:
    """ISO 8601 (e.g. 2026-09-07T14:30:00Z or 2026-09-07T14:30:00+00:00); a
    timestamp with no zone is taken as UTC, so every client — whatever its
    local zone — resolves the same value to the same instant."""
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        die(f"start_at / $LOAD_TEST_START_AT must be an ISO 8601 timestamp, got {value!r}")
        raise AssertionError("unreachable")
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed.astimezone(timezone.utc)


def resolve_start(cfg: dict[str, Any]) -> tuple[datetime | None, int]:
    """Reads the section's start_at / start_delay, each overridable by
    $LOAD_TEST_START_AT / $LOAD_TEST_START_DELAY (the env vars win, so one
    shared layers.yaml can be copied to every load client unchanged and the
    per-client values injected by whatever fans the run out). Returns the
    shared base instant (None = "when this client's preflight finishes")
    and this client's delay in seconds on top of it: with the same base
    everywhere, a different delay per client ramps the load up in steps
    until every client is running, instead of all of them hitting the
    server at once."""
    start_at_raw = os.environ.get("LOAD_TEST_START_AT") or cfg.get("start_at")
    start_at = parse_start_at(str(start_at_raw)) if start_at_raw else None
    delay_raw = os.environ.get("LOAD_TEST_START_DELAY")
    if delay_raw is None or delay_raw == "":
        delay_raw = cfg.get("start_delay")
    try:
        delay = int(delay_raw or 0)
    except ValueError:
        die(f"start_delay / $LOAD_TEST_START_DELAY must be a whole number of seconds, got {delay_raw!r}")
        raise AssertionError("unreachable")
    if delay < 0:
        die("start_delay / $LOAD_TEST_START_DELAY must not be negative")
    return start_at, delay


def wait_for_start(start_at: datetime | None, delay: int) -> datetime:
    """Holds until start_at + delay (or now + delay when no start_at is
    shared), logging the remaining wait so an operator watching several
    clients can see their staggered targets. A target already in the past
    starts immediately — useful when re-running one client after a failure
    without touching the shared timestamp. Returns the resolved target."""
    from datetime import timedelta

    base = start_at or datetime.now(timezone.utc)
    target = base + timedelta(seconds=delay)
    if start_at is None and delay == 0:
        return target
    remaining = (target - datetime.now(timezone.utc)).total_seconds()
    if remaining <= 0:
        log(f"start time {format_timestamp(target)} is in the past ({round(-remaining)}s ago) — starting immediately")
        return target
    log(f"Holding until {format_timestamp(target)} ({round(remaining)}s; start_at {format_timestamp(base)} + {delay}s delay) before generating any load")
    while remaining > 0:
        time.sleep(min(remaining, 30))
        remaining = (target - datetime.now(timezone.utc)).total_seconds()
        if remaining > 30:
            log(f"    {round(remaining)}s to go")
    log("Go")
    return target


def run_throughput(parts: list[dict[str, Any]], cast: int) -> dict[str, Any]:
    """Run-wide throughput over several sequential (or overlapping) parts —
    the per-tenant summaries of one run, or the per-client summaries of a
    distributed one — each carrying started_at/finished_at. Every tenant
    lives in the same deployment and the same database, so the figure that
    matters is what the whole run cast between the earliest start and the
    latest finish, not a per-tenant rate."""
    started_at = min(parse_timestamp(p["started_at"]) for p in parts)
    finished_at = max(parse_timestamp(p["finished_at"]) for p in parts)
    elapsed_secs = max(1, round((finished_at - started_at).total_seconds()))
    return {
        "started_at": format_timestamp(started_at),
        "finished_at": format_timestamp(finished_at),
        "elapsed_secs": elapsed_secs,
        "cast_per_second": round(cast / elapsed_secs, 3),
    }


def write_json(path: Path, data: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w") as f:
        json.dump(data, f, indent=2)
        f.write("\n")
