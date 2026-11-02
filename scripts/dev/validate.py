# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Broader validation: ``step-dev validate [--depth full] [--base REF]``.

Runs every fast and slow check the current changes select, as
``step-dev test --affected --depth broad``; ``--depth full`` adds the
integration checks that start services. Unknown impact selects every check.
"""

import signal
import sys

from .test import main

if __name__ == "__main__":
    signal.signal(signal.SIGPIPE, signal.SIG_DFL)
    sys.exit(
        main(prog="step-dev validate", defaults=["--affected", "--depth", "broad"])
    )
