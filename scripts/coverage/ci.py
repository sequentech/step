# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Measure a base/head pair and fail only for regressions or invalid evidence.

Run in a disposable worker, after preparing each checkout's locked dependencies.
Both sides keep their own source and tests. Measurement options come from this
one runner so adopting coverage in an existing package can measure its old tests
as well. No downloaded or checked-in percentage can serve as a passing baseline.
"""

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

from ratchet import compare, compare_rust, markdown, python_metrics
from report import CoverageError
from run import execute, write_json

HERE = Path(__file__).resolve().parent


def identity(root: Path) -> str:
    """Require a fixed commit with no modified tracked inputs."""
    revision = subprocess.check_output(
        ["git", "-C", str(root), "rev-parse", "HEAD"], text=True
    ).strip()
    changed = subprocess.check_output(
        ["git", "-C", str(root), "status", "--porcelain", "--untracked-files=normal"],
        text=True,
    )
    if changed or not re.fullmatch(r"[0-9a-f]{40}", revision):
        raise CoverageError(f"Coverage needs a clean, committed checkout: {root}")
    return revision


def command(arguments: list[str], root: Path, output: Path, name: str) -> str:
    """Retain command output and bound the entire child process group."""
    environment = dict(os.environ, CI="true")
    # Keep per-revision target reports out of the final CI verdict. The paired
    # summary, written below, is the only statement about passing this gate.
    environment.pop("GITHUB_STEP_SUMMARY", None)
    return execute(arguments, output / f"{name}.log", environment, cwd=root)


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text())
    if not isinstance(value, dict):
        raise CoverageError(f"Expected a JSON object: {path}")
    return value


def measure_python(root: Path, output: Path) -> dict[str, Any]:
    """Instrument all tooling modules, including unimported production code."""
    data = str(output / ".coverage")
    coverage = [sys.executable, "-m", "coverage"]
    log = command(
        [
            *coverage,
            "run",
            f"--data-file={data}",
            "--branch",
            "--source=scripts/coverage",
            "--omit=*/test_*.py",
            "-m",
            "unittest",
            "discover",
            "-s",
            "scripts/coverage",
        ],
        root,
        output,
        "tests",
    )
    if not re.search(r"Ran [1-9][0-9]* tests? in", log):
        raise CoverageError("No Python tests ran")
    command(
        [*coverage, "json", f"--data-file={data}", "-o", str(output / "coverage.json")],
        root,
        output,
        "json",
    )
    command(
        [*coverage, "html", f"--data-file={data}", "-d", str(output / "html")],
        root,
        output,
        "html",
    )
    return python_metrics(read_json(output / "coverage.json"))


def measure_rust(root: Path, package: str, output: Path) -> dict[str, Any]:
    """Use the candidate runner/profile for both revisions' native tests."""
    parent = root / "coverage" / package
    before = set(parent.glob("*/summary.json"))
    command(
        [
            sys.executable,
            str(HERE / "run.py"),
            package,
            "--baseline",
            "--checkout",
            str(root),
        ],
        root,
        output,
        "measurement",
    )
    created = set(parent.glob("*/summary.json")) - before
    if len(created) != 1:
        raise CoverageError("Expected exactly one new Rust measurement")
    report = read_json(created.pop())
    write_json(output / "coverage.json", report)
    if report.get("revision") != identity(root):
        raise CoverageError("Rust report belongs to another revision")
    return report


def measure(root: Path, kind: str, package: str, output: Path) -> dict[str, Any]:
    output.mkdir()
    if kind == "python":
        return measure_python(root, output)
    return measure_rust(root, package, output)


def paired_run(base: Path, head: Path, kind: str, package: str, parent: Path) -> int:
    """Publish one fresh paired verdict; failed runs cannot reuse old reports."""
    parent.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix=f"{package}-", dir=parent))
    result: dict[str, Any] = {
        "scope": package,
        "status": "error",
        "passes": False,
        "base_revision": "unknown",
        "head_revision": "unknown",
    }
    try:
        result.update(base_revision=identity(base), head_revision=identity(head))
        base_scope = base / (
            "scripts/coverage" if kind == "python" else f"packages/{package}/src"
        )
        head_report = measure(head, kind, package, output / "head")
        if not base_scope.exists():
            # This exception is only for newly introduced source, never a
            # missing report, missing tests, or a failed baseline measurement.
            metrics = head_report if kind != "rust" else head_report["metrics"]
            compare(metrics, metrics)
            result.update(
                status="initialized",
                passes=True,
                note=(
                    "New source scope: no base implementation exists. This run "
                    "establishes its first measurement; no improvement is claimed."
                ),
            )
        else:
            base_report = measure(base, kind, package, output / "base")
            verdict = (
                compare_rust(base_report, head_report)
                if kind == "rust"
                else compare(base_report, head_report)
            )
            result.update(verdict)
            result["status"] = "pass" if result["passes"] else "regression"
        if (
            identity(base) != result["base_revision"]
            or identity(head) != result["head_revision"]
        ):
            raise CoverageError("Checkout changed while coverage was running")
        return 0 if result["passes"] else 1
    except (
        CoverageError,
        OSError,
        ValueError,
        KeyError,
        TypeError,
        subprocess.CalledProcessError,
    ) as error:
        result.update(status="error", passes=False, failures=[str(error)])
        return 2
    finally:
        write_json(output / "verdict.json", result)
        summary = markdown(result)
        (output / "summary.md").write_text(summary)
        print(summary, flush=True)
        print(f"Reports: {output}", flush=True)
        if os.environ.get("GITHUB_STEP_SUMMARY"):
            with Path(os.environ["GITHUB_STEP_SUMMARY"]).open("a") as destination:
                destination.write(summary)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("kind", choices=("python", "rust"))
    parser.add_argument("package")
    parser.add_argument("--base", type=Path, required=True)
    parser.add_argument("--head", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if not re.fullmatch(r"[a-z][a-z0-9-]*", args.package):
        parser.error("Expected a package identifier")
    return paired_run(
        args.base.resolve(),
        args.head.resolve(),
        args.kind,
        args.package,
        args.output.resolve(),
    )


if __name__ == "__main__":
    sys.exit(main())
