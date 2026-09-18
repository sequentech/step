# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Fail-closed native profile export and source-line union reports."""
import html
import json
import os
from pathlib import Path
from .process import ROOT, execute, save, source_digest


def union(*maps):
    result = {}
    for mapping in maps:
        for filename, lines in mapping.items():
            merged = result.setdefault(filename, {})
            for line, count in lines.items():
                merged[str(line)] = max(merged.get(str(line), 0), int(count))
    return result


def totals(mapping):
    counts = [count for lines in mapping.values() for count in lines.values()]
    return sum(count > 0 for count in counts), len(counts)


def production_lines(mapping, baseline):
    """Unit binaries include test bodies; keep the production binary denominator."""
    return {filename: {line: count for line, count in lines.items()
                       if line in baseline[filename]}
            for filename, lines in mapping.items() if filename in baseline}


def lcov(text):
    result, current = {}, None
    for line in text.splitlines():
        if line.startswith("SF:"):
            filename = Path(line[3:])
            try: relative = filename.relative_to(ROOT).as_posix()
            except ValueError: current = None; continue
            current = result.setdefault(relative, {}) if relative.startswith("packages/") else None
        elif current is not None and line.startswith("DA:"):
            number, count, *_ = line[3:].split(",")
            current[number] = max(current.get(number, 0), int(count))
    return result


def native(artifacts, kind, objects):
    directory = artifacts / "coverage/rust" / kind
    raw = list(directory.rglob("*.profraw"))
    if not raw:
        raise RuntimeError(f"Missing {kind} native profiles")
    if kind == "e2e":
        for service in ("harvest", "windmill", "beat", "b4"):
            if not any((directory / service).glob("*.profraw")):
                raise RuntimeError(f"Missing native profile: {service}")
    sysroot = Path(execute(["rustc", "--print", "sysroot"], capture=True).strip())
    llvm = sysroot / "lib/rustlib/x86_64-unknown-linux-gnu/bin"
    profile = directory / "merged.profdata"
    execute([str(llvm / "llvm-profdata"), "merge", "-sparse", "-failure-mode=any", *map(str, raw), "-o", str(profile)])
    command = [str(llvm / "llvm-cov"), "export", "-format=lcov", f"-instr-profile={profile}", str(objects[0])]
    for file in objects[1:]: command += ["-object", str(file)]
    content = execute(command, capture=True)
    (directory / "coverage.lcov").write_text(content)
    return lcov(content)


def main():
    artifacts = Path(os.environ["E2E_ARTIFACTS"])
    manifest = json.loads((artifacts / "manifest.json").read_text())
    if (ROOT / ".e2e/bin/coverage/source-sha").read_text().strip() != manifest["sha"]:
        raise RuntimeError("Coverage build does not match the tested revision")
    if (ROOT / ".e2e/bin/coverage/source-digest").read_text().strip() != manifest["source_digest"] or source_digest() != manifest["source_digest"]:
        raise RuntimeError("Sources changed between build and coverage collection")
    binaries = [ROOT / ".e2e/bin/coverage" / name for name in ("harvest", "windmill", "beat", "b4", "step-cli")]
    scopes = {"Rust": {"e2e": native(artifacts, "e2e", binaries)}}
    if manifest["coverage"] == "combined":
        unit_objects = [Path(file) for file in json.loads((artifacts / "coverage/rust/unit/objects.json").read_text())]
        if len(unit_objects) != 2 or any(not file.is_relative_to(ROOT / ".e2e/cargo/coverage")
                or not file.is_file() or not os.access(file, os.X_OK) for file in unit_objects):
            raise RuntimeError("Missing current Cargo unit executables")
        unit_objects = binaries + unit_objects
        scopes["Rust"]["unit"] = production_lines(native(artifacts, "unit", unit_objects), scopes["Rust"]["e2e"])
    execute(["node", "packages/e2e/coverage/frontend.cjs"])
    scopes["Frontend"] = json.loads((artifacts / "coverage/frontend-lines.json").read_text())
    public = artifacts / "report"
    public.mkdir(exist_ok=True)
    rows = ["## Code coverage", "", "Source-line union; percentages are never averaged.", "",
            "| Scope | E2E | Unit | Union |", "| --- | ---: | ---: | ---: |"]
    detailed = {}
    def percentage(mapping):
        hit, total = totals(mapping)
        return f"{hit / total * 100:.1f}% ({hit}/{total})" if total else "no executable lines"
    for name, maps in scopes.items():
        baseline = {file: {line: 0 for line in lines} for file, lines in union(*maps.values()).items()}
        e2e = union(baseline, maps["e2e"])
        combined = union(e2e, maps.get("unit", {}))
        unit = percentage(union(baseline, maps["unit"])) if maps.get("unit") else "not collected"
        rows.append(f"| {name} | {percentage(e2e)} | {unit} | {percentage(combined)} |")
        detailed[name] = {"e2e": e2e, "unit": maps.get("unit", {}), "union": combined}
    rows += ["", "Frontend scope: portal source, excluding generated GraphQL, translations, stories, mocks and tests. "
             "Rust scope: production lines linked into instrumented backend services/CLI; unit-only test bodies do not enlarge the denominator. "
             "WASM, Java, SQL, dependencies and unlinked native crates are not measured. Unit scope currently covers voting-portal, sequent-core and step-cli. "
             "Cross-transform function/branch percentages are deliberately not combined.", ""]
    (public / "coverage.md").write_text("\n".join(rows))
    save(public / "coverage.json", {"sha": manifest["sha"], "scopes": detailed})
    sections = []
    for scope, maps in detailed.items():
        for file, lines in sorted(maps["union"].items()):
            source = ROOT / file
            if not source.is_file(): continue
            rendered = []
            for number, line in enumerate(source.read_text(errors="replace").splitlines(), 1):
                count = lines.get(str(number))
                color = "#ddf5dd" if count and count > 0 else "#fbdada" if count == 0 else "transparent"
                rendered.append(f'<div style="background:{color}">{number:5} {html.escape(line)}</div>')
            sections.append(f"<details><summary>{html.escape(scope + ': ' + file)} — {percentage({file: lines})}</summary><pre>{''.join(rendered)}</pre></details>")
    (public / "coverage.html").write_text('<!doctype html><meta charset="utf-8"><title>E2E coverage</title><h1>Source-line coverage union</h1>' + ''.join(sections))


if __name__ == "__main__": main()
