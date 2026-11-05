# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Entry point of ``python3 -m scripts.dev.affected``."""

import signal
import sys

from .cli import main

# Output piped into head or less may be cut short; that is not an error.
signal.signal(signal.SIGPIPE, signal.SIG_DFL)
sys.exit(main())
