# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Run a reviewed Cargo profile and publish a package coverage decision.

Each invocation gets a new report directory. Failed builds and partial exports
cannot reuse an earlier successful summary. The default is a strict 95% check;
``--baseline`` measures unfinished packages without claiming the target is met.
"""

import argparse
import fcntl
import hashlib
import json
import os
import re
import signal
import subprocess
import sys
import tempfile
import time
import tomllib
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from report import (
    CoverageError,
    exclusion_arguments,
    summarize,
    validate_exclusions,
)

ROOT = Path(__file__).resolve().parents[2]
WORKSPACE = ROOT / "packages"
CONFIG = Path(__file__).with_name("profiles.toml")


def execute(
    command: list[str],
    log: Path,
    environment: dict[str, str],
    *,
    cwd: Path | None = None,
) -> str:
    """Capture one command and stop its process group if the run times out."""
    print(f"Running {' '.join(command)}\n  Log: {log}", flush=True)
    with log.open("w") as output:
        process = subprocess.Popen(
            command,
            cwd=WORKSPACE if cwd is None else cwd,
            env=environment,
            stdin=subprocess.DEVNULL,
            stdout=output,
            stderr=subprocess.STDOUT,
            start_new_session=True,
        )
        try:
            returncode = process.wait(timeout=1200)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
            raise CoverageError(f"Command exceeded 20 minutes; see {log}") from None
        except BaseException:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
            raise
    if returncode:
        raise CoverageError(f"Command failed with exit {returncode}; see {log}")
    return log.read_text(errors="replace")


def git_output(*arguments: str) -> str:
    """Read checkout identity without loading hooks or using the shell."""
    output = subprocess.check_output(["git", "-C", str(ROOT), *arguments], text=True)
    # NUL-delimited filenames may legitimately begin or end with whitespace.
    return output if "-z" in arguments else output.strip()


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def validate_artifacts(output: Path) -> None:
    """Require readable, nonempty LCOV and HTML exports before publishing success."""
    lcov = (output / "lcov.info").read_text()
    if (
        not lcov.startswith("SF:")
        or not lcov.rstrip().endswith("end_of_record")
        or re.search(r"^DA:\d+,\d+", lcov, re.MULTILINE) is None
    ):
        raise CoverageError("LCOV export is empty, truncated or invalid")
    html = (output / "html" / "index.html").read_text().lower()
    if "<html" not in html or "</html>" not in html:
        raise CoverageError("HTML export is empty, truncated or invalid")


def source_digest(package: Path) -> str:
    """Identify source contents even when a developer measures uncommitted edits."""
    digest = hashlib.sha256()
    for path in sorted((package / "src").rglob("*.rs")):
        digest.update(path.relative_to(package).as_posix().encode() + b"\0")
        digest.update(path.read_bytes() + b"\0")
    return digest.hexdigest()


def checkout_digest() -> str:
    """Detect edits to any tracked input or new source file during a run.

    A dependency or Cargo manifest can change the result without changing this
    package's own files. Ignore generated output using the checkout's gitignore.
    """
    names = git_output("ls-files", "--cached", "--others", "--exclude-standard", "-z")
    digest = hashlib.sha256()
    for name in sorted(set(names.split("\0")) - {""}):
        path = ROOT / name
        digest.update(name.encode() + b"\0")
        if path.is_symlink():
            digest.update(b"link:" + os.readlink(path).encode())
        elif path.is_file():
            digest.update(path.read_bytes())
        else:
            digest.update(b"missing-or-submodule")
        digest.update(b"\0")
    return digest.hexdigest()


def markdown_summary(profile: str, result: dict[str, Any]) -> str:
    """Keep the target decision and unmeasured scope visible in CI artifacts."""
    decision = "PASS" if result["passes"] else "TARGET NOT MET"
    lines = [
        f"# {profile}: {decision}",
        "",
        f"Revision: `{result['revision']}`",
        "",
        "| Metric | Covered / measured | Percent |",
        "| --- | ---: | ---: |",
    ]
    for metric, counts in result["metrics"].items():
        percent = "n/a" if counts["percent"] is None else f"{counts['percent']:.2f}%"
        lines.append(
            f"| {metric} | {counts['covered']} / {counts['count']} | {percent} |"
        )
    lines.extend(
        [
            "",
            f"Tests passed: {result['tests_passed']}; "
            f"ignored: {result['tests_ignored']}.",
            "",
        ]
    )
    lines.extend(f"- {failure}" for failure in result["failures"])
    lines.extend(["", "## Measurement limits", ""])
    lines.extend(f"- {limitation}" for limitation in result["limitations"])
    if result.get("excluded_files"):
        lines.extend(["", "## Excluded from coverage", ""])
        lines.extend(
            f"- `{name}`: {reason}" for name, reason in result["excluded_files"].items()
        )
    if result["unaccounted_files"]:
        lines.extend(["", "## Files requiring scope review", ""])
        lines.extend(f"- `{name}`" for name in result["unaccounted_files"])
    return "\n".join(lines) + "\n"


def measure(profile_name: str, baseline: bool, offline: bool) -> int:
    """Measure one configured package; return 1 for a failed strict target."""
    config = tomllib.loads(CONFIG.read_text())
    profile = config["profiles"][profile_name]
    package = WORKSPACE / profile["package"]
    parent = ROOT / "coverage" / profile_name
    parent.mkdir(parents=True, exist_ok=True)
    timestamp = datetime.now(UTC).strftime("%Y%m%dT%H%M%SZ-")
    output = Path(tempfile.mkdtemp(prefix=timestamp, dir=parent))
    started = time.monotonic()
    environment = dict(os.environ, CARGO_TERM_COLOR="never", CARGO_BUILD_JOBS="2")
    if offline:
        # Report generation also invokes Cargo metadata internally.
        environment["CARGO_NET_OFFLINE"] = "true"
    # Profiles share dependency compilation. Clear counters explicitly below:
    # --no-report intentionally preserves them for multi-invocation collection.
    environment["CARGO_LLVM_COV_TARGET_DIR"] = str(
        WORKSPACE / "target" / "package-coverage"
    )
    result: dict[str, Any] = {
        "status": "running",
        "profile": profile_name,
        "passes": False,
    }
    write_json(output / "summary.json", result)

    try:
        excluded_files = profile.get("excluded_files", {})
        validate_exclusions(package, excluded_files, profile["scope_exceptions"])
        export_arguments = exclusion_arguments(package, excluded_files)
        tool = execute(
            ["cargo", "llvm-cov", "--version"], output / "tool.log", environment
        ).strip()
        if tool != f"cargo-llvm-cov {config['cargo_llvm_cov_version']}":
            raise CoverageError(
                f"Expected cargo-llvm-cov {config['cargo_llvm_cov_version']}; "
                f"found {tool}"
            )
        rust = execute(["rustc", "--version"], output / "rust.log", environment).strip()
        pinned_rust = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())[
            "toolchain"
        ]["channel"]
        if not rust.startswith(f"rustc {pinned_rust} "):
            raise CoverageError(f"Expected Rust {pinned_rust}; found {rust}")

        result.update(
            {
                "revision": git_output("rev-parse", "HEAD"),
                "dirty_files": git_output("status", "--porcelain"),
                "source_sha256": source_digest(package),
                "checkout_sha256": checkout_digest(),
                "lockfile_sha256": hashlib.sha256(
                    (WORKSPACE / "Cargo.lock").read_bytes()
                ).hexdigest(),
                "config_sha256": hashlib.sha256(CONFIG.read_bytes()).hexdigest(),
                "features": profile["features"],
                "tools": {"rust": rust, "cargo_llvm_cov": tool},
                "limitations": profile["limitations"],
                "issue": profile["issue"],
                "mode": "baseline" if baseline else "strict",
            }
        )

        arguments = ["--package", profile["package"], "--locked"]
        # cargo-llvm-cov also asks Cargo for workspace metadata when exporting.
        # Validate the lockfile before those internal, unflagged metadata calls.
        execute(
            [
                "cargo",
                "metadata",
                "--format-version=1",
                "--no-deps",
                "--locked",
                "--manifest-path",
                str(WORKSPACE / "Cargo.toml"),
            ],
            output / "metadata.log",
            environment,
        )
        if profile["features"]:
            arguments.extend(["--features", ",".join(profile["features"])])
        if offline:
            arguments.append("--offline")
        execute(
            ["cargo", "llvm-cov", "clean", "--workspace"],
            output / "clean.log",
            environment,
        )
        test_command = ["cargo", "llvm-cov", "--tests", "--no-report", *arguments]
        test_log = execute(test_command, output / "tests.log", environment)
        counts = re.findall(
            r"test result: ok\. (\d+) passed; \d+ failed; (\d+) ignored;", test_log
        )
        result["tests_passed"] = sum(int(passed) for passed, _ in counts)
        result["tests_ignored"] = sum(int(ignored) for _, ignored in counts)
        if result["tests_passed"] == 0:
            raise CoverageError(
                "No passing tests were recorded; coverage is not a valid baseline"
            )

        # Every exported format omits excluded files from its counters as well
        # as its file list. Exclusion reasons remain in the summary for review.
        for format_name, filename in (("json", "llvm.json"), ("lcov", "lcov.info")):
            execute(
                [
                    "cargo",
                    "llvm-cov",
                    "report",
                    *export_arguments,
                    f"--{format_name}",
                    "--output-path",
                    str(output / filename),
                ],
                output / f"{format_name}.log",
                environment,
            )
        execute(
            [
                "cargo",
                "llvm-cov",
                "report",
                *export_arguments,
                "--html",
                "--output-dir",
                str(output),
            ],
            output / "html.log",
            environment,
        )
        execute(
            ["cargo", "llvm-cov", "report", *export_arguments, "--show-missing-lines"],
            output / "uncovered-lines.log",
            environment,
        )

        payload = json.loads((output / "llvm.json").read_text())
        validate_artifacts(output)
        result.update(
            summarize(
                payload,
                package,
                config["minimum_lines"],
                profile["scope_exceptions"],
                excluded_files,
            )
        )
        excluded_paths = {(package / name).resolve() for name in excluded_files}
        if any(
            Path(entry["filename"]).resolve() in excluded_paths
            for entry in payload["data"][0]["files"]
        ):
            raise CoverageError("Excluded source remains in the LLVM export")
        if result["checkout_sha256"] != checkout_digest() or result[
            "revision"
        ] != git_output("rev-parse", "HEAD"):
            raise CoverageError(
                "Source changed during measurement; rerun against a stable checkout"
            )
        result["status"] = "measured"
        summary = markdown_summary(profile_name, result)
        (output / "summary.md").write_text(summary)
        print(summary, flush=True)
        if environment.get("GITHUB_STEP_SUMMARY"):
            with Path(environment["GITHUB_STEP_SUMMARY"]).open("a") as step_summary:
                step_summary.write(summary)
        return 0 if baseline or result["passes"] else 1
    except (CoverageError, OSError, ValueError, subprocess.CalledProcessError) as error:
        result.update({"status": "error", "passes": False, "error": str(error)})
        print(f"Coverage error: {error}", file=sys.stderr)
        return 2
    finally:
        result["duration_seconds"] = round(time.monotonic() - started, 2)
        write_json(output / "summary.json", result)
        print(f"Reports: {output}", flush=True)


def main() -> int:
    global ROOT, WORKSPACE
    config = tomllib.loads(CONFIG.read_text())
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("profile", choices=sorted(config["profiles"]))
    parser.add_argument(
        "--baseline",
        action="store_true",
        help="Measure without passing the 95%% target",
    )
    parser.add_argument(
        "--offline", action="store_true", help="Use only already fetched dependencies"
    )
    parser.add_argument(
        "--checkout",
        type=Path,
        help="Measure another checkout with this runner and its identical profile",
    )
    arguments = parser.parse_args()
    if arguments.checkout is not None:
        ROOT = arguments.checkout.resolve()
        WORKSPACE = ROOT / "packages"
    lock = ROOT / "coverage" / ".lock"
    lock.parent.mkdir(parents=True, exist_ok=True)
    with lock.open("a") as handle:
        try:
            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            print(
                "Another coverage run is using this checkout; wait for it to finish.",
                file=sys.stderr,
            )
            return 2
        return measure(arguments.profile, arguments.baseline, arguments.offline)


if __name__ == "__main__":
    sys.exit(main())
