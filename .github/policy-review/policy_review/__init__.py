# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""Policy review engine.

A generic, content-free engine that reviews a pull request against the policy
files carried by the repository being reviewed. The engine ships no policies of
its own: every rule it enforces is read at runtime from the calling repository's
configured policies directory.
"""

__all__ = ["__version__"]

__version__ = "1.0.0"
