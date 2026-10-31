# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Release an explicit version and verify its published Git tag.

The workflow supplies REQUESTED_VERSION, RELEASE_REF and RELEASE_SHA. A retry
reuses a remote tag already at the dispatch commit; it does not republish the
GitHub release. Conflicting tags are never moved or deleted.
"""

import argparse
import os
import re
import subprocess
import sys


VERSION = re.compile(r"v?(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-rc\.(0|[1-9][0-9]*))?")


def git(*args):
    return subprocess.check_output(["git", *args], text=True).strip()


def remote_tag(tag):
    ref = f"refs/tags/{tag}"
    rows = git("ls-remote", "origin", ref, ref + "^{}")
    refs = dict(line.split()[::-1] for line in rows.splitlines())
    return refs.get(ref + "^{}", refs.get(ref))


def version_key(match):
    major, minor, patch, rc = match.groups()
    return int(major), int(minor), int(patch), float("inf") if rc is None else int(rc)


def release(check_only=False):
    requested = os.environ["REQUESTED_VERSION"]
    match = VERSION.fullmatch(requested)
    if not match:
        raise ValueError("Expected version X.Y.Z or X.Y.Z-rc.N, optionally prefixed with v")
    version = requested.removeprefix("v")
    tag = "v" + version
    ref = os.environ["RELEASE_REF"]
    expected = os.environ["RELEASE_SHA"]
    if not ref.startswith("refs/heads/") or not re.fullmatch(r"[0-9a-f]{40}", expected):
        raise ValueError("An explicit release requires a branch ref and dispatch commit SHA")
    branch = ref.removeprefix("refs/heads/")
    subprocess.run(["git", "check-ref-format", ref], check=True)
    if git("branch", "--show-current") != branch:
        raise ValueError("Checkout does not match the dispatch branch")
    if git("rev-parse", "HEAD") != expected:
        raise ValueError("Checkout does not match the dispatch commit")
    release_branch = re.fullmatch(r"release/(\d+)\.(\d+)", branch)
    if release_branch and release_branch.groups() != match.groups()[:2]:
        raise ValueError(f"Version {version} does not belong to branch {branch}")

    existing = remote_tag(tag)
    if existing:
        if existing != expected:
            raise ValueError(f"Remote tag {tag} already points to a different commit: {existing}")
        # Fetch the verified tag without force so the workflow can resolve HASH.
        if not check_only:
            git("fetch", "--no-tags", "origin", f"refs/tags/{tag}:refs/tags/{tag}")
            if git("rev-parse", f"refs/tags/{tag}^{{commit}}") != expected or remote_tag(tag) != expected:
                raise ValueError(f"Tag {tag} changed during verification")
        print(f"Reusing published tag {tag} at {expected}")
        return

    latest = subprocess.run(
        ["git", "describe", "--tags", "--match=v[0-9]*.[0-9]*.[0-9]*", "--abbrev=0"],
        text=True, capture_output=True, check=False,
    )
    if latest.returncode == 0:
        previous = VERSION.fullmatch(latest.stdout.strip())
        if previous and version_key(match) <= version_key(previous):
            raise ValueError(f"Version {tag} is not newer than {latest.stdout.strip()}")
    if check_only:
        return

    subprocess.run(["release-it", "--ci", version, "--verbose"], check=True)
    head = git("rev-parse", "HEAD")
    subprocess.run(["git", "merge-base", "--is-ancestor", expected, head], check=True)
    published = remote_tag(tag)
    if published != head:
        raise ValueError(f"Tag {tag} was not published at release commit {head}; remote target: {published}")
    if git("rev-parse", f"refs/tags/{tag}^{{commit}}") != head:
        raise ValueError(f"Local tag {tag} does not match the release commit")
    print(f"Verified published tag {tag} at {head}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Validate without releasing")
    args = parser.parse_args()
    try:
        release(args.check)
    except (ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(f"Error: {error}", file=sys.stderr)
        sys.exit(1)
