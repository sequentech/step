# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Entry point of ``python3 -m scripts.dev.affected``."""

import sys

from .cli import main

sys.exit(main())
