# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""The change and dependency model, run as ``python3 -m scripts.dev.affected``.

It maps changed files to workspace packages and areas, follows their consumers
and selects the checks to run. ``scripts/dev/affected.toml`` holds everything the
workspace manifests do not record; ``step-dev test`` and CI share it.
"""
