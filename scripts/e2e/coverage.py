# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Merge the backend E2E services' LLVM profiles and report package coverage.

scripts/e2e/run.sh runs this in the runtime image, with llvm-tools added to
the build's toolchain, after a STEP_E2E_COVERAGE=1 stack has stopped:

    python3 scripts/e2e/coverage.py <coverage directory> <binary directory>

Every LLVM_PROFILE_FILE in docker-compose-ci-coverage.yml must have produced a
profile that executed code. Package metrics reuse scripts/coverage/report.py:
LLVM lines, functions and regions of packages/<package>/src, without the files
that the package's coverage profiles exclude.
"""

import json
import os
import re
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/coverage"))
from report import METRICS, summarize  # noqa: E402

OVERLAY = ROOT / ".devcontainer/docker-compose-ci-coverage.yml"


def run(*command):
    result = subprocess.run(command, capture_output=True, text=True)
    if result.returncode:
        sys.stderr.write(result.stderr)
        raise SystemExit(
            f"{Path(command[0]).name} failed with exit {result.returncode}"
        )
    return result.stdout


def llvm_tool(name):
    host = re.search(r"^host: (\S+)$", run("rustc", "-vV"), re.MULTILINE)[1]
    sysroot = run("rustc", "--print", "sysroot").strip()
    return str(Path(sysroot, "lib/rustlib", host, "bin", name))


def exit_codes(path):
    """Read `docker compose ps --all --format json`, as an array or JSON lines."""
    if not path.is_file():
        return {}
    text = path.read_text()
    try:
        rows = json.loads(text)
    except json.JSONDecodeError:
        rows = [json.loads(line) for line in text.splitlines() if line.startswith("{")]
    rows = rows if isinstance(rows, list) else [rows]
    return {row["Service"]: row.get("ExitCode") for row in rows}


def profiles(directory, profdata):
    """Merge each profile name's files; count the functions they executed."""
    names = re.findall(
        r"LLVM_PROFILE_FILE: /profiles/([a-z0-9-]+)-%", OVERLAY.read_text()
    )
    if not names:
        raise SystemExit(f"No LLVM_PROFILE_FILE in {OVERLAY}")
    codes = exit_codes(directory / "services.json")
    inventory = []
    for name in names:
        files = sorted((directory / "profiles").glob(f"{name}-*.profraw"))
        merged = directory / "profdata" / f"{name}.profdata"
        executed = 0
        if files and all(path.stat().st_size for path in files):
            run(profdata, "merge", "--sparse", "-o", str(merged), *map(str, files))
            shown = run(profdata, "show", str(merged))
            executed = int(re.search(r"^Total functions: (\d+)$", shown, re.M)[1])
        inventory.append(
            {
                "profile": name,
                "files": len(files),
                "bytes": sum(path.stat().st_size for path in files),
                "executed_functions": executed,
                "exit_code": codes.get(name),
                "profdata": merged if executed else None,
            }
        )
    return inventory


def package_directories(payload):
    """Map each measured packages/<package>/src tree to its Cargo package name."""
    found = {}
    for entry in payload["data"][0]["files"]:
        path = Path(entry["filename"]).resolve()
        for parent in path.parents:
            if (
                path.is_relative_to(parent / "src")
                and (parent / "Cargo.toml").is_file()
            ):
                if parent not in found:
                    manifest = tomllib.loads((parent / "Cargo.toml").read_text())
                    found[parent] = manifest["package"]["name"]
                break
    return found


def excluded_files(name):
    """Union of the reviewed test-support exclusions of the package's profiles."""
    config = tomllib.loads((ROOT / "scripts/coverage/profiles.toml").read_text())
    excluded = {}
    for profile in config["profiles"].values():
        if profile["package"] == name:
            excluded.update(profile.get("excluded_files", {}))
    return excluded


def percent(counts):
    return "-" if counts["percent"] is None else f"{counts['percent']:.2f}%"


def markdown(packages, inventory, failures):
    lines = [
        "### Coverage exercised by the journeys",
        "",
        "LLVM source-based coverage of `packages/<package>/src` in the instrumented"
        " service binaries, with the `scripts/coverage` metric definitions. Files"
        " that no binary links are outside the denominators.",
        "",
        "| Package | Lines | Functions | Regions | Files measured |",
        "| --- | ---: | ---: | ---: | ---: |",
    ]
    for row in packages:
        cells = [
            f"{percent(row[metric])} ({row[metric]['covered']}/{row[metric]['count']})"
            for metric in METRICS
        ]
        files = f"{row['files_measured']}/{row['source_files']}"
        lines.append(f"| `{row['package']}` | {' | '.join(cells)} | {files} |")
    lines += [
        "",
        "| Profile | Files | Size | Executed functions | Exit code |",
        "| --- | ---: | ---: | ---: | ---: |",
    ]
    for row in inventory:
        code = "-" if row["exit_code"] is None else row["exit_code"]
        lines.append(
            f"| `{row['profile']}` | {row['files']} | {row['bytes'] / 2**20:.1f} MiB"
            f" | {row['executed_functions']} | {code} |"
        )
    lines += [""] + [f"**Coverage failure:** {failure}" for failure in failures]
    return "\n".join(lines).rstrip() + "\n"


def main():
    directory, binaries = map(Path, sys.argv[1:3])
    profdata, cov = llvm_tool("llvm-profdata"), llvm_tool("llvm-cov")
    (directory / "profdata").mkdir(exist_ok=True)
    inventory = profiles(directory, profdata)
    failures = [
        f"`{row['profile']}` wrote no profile that executed code"
        for row in inventory
        if not row["executed_functions"]
    ]
    # These continuous-profile services terminate through init on SIGTERM.
    terminated = {"beat", "b4", "trustee1", "trustee2"}
    for row in inventory:
        # The ephemeral driver is removed; run.sh propagates its exit status.
        if row["profile"] == "step-cli":
            continue
        allowed = {0, 143} if row["profile"] in terminated else {0}
        if row["exit_code"] not in allowed:
            failures.append(
                f"`{row['profile']}` has unexpected exit code {row['exit_code']}"
            )
    packages = []
    measured = [str(row["profdata"]) for row in inventory if row["profdata"]]
    if measured:
        merged = directory / "profdata/merged.profdata"
        run(profdata, "merge", "--sparse", "-o", str(merged), *measured)
        objects = sorted(
            path
            for path in binaries.iterdir()
            if path.is_file() and path.name[0] != "." and os.access(path, os.X_OK)
        )
        if not objects:
            raise SystemExit(f"No instrumented binaries in {binaries}")
        command = [cov, "export", "--summary-only", f"--instr-profile={merged}"]
        command.append(str(objects[0]))
        for path in objects[1:]:
            command += ["--object", str(path)]
        payload = json.loads(run(*command))
        for package, name in sorted(
            package_directories(payload).items(), key=lambda item: item[1]
        ):
            excluded = excluded_files(name)
            result = summarize(payload, package, 0, {}, excluded)
            packages.append(
                {
                    "package": name,
                    "path": package.relative_to(ROOT).as_posix(),
                    **result["metrics"],
                    "files_measured": len(result["files"]),
                    "source_files": len(result["source_files"]),
                    "excluded_files": sorted(excluded),
                    "files": result["files"],
                }
            )
    for row in inventory:
        row["profdata"] = row["profdata"] and row["profdata"].name
    summary = {"packages": packages, "profiles": inventory, "failures": failures}
    (directory / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    text = markdown(packages, inventory, failures)
    (directory / "summary.md").write_text(text)
    print(text, end="")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
