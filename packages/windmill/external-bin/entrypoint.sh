#!/bin/sh
# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
# Set the core dump size limit to 0 (disabled)
ulimit -c 0

# Execute the command passed as arguments
exec "$@"