# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Summarize observed HTTP and PostgreSQL work without counting audit copies twice."""

from collections import Counter
import re


def summarize_sql(records: list[dict], clients: dict[str, str]) -> list[dict]:
    """Separate submitted statements, transaction commands and nested execution plans."""
    counts = Counter()
    for record in records:
        message = record.get("message", "")
        service = clients.get(
            record.get("remote_host", ""),
            record.get("application_name") or "unattributed",
        )
        match = re.match(r"(?:statement:|execute [^:]+:)\s*(.*)", message, re.DOTALL)
        if match:
            sql = match.group(1).lstrip()
            # Rust query files start with explanatory SQL comments. Classify the
            # statement that follows; comments are not an additional read.
            sql = re.sub(
                r"^(?:(?:--[^\n]*(?:\n|$)|/\*.*?\*/)\s*)+", "", sql, flags=re.DOTALL
            )
            verb = sql.split(None, 1)[0].upper() if sql else "EMPTY"
            kind = (
                "transaction"
                if verb
                in {"BEGIN", "COMMIT", "ROLLBACK", "START", "SAVEPOINT", "RELEASE"}
                else "submitted SQL"
            )
            if verb == "EMPTY":
                kind = "empty protocol check"
            counts[service, kind, verb] += 1
        elif "Query Text:" in message and record.get("context"):
            # Top-level auto_explain plans and pgaudit messages describe work
            # already counted above. Only nested plans form this separate count.
            counts[service, "nested SQL plan", "PLAN"] += 1
    return [
        {"service": service, "kind": kind, "verb": verb, "count": count}
        for (service, kind, verb), count in sorted(counts.items())
    ]


def percentile(values: list[float], percentage: float) -> float:
    """Interpolate a sample percentile; callers must disclose the small sample size."""
    ordered = sorted(values)
    position = (len(ordered) - 1) * percentage / 100
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    return ordered[lower] + (ordered[upper] - ordered[lower]) * (position - lower)


def journey_metrics(capture: dict) -> dict[str, float]:
    """Use recorded phase boundaries and separate cast HTTP time from total journey time."""
    phases = capture.get("phases", {})
    metrics = {"Full journey": float(capture["elapsed_ms"])}
    for label, start, end in [
        ("Portal to login form", "navigation_started", "login_form_ready"),
        (
            "Authentication and initial data",
            "credentials_submitted",
            "ballot_list_ready",
        ),
        (
            "Ballot selection through confirmation",
            "ballot_list_ready",
            "confirmation_ready",
        ),
    ]:
        if start in phases and end in phases:
            metrics[label] = phases[end] - phases[start]
    casts = [
        request
        for request in capture["requests"]
        if request.get("operation") == "InsertCastVote"
    ]
    durations = [
        request["timing"]["responseEnd"]
        for request in casts
        if request.get("timing", {}).get("responseEnd", -1) >= 0
    ]
    if durations:
        metrics["Cast API HTTP time"] = sum(durations)
    return metrics
