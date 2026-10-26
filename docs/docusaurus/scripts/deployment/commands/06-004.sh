# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
timeout 5 bash -c "</dev/tcp/<authentication-server-host>/443" && echo reachable
