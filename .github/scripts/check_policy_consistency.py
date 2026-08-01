#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""Assert that the policy system agrees with itself.

One policy owns one label owns one routing rule owns one alert. That chain is
spread across four files, and nothing until now checked that they still say the
same thing. Deleting a policy but leaving its label, renaming a label in one
file, adding a policy and forgetting the alert case — all of those merge
cleanly and silently reduce what the system reports.

This is the check `40-changes-must-be-checked.md` asks for, applied to the
policy system itself.

This repository is self-contained by policy — see
`.github/policies/10-repository-scope.md`. Its CI may not depend on anything it
cannot clone anonymously, so it carries its own copy of this checker rather than
fetching one. That is a deliberate duplicate, not an oversight.

Run: python3 check_policy_consistency.py [repository-root]

The root defaults to two directories above this file, which is right when the
script sits in the repository it is checking.

Exit: 0 when consistent, 1 otherwise. No third-party dependencies.
"""

from __future__ import annotations

import glob
import json
import os
import re
import sys

ROOT = os.path.realpath(
    sys.argv[1] if len(sys.argv) > 1
    else os.path.join(os.path.dirname(__file__), "..", "..")
)
POLICY_DIR = os.path.join(ROOT, ".github", "policies")
CODERABBIT = os.path.join(ROOT, ".coderabbit.yaml")
ALERT = os.path.join(ROOT, ".github", "workflows", "policy-alert.yml")
README = os.path.join(POLICY_DIR, "README.md")
CODEOWNERS = os.path.join(ROOT, ".github", "CODEOWNERS")

LABEL_RE = re.compile(r"^>\s*\*\*Label:\*\*\s*`([^`]+)`", re.M)
CASE_RE = re.compile(r"^\s{10,}(policy:[a-z0-9-]+)\)", re.M)
LINK_RE = re.compile(r"\]\((?!https?://|mailto:|#)([^)#]+)")

failures: list[str] = []
notes: list[str] = []


def fail(msg: str) -> None:
    failures.append(msg)


def rel(path: str) -> str:
    return os.path.relpath(path, ROOT)


def load_yaml(path: str) -> dict:
    """Minimal reader for the two keys we need, so the check has no deps."""
    try:
        import yaml  # type: ignore

        with open(path, encoding="utf-8") as handle:
            return yaml.safe_load(handle) or {}
    except ImportError:
        notes.append("PyYAML not installed — falling back to a regex reader")
        with open(path, encoding="utf-8") as handle:
            text = handle.read()
        labels = re.findall(r'^\s*-\s*label:\s*"([^"]+)"', text, re.M)
        files = re.findall(r'^\s*-\s*files:\s*"([^"]+)"', text, re.M)
        return {
            "reviews": {"labeling_instructions": [{"label": x} for x in labels]},
            "knowledge_base": {
                "code_guidelines": {"filePatterns": [{"files": f} for f in files]}
            },
        }


def main() -> int:
    # ---------------------------------------------------------------- policies
    policy_files = sorted(
        p for p in glob.glob(os.path.join(POLICY_DIR, "*.md"))
        if os.path.basename(p) != "README.md"
    )
    if not policy_files:
        fail(f"no policy files found in {rel(POLICY_DIR)}")
        return report()

    policy_labels: dict[str, str] = {}
    for path in policy_files:
        text = open(path, encoding="utf-8").read()
        found = LABEL_RE.findall(text)
        if not found:
            fail(
                f"{rel(path)}: no '> **Label:** `policy:...`' header. Every policy "
                f"must declare the label it owns."
            )
            continue
        if len(found) > 1:
            fail(f"{rel(path)}: declares {len(found)} labels; a policy owns exactly one")
        policy_labels[found[0]] = rel(path)

    # -------------------------------------------------------------- coderabbit
    config = load_yaml(CODERABBIT)
    reviews = config.get("reviews") or {}
    configured = {
        entry["label"]
        for entry in (reviews.get("labeling_instructions") or [])
        if isinstance(entry, dict) and "label" in entry
    }

    declared = set(policy_labels)
    for label in sorted(declared - configured):
        fail(
            f"label '{label}' is declared by {policy_labels[label]} but has no "
            f"labeling_instructions entry in .coderabbit.yaml — CodeRabbit will "
            f"never apply it"
        )
    for label in sorted(configured - declared):
        fail(
            f"label '{label}' is configured in .coderabbit.yaml but no policy file "
            f"declares it — the rule behind it is missing or was deleted"
        )

    # ------------------------------------------------------------------- alert
    alert_text = open(ALERT, encoding="utf-8").read()
    alert_labels = set(CASE_RE.findall(alert_text))
    for label in sorted(alert_labels - configured):
        fail(
            f"policy-alert.yml has a case for '{label}', which is not a configured "
            f"label — it can never fire"
        )

    # ------------------------------------------------------------------ README
    readme = open(README, encoding="utf-8").read()
    silent: set[str] = set()
    for label in sorted(configured):
        row = re.search(
            r"^\|[^|\n]*\|\s*`" + re.escape(label) + r"`\s*\|([^|\n]*)\|([^|\n]*)\|",
            readme,
            re.M,
        )
        if not row:
            fail(
                f"label '{label}' is missing from the table in "
                f"{rel(README)} — the documented routing and the real routing disagree"
            )
            continue
        if row.group(2).strip().lower() in {"none", "-", "—", ""}:
            silent.add(label)

    for label in sorted(configured - alert_labels - silent):
        fail(
            f"label '{label}' has no case in policy-alert.yml, and "
            f"{rel(README)} does not record it as deliberately silent"
        )

    # ------------------------------------------------- links inside the policies
    for path in policy_files + [README]:
        base = os.path.dirname(path)
        for match in LINK_RE.finditer(open(path, encoding="utf-8").read()):
            target = os.path.normpath(os.path.join(base, match.group(1)))
            if not os.path.exists(target):
                fail(f"{rel(path)}: broken link to '{match.group(1)}'")

    # ---------------------------------------------------------- knowledge base
    patterns = (
        ((config.get("knowledge_base") or {}).get("code_guidelines") or {}).get(
            "filePatterns"
        )
        or []
    )
    guideline_files: list[str] = []
    for pattern in patterns:
        spec = pattern["files"] if isinstance(pattern, dict) else pattern
        matched = glob.glob(os.path.join(ROOT, spec))
        if not matched:
            fail(
                f".coderabbit.yaml: code_guidelines pattern '{spec}' matches no file — "
                f"the reviewer is being pointed at nothing"
            )
        guideline_files.extend(rel(m) for m in matched)
        if isinstance(pattern, dict) and pattern.get("applyTo") != "**":
            fail(
                f".coderabbit.yaml: code_guidelines pattern '{spec}' does not set "
                f"applyTo: \"**\" — it would govern only its own directory"
            )

    # --------------------------------------------- the architecture document
    architecture = [f for f in guideline_files if f.endswith("architecture.md")]
    if not architecture:
        fail(
            ".coderabbit.yaml: no architecture document in code_guidelines — "
            "60-architectural-changes.md asks the reviewer to compare against a "
            "document it would never have read"
        )
    owners = open(CODEOWNERS, encoding="utf-8").read() if os.path.exists(CODEOWNERS) else ""
    for doc in architecture:
        if "/" + doc not in owners:
            fail(
                f"{doc} is not owned in .github/CODEOWNERS — the record could be "
                f"rewritten to match whatever just merged"
            )

    print(
        json.dumps(
            {
                "policies": len(policy_files),
                "labels": sorted(configured),
                "alerting": sorted(alert_labels),
                "silent": sorted(silent),
                "guidelines": guideline_files,
            },
            indent=2,
        )
    )
    return report()


def report() -> int:
    for note in notes:
        print(f"note: {note}", file=sys.stderr)
    if failures:
        print(f"\n{len(failures)} inconsistency(ies) in the policy system:\n", file=sys.stderr)
        for item in failures:
            print(f"  - {item}", file=sys.stderr)
        return 1
    print("\nThe policy system is internally consistent.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
