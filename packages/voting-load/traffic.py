# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Inventory observed endpoints without headers, query values, or voter IDs."""
from collections import Counter, defaultdict
import re
from urllib.parse import urlsplit
from measurements import percentile


def inventory(requests: list[dict]) -> list[dict]:
    """Group observed traffic by normalized endpoint and summarize latency and bytes."""
    groups = defaultdict(list)
    for request in requests:
        url = urlsplit(request["url"])
        path = re.sub(
            r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}",
            "{id}",
            url.path,
            flags=re.I,
        )
        groups[
            url.hostname,
            url.port,
            request["method"],
            path,
            request.get("operation"),
            request.get("phase", "journey"),
        ].append(request)
    result = []
    for (host, port, method, path, operation, phase), rows in groups.items():
        durations = [
            r["timing"]["responseEnd"]
            for r in rows
            if r.get("timing", {}).get("responseEnd", -1) >= 0
        ]
        sizes = [r.get("sizes", {}).get("responseBodySize") for r in rows]
        result.append(
            dict(
                host=host,
                port=port,
                method=method,
                path=path,
                operation=operation,
                phase=phase,
                count=len(rows),
                statuses=dict(Counter(str(r.get("status", "failed")) for r in rows)),
                p50_ms=percentile(durations, 50) if durations else None,
                p95_ms=percentile(durations, 95) if durations else None,
                response_bytes=sum(s for s in sizes if s is not None and s >= 0),
                unknown_size_count=sum(s is None or s < 0 for s in sizes),
            )
        )
    return result


def validate_s3_flow(requests: list[dict]) -> list[str]:
    """Reject unexpected GraphQL reads or incomplete publication downloads."""
    operations = Counter(r.get("operation") for r in requests if r.get("operation"))
    errors = []
    if operations["GetVoterStatus"] != 1:
        errors.append("Expected exactly one GetVoterStatus operation")
    if not operations["InsertCastVote"]:
        errors.append("Missing InsertCastVote operation")
    if set(operations) - {"GetVoterStatus", "InsertCastVote"}:
        errors.append("Unexpected GraphQL operation in S3 voter flow")
    objects = [
        r
        for r in requests
        if "/publication-" in urlsplit(r["url"]).path and r["method"] == "GET"
    ]
    if len(objects) < 4 or any(r.get("status") != 200 for r in objects):
        errors.append("Missing successful private publication object downloads")
    names = {
        urlsplit(r["url"]).path.rsplit("/", 1)[-1].split("-")[0].split(".")[0]
        for r in objects
    }
    if not {"event", "election", "summary", "style"}.issubset(names):
        errors.append("Missing event, election, summary or selected style object")
    return errors
