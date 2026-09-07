# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Generate Docusaurus evidence from real capture or prerequisite-check artifacts."""

import argparse
from datetime import datetime, timezone
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DOC = ROOT / "docs/docusaurus/docs/07-developers/05-voting-portal/voting-flow-e2e.md"
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


def render(directory: Path) -> str:
    """Keep prerequisite checks distinct from a persisted full voter journey."""
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


def generate(directory: Path, document: Path = DOC) -> None:
    """Replace only the generated evidence section, preserving architectural guidance."""
    original = document.read_text()
    if MARKER not in original:
        raise ValueError("Documentation is missing its generated-results marker")
    document.write_text(original.split(MARKER)[0] + MARKER + "\n\n" + render(directory))


def main() -> None:
    """Regenerate documentation without running a browser or rerunning benchmarks."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--document", type=Path, default=DOC)
    args = parser.parse_args()
    generate(args.directory, args.document)


if __name__ == "__main__":
    main()
