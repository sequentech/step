# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Plan retention first; mutate only generated PR previews after the size check."""
import argparse
from datetime import datetime, timedelta, timezone
import json
from pathlib import Path
import re
import shutil

PREVIEW = re.compile(r"pr-([1-9][0-9]*)\Z")
TARGET = re.compile(r"docusaurus/(?:main|pr-preview/pr-[1-9][0-9]*|release-preview/release/[A-Za-z0-9._-]+)\Z")


def files(root):
    for path in root.rglob("*"):
        if path.relative_to(root).parts[0] == ".git":
            continue
        if path.is_symlink():
            raise ValueError(f"Symlinks are not allowed in Pages output: {path}")
        if path.is_file():
            yield path


def size(root):
    return sum(path.stat().st_size for path in files(root)) if root.exists() else 0


def open_prs(pages):
    """gh api --paginate --slurp produces a list of complete result pages."""
    if not isinstance(pages, list) or not pages or not all(isinstance(page, list) for page in pages):
        raise ValueError("Expected paginated open-PR API results; refusing cleanup")
    result = {}
    for page in pages:
        for pr in page:
            if pr.get("state") != "open" or not isinstance(pr.get("number"), int):
                raise ValueError("Invalid open-PR metadata; refusing cleanup")
            result[pr["number"]] = datetime.fromisoformat(pr["updated_at"].replace("Z", "+00:00"))
    return result


def plan(site, policy, prs, now, source=None, target=None, active_pr=None):
    site = Path(site)
    pins = set(policy["pinned_prs"])
    if not {2335, 3949} <= pins:
        raise ValueError("PRs 2335 and 3949 must remain explicitly pinned")
    if not 0 < policy["max_site_bytes"] <= 900_000_000:
        raise ValueError("Pages guardrail must be at most 900 MB")
    if policy["max_unpinned_previews"] < 1 or policy["max_age_days"] < 1:
        raise ValueError("Retention count and age must be positive")
    if bool(source) != bool(target) or (target and not TARGET.fullmatch(target)):
        raise ValueError("Invalid publication target")
    if source and (not Path(source).is_dir() or not (Path(source)/"index.html").is_file()):
        raise ValueError("Publication source must be a complete site with index.html")
    if active_pr and (target != f"docusaurus/pr-preview/pr-{active_pr}" or active_pr not in prs):
        raise ValueError("Only an open PR may publish its matching preview")
    total = size(site)
    existing = {}
    parent = site / "docusaurus/pr-preview"
    if parent.exists():
        for path in parent.iterdir():
            match = PREVIEW.fullmatch(path.name)
            if match and path.is_dir():
                existing[int(match[1])] = size(path)
    target_bytes = size(site / target) if target else 0
    if source:
        total += size(Path(source)) - target_bytes
    cutoff = now - timedelta(days=policy["max_age_days"])
    eligible = sorted((n for n in existing if n not in pins and n in prs and prs[n] >= cutoff),
                      key=lambda n: (prs[n], n), reverse=True)
    # The preview being published counts towards the bound even when it is new.
    if active_pr and active_pr not in pins:
        eligible = [active_pr] + [n for n in eligible if n != active_pr]
    kept = set(eligible[:policy["max_unpinned_previews"]]) | pins
    remove = {n for n in existing if n not in kept}
    total -= sum(existing[n] for n in remove)
    # Prefer evicting the least recently updated unpinned output before failing.
    for number in reversed(eligible[:policy["max_unpinned_previews"]]):
        if total <= policy["max_site_bytes"]:
            break
        if number != active_pr:
            remove.add(number)
            total -= existing[number]
    if total > policy["max_site_bytes"]:
        raise ValueError(f"Projected Pages output is {total:,} bytes, above the {policy['max_site_bytes']:,}-byte guardrail; protected content was not removed")
    return {"remove": sorted(remove), "projected_bytes": total, "pinned_prs": sorted(pins)}


def apply(site, result, source=None, target=None):
    site = Path(site)
    for number in result["remove"]:
        shutil.rmtree(site / f"docusaurus/pr-preview/pr-{number}")
    if source:
        destination = site / target
        if destination.exists():
            shutil.rmtree(destination)
        shutil.copytree(source, destination)
    actual = size(site)
    if actual != result["projected_bytes"]:
        raise ValueError(f"Output changed during cleanup: expected {result['projected_bytes']}, got {actual}; do not push")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--site", required=True)
    parser.add_argument("--policy", required=True)
    parser.add_argument("--open-prs", required=True)
    parser.add_argument("--source")
    parser.add_argument("--target")
    parser.add_argument("--active-pr", type=int)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    policy = json.loads(Path(args.policy).read_text())
    prs = open_prs(json.loads(Path(args.open_prs).read_text()))
    result = plan(Path(args.site), policy, prs, datetime.now(timezone.utc), args.source, args.target, args.active_pr)
    print(json.dumps(result, indent=2))
    if not args.dry_run:
        apply(args.site, result, args.source, args.target)


if __name__ == "__main__":
    main()
