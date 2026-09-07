# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Compare fresh browser descendants on the same read-only status workload.

PSS and CPU are sampled from Linux /proc, exclude the Node driver and do not
represent whole-deployment resources or full voter-journey behavior.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time
from capture import ROOT, save
from measurements import percentile


def snapshot(parent):
    """Read descendant process CPU ticks and proportional resident memory."""
    processes = {}
    for path in Path("/proc").iterdir():
        if not path.name.isdigit():
            continue
        try:
            parts = (path / "stat").read_text().rsplit(")", 1)[1].split()
            processes[int(path.name)] = (
                int(parts[1]),
                int(parts[11]) + int(parts[12]),
                path,
            )
        except (OSError, ValueError):
            pass
    descendants = {parent}
    for _ in range(20):
        expanded = descendants | {
            pid for pid, (ppid, _, _) in processes.items() if ppid in descendants
        }
        if expanded == descendants:
            break
        descendants = expanded
    cpu, pss = {}, 0
    for pid in descendants - {parent}:
        if pid not in processes:
            continue
        _, ticks, path = processes[pid]
        cpu[pid] = ticks
        try:
            lines = (path / "smaps_rollup").read_text().splitlines()
            pss += sum(
                int(line.split()[1]) for line in lines if line.startswith("Pss:")
            )
        except (OSError, ValueError):
            pass
    return cpu, pss


def main():
    """Alternate fresh engines and retain per-run raw timing/resource samples."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target", type=Path)
    parser.add_argument("prepared", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--repeats", type=int, default=3)
    args = parser.parse_args()
    if not 1 <= args.repeats <= 10:
        parser.error("repeats must be between 1 and 10")
    os.umask(0o077)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    target = json.loads(args.target.read_text())
    ballot = json.loads(args.prepared.read_text())[0]
    results = []
    for repeat in range(args.repeats):
        for engine in (
            ("chromium", "obscura") if repeat % 2 == 0 else ("obscura", "chromium")
        ):
            directory = output / f"{repeat}-{engine}"
            directory.mkdir()
            save(
                directory / "input.json",
                dict(
                    engine=engine,
                    ballot=ballot,
                    login_url=target["login_url"],
                    samples=100,
                    port=19223,
                    obscura=str(ROOT / ".cache/obscura/obscura"),
                ),
            )
            with (directory / "runner.log").open("w") as log:
                process = subprocess.Popen(
                    [
                        "node",
                        str(ROOT / "packages/voting-portal/test/load/browser-cost.cjs"),
                        str(directory),
                    ],
                    stdout=log,
                    stderr=log,
                )
                totals = {}
                baseline = None
                last = None
                peak = 0
                started = time.monotonic()
                while process.poll() is None:
                    if time.monotonic() - started > 90:
                        process.terminate()
                        raise RuntimeError("Browser comparison timed out")
                    cpu, pss = snapshot(process.pid)
                    totals.update(cpu)
                    phase = (
                        (directory / "phase").read_text()
                        if (directory / "phase").exists()
                        else ""
                    )
                    if phase == "measured":
                        if baseline is None:
                            baseline = sum(totals.values())
                        last = sum(totals.values())
                        peak = max(peak, pss)
                    time.sleep(0.05)
            if process.returncode:
                raise RuntimeError(f"{engine} comparison failed; see private artifacts")
            if baseline is None or last is None or peak <= 0:
                raise RuntimeError("No valid browser resource samples were collected")
            result = json.loads((directory / "browser.json").read_text())
            result.update(
                p50_ms=percentile(result["samples"], 50),
                p99_ms=percentile(result["samples"], 99),
                sampled_peak_browser_pss_mib=peak / 1024,
                sampled_browser_cpu_seconds=(
                    (last - baseline) / os.sysconf("SC_CLK_TCK")
                    if baseline is not None
                    else None
                ),
            )
            save(directory / "result.json", result)
            results.append({k: v for k, v in result.items() if k != "samples"})
            save(output / "results.json", results)
            print(engine, "completed 100 read-only requests", flush=True)
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
