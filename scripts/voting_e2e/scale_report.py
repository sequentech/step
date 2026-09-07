# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Merge raw samples on disk and produce a portable, self-contained load report."""
from __future__ import annotations

from collections import Counter
import html
import json
import math
import os
from pathlib import Path
import sqlite3

from scale import read, save, shard_bounds, shard_count


def percentile(db: sqlite3.Connection, column: str, fraction: float) -> float | None:
    """Calculate global interpolated percentiles; never average worker percentiles."""
    if column not in {"cast_ms", "status_ms", "journey_ms"}:
        raise ValueError("Unknown latency column")
    count = db.execute(f"SELECT count({column}) FROM samples").fetchone()[0]
    if not count:
        return None
    position = (count - 1) * fraction
    rows = db.execute(
        f"SELECT {column} FROM samples WHERE {column} IS NOT NULL ORDER BY {column} LIMIT 2 OFFSET ?",
        (math.floor(position),),
    ).fetchall()
    low = rows[0][0]
    return low + (rows[-1][0] - low) * (position - math.floor(position))


def verify_receipts(db: sqlite3.Connection, config: dict, dsn_env: str) -> int:
    """Audit accepted IDs in bounded read-only database batches after load timing."""
    import psycopg

    cursor = db.execute("SELECT receipt FROM samples WHERE receipt IS NOT NULL")
    verified = 0
    with psycopg.connect(
        os.environ[dsn_env], autocommit=True, connect_timeout=15
    ) as observer:
        observer.execute("SET default_transaction_read_only=on")
        observer.execute("SET statement_timeout=30000")
        while rows := cursor.fetchmany(1000):
            verified += observer.execute(
                "SELECT count(*) FROM sequent_backend.cast_vote WHERE id=ANY(%s::uuid[]) "
                "AND tenant_id=%s::uuid AND election_event_id=%s::uuid",
                (
                    [row[0] for row in rows],
                    config["tenant_id"],
                    config["election_event_id"],
                ),
            ).fetchone()[0]
    return verified


def report(directory: Path, dsn_env: str | None = None) -> dict:
    """Validate complete ownership and receipts, then render aggregate-only artifacts.

    SQLite bounds merge memory independently of voter count. Raw receipts remain
    private; the HTML contains only counts, latencies and the protocol definition.
    API acceptance is reported separately from independent database verification.
    """
    config = read(directory / "config.json")
    database = directory / "measurements.sqlite"
    database.unlink(missing_ok=True)
    traffic: Counter[str] = Counter()
    errors = []
    with sqlite3.connect(database) as db:
        db.execute("PRAGMA cache_size=-16384")
        db.execute("PRAGMA temp_store=FILE")
        db.execute(
            "CREATE TABLE samples (voter INTEGER PRIMARY KEY, passed INTEGER, start REAL, end REAL, cast_ms REAL, status_ms REAL, journey_ms REAL, receipt TEXT UNIQUE)"
        )
        for shard in range(shard_count(config)):
            result = directory / "results" / f"{shard:06d}"
            first, count = shard_bounds(config, shard)
            if (
                not (result / "exit.json").exists()
                or read(result / "exit.json")["code"] != 0
            ):
                errors.append(f"Shard {shard} did not finish successfully")
            samples = result / "samples.jsonl"
            if not samples.exists():
                continue
            with samples.open() as stream:
                for line in stream:
                    row = json.loads(line)
                    if not first <= row["index"] < first + count:
                        raise ValueError("Worker submitted a voter outside its shard")
                    db.execute(
                        "INSERT INTO samples VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                        (
                            row["index"],
                            row["passed"],
                            row["start"],
                            row["end"],
                            row.get("cast_ms"),
                            row.get("status_ms"),
                            row["end"] - row["start"],
                            row.get("receipt"),
                        ),
                    )
            db.commit()
            summary_path = result / "summary.json"
            if summary_path.exists():
                for name, metric in read(summary_path).get("metrics", {}).items():
                    if name.startswith("http_reqs{name:"):
                        traffic[name[len("http_reqs{name:") : -1]] += metric["values"][
                            "count"
                        ]
            browser_traffic = result / "traffic.json"
            if browser_traffic.exists():
                traffic.update(read(browser_traffic))
        for column in ("cast_ms", "status_ms", "journey_ms"):
            db.execute(f"CREATE INDEX {column}_order ON samples({column})")
        completed, passed, receipts, start, end = db.execute(
            "SELECT count(*), coalesce(sum(passed),0), count(receipt), min(start), max(end) FROM samples"
        ).fetchone()
        elapsed = (end - start) / 1000 if completed else 0
        cps = receipts / elapsed if elapsed else 0
        metrics = {
            column: {
                "p50": percentile(db, column, 0.5),
                "p99": percentile(db, column, 0.99),
            }
            for column in ("cast_ms", "status_ms", "journey_ms")
        }
        if completed != config["count"] or passed != config["count"]:
            errors.append("Missing or failed voter journeys")
        if config.get("mode", "vote") == "vote" and receipts != config["count"]:
            errors.append("Missing unique API receipts")
        for metric, limits in config.get("goals", {}).items():
            if metric == "min_casts_per_second":
                if cps < limits:
                    errors.append(f"Throughput {cps:.2f} below goal {limits}")
            else:
                for quantile, limit in limits.items():
                    actual = metrics[metric][quantile]
                    if actual is None or actual > limit:
                        errors.append(
                            f"{metric} {quantile} exceeds {limit} ms or is missing"
                        )
        verification = "API receipts; no independent database audit"
        if dsn_env:
            verified = verify_receipts(db, config, dsn_env)
            verification = (
                f"{verified}/{receipts} API receipts matched PostgreSQL in batches"
            )
            if verified != receipts:
                errors.append("Database receipt audit is incomplete")
        result = dict(
            engine=config["engine"],
            mode=config.get("mode", "vote"),
            planned=config["count"],
            completed=completed,
            passed=passed,
            accepted_casts=receipts,
            elapsed_seconds=elapsed,
            casts_per_second=cps,
            latency=metrics,
            traffic=dict(traffic),
            errors=errors,
            persistence_verification=verification,
        )
        save(directory / "results.json", result)
        render(directory, db, config, result)
    if errors:
        raise RuntimeError(
            "Load goals failed; inspect report.html: " + "; ".join(errors[:5])
        )
    return result


def render(directory: Path, db: sqlite3.Connection, config: dict, result: dict) -> None:
    """Draw bounded aggregate curves and embed them in a standalone HTML document."""
    import matplotlib

    matplotlib.use("Agg")
    matplotlib.rcParams["svg.hashsalt"] = "sequent-voting-scale"
    matplotlib.rcParams["svg.fonttype"] = "none"
    import matplotlib.pyplot as plt
    import io

    figure, axes = plt.subplots(1, 2, figsize=(10, 3.5), constrained_layout=True)
    for column, label in (("status_ms", "Voter status"), ("cast_ms", "Cast")):
        points = [(percentile(db, column, p / 100), p) for p in range(0, 101, 2)]
        points = [(x, y) for x, y in points if x is not None]
        if points:
            axes[0].plot(*zip(*points), label=label)
    axes[0].set(
        xlabel="Response time (ms)", ylabel="Percentile", title="Response latency"
    )
    if axes[0].lines:
        axes[0].legend()
    if result["completed"]:
        start, end = db.execute("SELECT min(start), max(end) FROM samples").fetchone()
        width = max(1000, (end - start) / 60)
        bins = db.execute(
            "SELECT cast((end-?)/? AS INTEGER), count(*) FROM samples WHERE receipt IS NOT NULL GROUP BY 1",
            (start, width),
        ).fetchall()
        axes[1].bar(
            [b * width / 1000 for b, _ in bins],
            [n * 1000 / width for _, n in bins],
            width=width / 1000 * 0.9,
        )
    axes[1].set(
        xlabel="Seconds from first journey",
        ylabel="Accepted casts/s",
        title="Delivered throughput",
    )
    chart = io.StringIO()
    figure.savefig(chart, format="svg", metadata={"Date": None})
    plt.close(figure)
    svg = chart.getvalue().split("<svg", 1)[1]
    svg = "\n".join(line.rstrip() for line in ("<svg" + svg).splitlines()) + "\n"
    (directory / "performance.svg").write_text(svg)
    def latency(value: float | None) -> str:
        """Keep absent operations distinct from a measured zero-millisecond response."""
        return "—" if value is None else f"{value:,.2f}"

    rows = "".join(
        f"<tr><td>{html.escape(name)}</td><td>{latency(values['p50'])}</td><td>{latency(values['p99'])}</td></tr>"
        for name, values in result["latency"].items()
    )
    requests = "".join(
        f"<tr><td>{html.escape(name)}</td><td>{count}</td></tr>"
        for name, count in sorted(result["traffic"].items())
    )
    verdict = "Passed" if not result["errors"] else "Failed"
    document = f"""<!doctype html><html lang="en"><meta charset="utf-8"><title>Voting load results</title>
<style>body{{font:16px system-ui;max-width:1050px;margin:3rem auto;padding:0 1rem;color:#172b46;background:#f9fbff}}h1{{font-size:2.5rem}}svg{{width:100%;height:auto}}table{{border-collapse:collapse;width:100%;background:white}}td,th{{text-align:left;padding:.65rem;border-bottom:1px solid #dce4ef}}pre{{white-space:pre-wrap;background:#edf2fa;padding:1rem}}.cards{{font-size:1.3rem;padding:1.5rem;background:#e2efff;border-radius:12px}}</style>
<h1>Voting load results · {verdict}</h1><p>{result['engine']} · {result['mode']} · {config['vus']} concurrent voters per node · shard limit {config['shard_size']}</p>
<div class="cards">{result['accepted_casts']:,} accepted casts &nbsp; {result['casts_per_second']:.2f} casts/s &nbsp; {result['passed']:,}/{result['planned']:,} successful journeys</div>
<p>Percentiles combine individual samples across every worker. Throughput includes startup and the final in-flight journey. {result['persistence_verification']}.</p>
{svg}<h2>Latency (milliseconds)</h2><table><tr><th>Operation</th><th>p50</th><th>p99</th></tr>{rows}</table>
<h2>Observed application requests</h2><table><tr><th>Endpoint / operation</th><th>Requests</th></tr>{requests}</table>
<p>k6 measures the authenticated HTTP protocol; Chromium also executes UI JavaScript, encryption and resource downloads. These workloads have different client costs.</p>
<h2>Goals and failures</h2><pre>{html.escape(json.dumps(dict(goals=config.get('goals', {}), errors=result['errors']), indent=2))}</pre>
<h2>GraphQL operations</h2><pre>{html.escape(config['profile']['steps'][3]['payload']['query'])}</pre><pre>{html.escape(config['cast_query'])}</pre></html>"""
    (directory / "report.html").write_text(document)
    (directory / "performance.md").write_text(
        f"Voting load: **{verdict}**. {result['passed']:,}/{result['planned']:,} journeys; {result['casts_per_second']:.2f} accepted casts/s.\n\n![Performance](performance.svg)\n"
    )


def publish_docs(directory: Path) -> None:
    """Refresh the current measured example from a successful aggregate artifact."""
    from scale import ROOT

    result = read(directory / "results.json")
    if result["errors"]:
        raise ValueError("Only a passing run can replace the documented example")
    docs = ROOT / "docs/docusaurus"
    image = docs / "static/img/voting-scale-performance.svg"
    image.write_text((directory / "performance.svg").read_text())
    # REUSE-IgnoreStart
    image.with_suffix(".svg.license").write_text(
        "SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>\n"
        "SPDX-License-Identifier: AGPL-3.0-only\n"
    )
    # REUSE-IgnoreEnd
    page = docs / "docs/07-developers/05-voting-portal/voter-status-performance.md"
    text = page.read_text()
    start, end = "<!-- scale-performance:start -->", "<!-- scale-performance:end -->"
    before, rest = text.split(start)
    _, after = rest.split(end)
    status, cast = result["latency"]["status_ms"], result["latency"]["cast_ms"]
    summary = (
        f"\nMeasured example: **{result['engine']}**, {result['passed']:,}/{result['planned']:,} "
        f"successful journeys, **{result['casts_per_second']:.2f} accepted casts/s**. "
        f"Voter-status p50/p99: **{status['p50']:.2f}/{status['p99']:.2f} ms**.\n\n"
    )
    if cast["p50"] is not None:
        summary += f"Cast p50/p99: **{cast['p50']:.2f}/{cast['p99']:.2f} ms**.\n\n"
    summary += "This is a measured workload, not a deployment capacity guarantee.\n\n![Voting performance](/img/voting-scale-performance.svg)\n"
    page.write_text(before + start + summary + end + after)
