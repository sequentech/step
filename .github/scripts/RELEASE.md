<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Explicit release versions

Run **Automated Release Process** on the intended branch and set `version` to
`X.Y.Z` or `X.Y.Z-rc.N`, optionally prefixed with `v`. The version must match
`release/X.Y` when using a release branch and the `preRelease` input. Leave
`version` empty to retain automatic version selection.

An explicit release verifies the checked-out branch and dispatch commit before
running the pinned `release-it`. Before downstream jobs receive the version,
both the local tag and the published `origin` tag must resolve to the release
commit. A failed publication or successful command that did not publish the tag
fails the workflow. Explicit releases do not delete tags during cleanup.

A retry reuses an existing remote tag only when it resolves to the dispatch
commit. It does not republish the GitHub release. A tag at another commit fails;
the workflow never moves or force-pushes it. Use the original commit for a retry
or choose a new version for changed sources. Image publishing, registry copying
and deployment remain separate downstream operations.

To test locally, install `release-it@16.2.1` on `PATH` and run:

```sh
python3 .github/scripts/test_release_explicit_version.py
```

The tests use temporary Git repositories and local bare remotes. GitHub release
publication is disabled in the fixtures; no repository credentials are needed.
