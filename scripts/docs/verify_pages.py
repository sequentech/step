# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Require the Pages deployment job and a live content marker to succeed."""
import argparse
import json
from pathlib import Path
import subprocess
import time
import urllib.request


def deployed(runs, sha):
    matches = [r for r in runs if r.get("head_sha") == sha and r.get("name") == "pages build and deployment"]
    if not matches:
        return False
    latest = max(matches, key=lambda r: (r["id"], r.get("run_attempt", 1)))
    if latest["status"] != "completed":
        return False
    if latest["conclusion"] != "success":
        raise RuntimeError(f"Pages deployment failed: {latest['html_url']}")
    return True


def get(url):
    request = urllib.request.Request(url, headers={"Cache-Control": "no-cache"})
    with urllib.request.urlopen(request, timeout=30) as response:
        if response.status != 200:
            raise RuntimeError(f"HTTP {response.status}: {url}")
        return response.read()


def verification_paths(site, target=None):
    paths = [".pages-deployment.json"]
    if target:
        paths.append(target + "/index.html")
    for number in (2335, 3949):
        prefix = f"docusaurus/pr-preview/pr-{number}/"
        policy = prefix + "docs/technology/vulnerability-disclosure-policy"
        # Docusaurus currently uses trailingSlash: false (.html output).
        # Also support a reviewed future switch to directory/index.html output.
        candidates = [policy + ".html", policy + "/index.html"]
        present = [path for path in candidates if (site / path).is_file()]
        if len(present) != 1:
            raise RuntimeError(f"Missing or ambiguous protected policy output for PR {number}")
        paths.extend(present + [prefix + ".well-known/security.txt", prefix + "security/pgp-key.txt"])
    for path in paths:
        if not (site / path).is_file():
            raise RuntimeError(f"Missing expected publication file: {path}")
    return paths


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--repo", required=True)
    p.add_argument("--sha", required=True)
    p.add_argument("--site", required=True)
    p.add_argument("--base-url", default="https://docs.sequentech.io")
    p.add_argument("--target")
    p.add_argument("--timeout", type=int, default=1200)
    args = p.parse_args()
    site = Path(args.site)
    marker = (site / ".pages-deployment.json").read_bytes()
    deadline = time.monotonic() + args.timeout
    while time.monotonic() < deadline:
        runs = json.loads(subprocess.check_output(["gh", "api", f"repos/{args.repo}/actions/runs?head_sha={args.sha}&per_page=100"]))["workflow_runs"]
        if deployed(runs, args.sha):
            break
        print("Waiting for the actual Pages deployment", flush=True)
        time.sleep(20)
    else:
        raise RuntimeError("Timed out waiting for Pages deployment")
    paths = verification_paths(site, args.target)
    while time.monotonic() < deadline:
        try:
            for path in paths:
                expected = marker if path == paths[0] else (site / path).read_bytes()
                actual = get(args.base_url.rstrip("/") + "/" + path + "?deployment=" + args.sha)
                if actual != expected:
                    raise RuntimeError(f"Live content does not match deployed output: {path}")
            print(f"Verified Pages deployment {args.sha} and {len(paths)} live files")
            return
        except (OSError, RuntimeError) as error:
            print(f"Waiting for live publication: {error}", flush=True)
            time.sleep(20)
    raise RuntimeError("Timed out verifying live Pages content")


if __name__ == "__main__":
    main()
