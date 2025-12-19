# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

Invoke-RestMethod -Uri "http://localhost:3000/boards" -Method Post -ContentType "application/json" -Body '{"name":"protocoltest"}' | ConvertTo-Json