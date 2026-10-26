# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
chronyc -N authdata    # every source: NTS in the Mode column
chronyc tracking       # "Leap status" is Normal
chronyc sources        # the selected source (marked *) is one of the site's servers
