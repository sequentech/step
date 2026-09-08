# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Compile a verified Chromium journey into a session-independent k6 recipe."""
from datetime import datetime
import hashlib
import json
from urllib.parse import parse_qs, urlsplit, urlunsplit


def compile_profile(capture: dict, har: dict) -> dict:
    """Replace session-specific browser requests with validated protocol bindings."""
    if capture.get("engine") != "chromium" or not (
        capture.get("completed") and capture.get("persistence_verified")
    ):
        raise ValueError("A verified Chromium journey is required")
    files = capture.get("publication_files", [])
    if len(files) != 1:
        raise ValueError("Profile currently requires one eligible election per voter")
    bindings = {url: key for key, url in files[0]["urls"].items()}
    entries = sorted(har["log"]["entries"], key=lambda e: e["startedDateTime"])
    steps = []
    graphql = iter(
        sorted(
            (r for r in capture["requests"] if r.get("operation")),
            key=lambda r: r["id"],
        )
    )
    start = datetime.fromisoformat(entries[0]["startedDateTime"].replace("Z", "+00:00"))
    for entry in entries:
        request = entry["request"]
        url = urlsplit(request["url"])
        method = request["method"]
        if method == "OPTIONS":
            continue  # Browser CORS preflights are not application requests.
        if entry["response"]["status"] not in (200, 302):
            raise ValueError("Profile contains an unsuccessful request")
        step = dict(
            method=method,
            url=urlunsplit(url._replace(fragment="")),
            offset_ms=(
                datetime.fromisoformat(entry["startedDateTime"].replace("Z", "+00:00"))
                - start
            ).total_seconds()
            * 1000,
        )
        if url.path.endswith("/protocol/openid-connect/auth"):
            query = parse_qs(url.query)
            step.update(
                kind="auth",
                url=urlunsplit(url._replace(query="", fragment="")),
                parameters={
                    key: query[key][0]
                    for key in (
                        "client_id",
                        "redirect_uri",
                        "response_type",
                        "response_mode",
                        "scope",
                        "ui_locales",
                    )
                    if key in query
                },
            )
        elif "/login-actions/" in url.path:
            if method != "POST" or not url.path.endswith("/authenticate"):
                raise ValueError(
                    "Unsupported authentication flow; add an explicit adapter"
                )
            step.update(kind="login", url=None)
        elif url.path.endswith("/protocol/openid-connect/token"):
            step.update(kind="token")
        elif request["url"] in bindings:
            step.update(kind="publication", url=None, binding=bindings[request["url"]])
        elif method == "POST":
            observed = next(graphql, {})
            if observed.get("url") != request["url"]:
                raise ValueError("Unmatched GraphQL request")
            payload = observed.get("query_payload", {})
            operation = observed.get("operation")
            if operation == "GetVoterStatus" and payload.get(
                "query", ""
            ).lstrip().startswith("query "):
                step.update(kind="status", payload=payload)
            elif operation == "InsertCastVote":
                step.update(kind="cast")
            else:
                raise ValueError("Unrecognized POST in browser journey")
        elif method == "GET":
            if url.query and any(
                k.lower() in {"code", "state", "session_state", "iss"}
                for k in parse_qs(url.query)
            ):
                step["url"] = urlunsplit(url._replace(query="", fragment=""))
            step.update(kind="account" if url.path.endswith("/account") else "resource")
            if "/publication-" in url.path:
                raise ValueError("Unbound publication URL")
        else:
            raise ValueError("Unsupported browser method")
        steps.append(step)
    kinds = [step["kind"] for step in steps]
    required = ["auth", "login", "token", "status", "cast"]
    if any(kinds.count(kind) != 1 for kind in required):
        raise ValueError("Require one login, token, status and cast per journey")
    if [kinds.index(kind) for kind in required] != sorted(
        kinds.index(kind) for kind in required
    ):
        raise ValueError("Unexpected protocol dependency order")
    if {s.get("binding") for s in steps if s["kind"] == "publication"} != set(
        bindings.values()
    ):
        raise ValueError("Missing publication downloads")
    return dict(
        schema_version=1,
        browser="chromium",
        steps=steps,
        source_sha256=hashlib.sha256(
            json.dumps(har, sort_keys=True).encode()
        ).hexdigest(),
        source_elapsed_ms=capture["elapsed_ms"],
        scope={
            key: capture["casts"][0][key] for key in ("tenant_id", "election_event_id")
        },
        scheduling="ordered requests with recorded start offsets; no browser JavaScript",
    )
