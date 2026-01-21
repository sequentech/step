// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export interface ICastVoteEntry {
    statement_timestamp: number
    statement_kind: string
    ballot_id: string
    username: string | null
    message: string | null
}
