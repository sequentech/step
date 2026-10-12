# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Extract a resource recipe without captured headers, bodies or query values from a successful Playwright HAR.

The recipe describes optional load, not authentication or a reusable cast payload.
Query values and GraphQL variables must be rebound to the new voter's session.
"""

import argparse
import json
import re
from datetime import datetime
from pathlib import Path
from urllib.parse import parse_qsl, urlsplit


RESOURCE_TYPES = {
    "document",
    "stylesheet",
    "script",
    "image",
    "font",
    "media",
    "wasm",
    "fetch",
}


def protocol_steps(entries: list[dict]) -> list[dict]:
    """Extract a non-executable protocol inventory; never retain authentication values."""
    steps = []
    for entry in entries:
        request = entry["request"]
        url = urlsplit(request["url"])
        body = request.get("postData", {})
        try:
            operation = json.loads(body.get("text", "{}")).get("operationName")
        except (ValueError, AttributeError):
            operation = None
        if not operation and not any(
            part in url.path
            for part in ("/protocol/openid-connect/", "/login-actions/", "/account")
        ):
            continue
        form_names = []
        if (
            body.get("mimeType", "").split(";")[0]
            == "application/x-www-form-urlencoded"
        ):
            form_names = sorted(
                {
                    key
                    for key, _ in parse_qsl(
                        body.get("text", ""), keep_blank_values=True
                    )
                }
            )
            form_names = sorted(
                set(form_names) | {param["name"] for param in body.get("params", [])}
            )
        path = re.sub(r"[0-9a-f]{8}-[0-9a-f-]{27,}", "{id}", url.path)
        steps.append(
            {
                "method": request["method"],
                "path_template": path,
                "query_keys": sorted(
                    {key for key, _ in parse_qsl(url.query, keep_blank_values=True)}
                ),
                "form_field_names": form_names,
                "operation": operation,
                "status": entry["response"]["status"],
                "requires_fresh_session": True,
                "requires_fresh_ballot": operation == "InsertCastVote",
            }
        )
    return steps


def extract(har: dict) -> dict:
    """Preserve resource ordering/pacing while replacing origins and query values."""
    entries = har["log"]["entries"]
    resources = []
    origins = {}
    start = min(
        (
            datetime.fromisoformat(e["startedDateTime"].replace("Z", "+00:00"))
            for e in entries
        ),
        default=None,
    )
    for entry in entries:
        request = entry["request"]
        url = urlsplit(request["url"])
        resource_type = entry.get("_resourceType")
        if not resource_type:
            # Playwright's HAR omits Chromium's nonstandard _resourceType field.
            # MIME inference keeps extraction independent of a particular engine.
            mime = (
                entry["response"].get("content", {}).get("mimeType", "").split(";")[0]
            )
            if mime in ("text/html", "application/xhtml+xml"):
                resource_type = "document"
            elif mime == "text/css":
                resource_type = "stylesheet"
            elif "javascript" in mime:
                resource_type = "script"
            elif mime == "application/wasm":
                resource_type = "wasm"
            elif mime.startswith("image/"):
                resource_type = "image"
            elif mime.startswith("font/"):
                resource_type = "font"
            else:
                resource_type = "fetch"
        operation = None
        try:
            operation = json.loads(request.get("postData", {}).get("text", "{}")).get(
                "operationName"
            )
        except (ValueError, AttributeError):
            pass
        # Authentication, casts and other mutations belong to the protocol driver.
        # Only named GraphQL queries can be optional data fetches; never replay a POST blindly.
        graphql = False
        try:
            query = (
                json.loads(request.get("postData", {}).get("text", "{}"))
                .get("query", "")
                .lstrip()
            )
            graphql = bool(operation and query.startswith("query "))
        except (ValueError, AttributeError):
            pass
        if (
            not (request["method"] == "GET" and resource_type in RESOURCE_TYPES)
            and not graphql
        ):
            continue
        origin = f"{url.scheme}://{url.hostname}" + (f":{url.port}" if url.port else "")
        alias = origins.setdefault(origin, f"origin_{len(origins) + 1}")
        timestamp = datetime.fromisoformat(
            entry["startedDateTime"].replace("Z", "+00:00")
        )
        resources.append(
            {
                "origin": alias,
                "path": url.path,
                "method": request["method"],
                "query_keys": [
                    key for key, _ in parse_qsl(url.query, keep_blank_values=True)
                ],
                "kind": "graphql_query" if graphql else resource_type,
                "operation": operation,
                "offset_ms": (timestamp - start).total_seconds() * 1000,
                "duration_ms": entry["time"],
                "status": entry["response"]["status"],
                "response_body_bytes": (
                    max(entry["response"].get("bodySize", -1), 0)
                    if entry["response"].get("bodySize", -1) >= 0
                    else None
                ),
                "decoded_body_bytes": entry["response"].get("content", {}).get("size"),
                "requires_session_binding": bool(url.query or graphql),
            }
        )
    return {
        "schema_version": 2,
        "protocol_inventory": protocol_steps(entries),
        "origins": {alias: origin for origin, alias in origins.items()},
        "resources": resources,
        "authentication_included": False,
        "review_required": "Bind session query values, GraphQL variables and dynamic path IDs before replay.",
    }


def main() -> None:
    """Generate a local profile that can be refreshed after Playwright flow changes."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("har", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(extract(json.loads(args.har.read_text())), indent=2) + "\n"
    )


if __name__ == "__main__":
    main()
