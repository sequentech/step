# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Generate private reports from browser captures and prerequisite checks."""

from collections import Counter
import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import re
from urllib.parse import urlsplit

from measurements import journey_metrics, percentile


MARKER = "<!-- generated-e2e-results -->"
CHECK_LABELS = {
    "selector": "Playwright button interaction",
    "wasm": "Basic WASM execution",
    "webCrypto": "WebCrypto SHA-256 execution",
    "stylesheetObserved": "Stylesheet request observed",
    "contextIsolation": "Cookies isolated between voters",
    "harFlushed": "HAR saved on context close",
    "harBodySizesValid": "Transferred response-body sizes",
}


def request_key(request: dict) -> tuple[str, str, str]:
    """Normalize dynamic routing IDs without publishing captured query strings."""
    url = urlsplit(request["url"])
    path = re.sub(
        r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}",
        "{id}",
        url.path,
    )
    path = re.sub(r"/resources/[^/]+/", "/resources/{version}/", path)
    path = re.sub(r"/[0-9a-f]{16,}\.wasm", "/{bundle}.wasm", path)
    service = {
        "127.0.0.1": "portal",
        "keycloak": "Keycloak",
        "graphql-engine": "Hasura",
        "minio-proxy": "object storage",
    }.get(url.hostname, "external")
    return service, request["method"], request.get("operation") or path


def cohort_captures(directory: Path) -> tuple[dict, list[dict]]:
    """Load only evidence files named in the cohort manifest, never private targets."""
    manifest = json.loads((directory / "cohort.json").read_text())
    captures = [
        json.loads((directory / item["directory"] / "capture.json").read_text())
        for item in manifest["runs"]
    ]
    return manifest, captures


def render_cohort(directory: Path) -> str:
    """Publish sample latency, observed request paths and SQL counts with their scope."""
    manifest, captures = cohort_captures(directory)
    valid = [
        capture
        for capture in captures
        if capture.get("completed") and capture.get("persistence_verified")
    ]
    lines = [
        "## Recorded local results",
        "",
        f"Verified journeys: **{len(valid)}/{len(captures)}**.",
        "",
    ]
    if not valid:
        return "\n".join(
            lines + ["No verified cohort latency or throughput is available.", ""]
        )
    ballots = sum(len(capture["casts"]) for capture in valid)
    elapsed = sum(capture["elapsed_ms"] for capture in valid)
    first = valid[0]
    lines += [
        f"Measured {datetime.fromtimestamp(first['started_at_ms']/1000, timezone.utc).date()}: {manifest['engine']} {first.get('browser_version', 'unrecorded')}, {manifest['architecture']}, {manifest['logical_cpus']} logical CPUs, concurrency {manifest['concurrency']}.",
        "",
        "One event, one election, one contest, one area and distinct synthetic voters. Every sample uses a fresh browser context, warm application services, full SQL/auto_explain logging and no human think time. This is a diagnostic cohort, not a capacity benchmark or a before/after comparison.",
        "",
        f"HTTP cache: {first.get('http_cache', 'not explicitly controlled')}. All API receipts were matched to the UI receipt and the stored ballot scope.",
        "",
        "| Interval (ms) | Min | p50 | p95 | p99 | Max |",
        "|---|---:|---:|---:|---:|---:|",
    ]
    metrics = [journey_metrics(capture) for capture in valid]
    for label in metrics[0]:
        values = [item[label] for item in metrics if label in item]
        lines.append(
            f"| {label} | {min(values):.1f} | {percentile(values, 50):.1f} | {percentile(values, 95):.1f} | {percentile(values, 99):.1f} | {max(values):.1f} |"
        )
    lines += [
        "",
        f"**{ballots} accepted votes / {elapsed / 1000:.3f} measured journey seconds = {ballots * 1000 / elapsed:.3f} votes/s** for serial browser work. Including runner startup, log-drain waits and verification: {ballots} / {manifest['wall_elapsed_ms'] / 1000:.3f} = {ballots * 1000 / manifest['wall_elapsed_ms']:.3f} votes/s. Neither is sustainable cluster throughput. With n={len(valid)}, p99 is an interpolation near the slowest sample, not a reliable production tail estimate.",
        "",
        "![Recorded journey intervals](/img/voting-flow-e2e-latency.svg)",
        "",
        "### Observed HTTP APIs and resource paths",
        "",
        "Counts preserve repeated requests. GraphQL operation names below all use POST `/v1/graphql`; other rows show normalized paths. IDs, query values, credentials and response bodies are excluded.",
        "",
        "| Service | Method | Operation or path | Total | Mean/voter | Status counts |",
        "|---|---|---|---:|---:|---|",
    ]
    requests = Counter()
    statuses: dict[tuple, Counter] = {}
    for capture in valid:
        for request in capture["requests"]:
            key = request_key(request)
            requests[key] += 1
            statuses.setdefault(key, Counter())[request.get("status", "failed")] += 1
    for key, count in sorted(requests.items()):
        status = ", ".join(
            f"{name}: {number}"
            for name, number in sorted(
                statuses[key].items(), key=lambda pair: str(pair[0])
            )
        )
        lines.append(
            f"| {key[0]} | {key[1]} | `{key[2]}` | {count} | {count / len(valid):.1f} | {status} |"
        )
    lines += [
        "",
        f"Total HTTP requests: **{sum(requests.values())}**, mean **{sum(requests.values()) / len(valid):.1f}/voter**.",
        "",
        "### Observed PostgreSQL work",
        "",
        "Each database is a separate PostgreSQL instance. Counts cover the capture interval plus a one-second log-drain wait. Client IP identifies the submitting service, not the originating HTTP request. Background worker and Hasura metadata activity remain included. Observer verification queries are excluded.",
        "",
        "Submitted statements, transaction commands, nested plans and empty protocol checks are separate categories. Nested plans describe work inside functions/triggers and must not be added as independent client requests. These counts do not measure physical rows, connection checkouts or pool occupancy.",
        "",
        "| Database | Service | Category | SQL verb | Total | Mean/voter | Min–max/voter |",
        "|---|---|---|---|---:|---:|---:|",
    ]
    sql = []
    for capture in valid:
        counts = Counter()
        for database, entries in capture["sql_summary"].items():
            for entry in entries:
                counts[
                    database, entry["service"], entry["kind"], entry["verb"]
                ] += entry["count"]
        sql.append(counts)
    for key in sorted(set().union(*(counts.keys() for counts in sql))):
        values = [counts[key] for counts in sql]
        lines.append(
            f"| {' | '.join(key)} | {sum(values)} | {sum(values)/len(valid):.1f} | {min(values)}–{max(values)} |"
        )
    lines += [
        "",
        "Raw SQL and HAR remain private and are regenerated by the scripts. The tables are observations of this cohort, not an assertion that every background statement was caused by a voter.",
        "",
    ]
    return "\n".join(lines)


def plot_cohort(directory: Path, destination: Path) -> None:
    """Draw a compact SVG from actual sample intervals without committing raw JSON."""
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    _, captures = cohort_captures(directory)
    samples = [
        journey_metrics(c)
        for c in captures
        if c.get("completed") and c.get("persistence_verified")
    ]
    if not samples:
        return
    plt.rcParams.update({"svg.fonttype": "none", "font.size": 10})
    labels = list(samples[0])
    figure, axis = plt.subplots(figsize=(9, 3.6))
    for index, (percent, color) in enumerate([(50, "#247ba0"), (99, "#ed9b40")]):
        values = [percentile([s[label] for s in samples], percent) for label in labels]
        axis.barh(
            [i + (index - 0.5) * 0.35 for i in range(len(labels))],
            values,
            height=0.33,
            label=f"p{percent}",
            color=color,
        )
    axis.set_yticks(range(len(labels)), labels)
    axis.invert_yaxis()
    axis.set_xlabel("Milliseconds; instrumented serial journeys")
    axis.set_title(f"Local browser capture: {len(samples)} distinct voters")
    axis.legend()
    figure.tight_layout()
    destination.parent.mkdir(parents=True, exist_ok=True)
    figure.savefig(destination, metadata={"Date": None})
    plt.close(figure)


def render(directory: Path) -> str:
    """Keep prerequisite checks distinct from a persisted full voter journey."""
    if (directory / "performance.json").exists():
        capture = json.loads((directory / "capture.json").read_text())
        metrics = json.loads((directory / "performance.json").read_text())
        return "\n".join(
            [
                "## Recorded GetVoterStatus performance",
                "",
                f"API validation: **{'passed' if capture.get('completed') else 'failed'}**; no ballots are cast by this mode.",
                "",
                f"{metrics['samples']} samples, {metrics['warmup']} excluded warm-ups, concurrency {metrics['concurrency']}.",
                "",
                f"p50 {metrics['p50_ms']:.2f} ms; p95 {metrics['p95_ms']:.2f} ms; p99 {metrics['p99_ms']:.2f} ms; {metrics['requests_per_second']:.2f} requests/s.",
                "",
                "One authenticated voter, warm services, closed-loop requests; authentication and S3 downloads excluded. This is not a production capacity estimate.",
                "",
            ]
        )

    if (directory / "cohort.json").exists():
        return render_cohort(directory)
    lines = [
        "## Recorded local results",
        "",
        f"Report generated: {datetime.now(timezone.utc).date().isoformat()}.",
        "",
    ]
    probe_path = directory / "probe.json"
    if probe_path.exists():
        probe = json.loads(probe_path.read_text())
        lines += [
            f"Obscura {probe['version']}, Playwright {probe.get('playwright_version', 'unrecorded')}, {probe.get('architecture', 'unrecorded')} synthetic compatibility check ({probe['measured_at']}):",
            "",
            "| Check | Result |",
            "|---|---|",
        ]
        lines += [
            f"| {CHECK_LABELS.get(name, name)} | {'Passed' if passed else 'Failed'} |"
            for name, passed in probe["checks"].items()
        ]
        lines += [
            f"| {CHECK_LABELS.get(name, name)} | {'Available' if passed else 'Unavailable'} |"
            for name, passed in probe.get("coverage", {}).items()
        ]
        lines += [
            "",
            f"Probe interval: {probe['elapsed_ms']} ms. This is one synthetic browser check, not login-to-cast latency or a throughput benchmark.",
            "",
        ]
    preflight_path = directory / "preflight.json"
    if preflight_path.exists():
        preflight = json.loads(preflight_path.read_text())
        lines += ["| Capture prerequisite | Ready |", "|---|---|"]
        lines += [
            f"| {name} | {'Yes' if result['ready'] else 'No'} |"
            for name, result in preflight.items()
        ]
        lines += [""]
    capture_path = directory / "capture.json"
    if capture_path.exists():
        capture = json.loads(capture_path.read_text())
        verified = capture.get("completed") and capture.get("persistence_verified")
        lines += [
            f"Full voter journey with persistence verification: **{'passed' if verified else 'not verified'}**.",
            "",
        ]
        if verified:
            count = len(capture["casts"])
            duration = capture["elapsed_ms"]
            lines += [
                f"One diagnostic journey: {duration} ms; {count} accepted ballots; {len(capture['requests'])} observed HTTP requests.",
                "",
                "A single instrumented journey does not establish p50/p99 or sustainable votes/second.",
                "",
            ]
        lines += [
            "SQL coverage: database-interval JSON logs; background statements may be included. Nested SQL and per-request attribution remain unverified.",
            "",
        ]
    else:
        lines += [
            "**No real login-to-cast result is recorded.** Provision and start the application stack, then complete capture and persistence verification.",
            "",
        ]
    return "\n".join(lines)


def generate(directory: Path, document: Path | None = None) -> None:
    """Replace only the generated evidence section, preserving architectural guidance."""
    if (directory / "cohort.json").exists():
        plot_cohort(directory, directory / "voting-flow-e2e-latency.svg")
    if document is None:
        (directory / "report.md").write_text(render(directory))
        return
    original = document.read_text()
    if MARKER not in original:
        raise ValueError("Documentation is missing its generated-results marker")
    document.write_text(original.split(MARKER)[0] + MARKER + "\n\n" + render(directory))


def main() -> None:
    """Regenerate documentation without running a browser or rerunning benchmarks."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--document", type=Path)
    args = parser.parse_args()
    generate(args.directory, args.document)


if __name__ == "__main__":
    main()
