// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {VotingStatusChannel} from "@sequentech/ui-core"

export interface ICronConfig {
    cron?: string
    scheduled_date?: string
}

export interface IManageElectionDatePayload {
    election_id?: string
    voting_channels?: VotingStatusChannel[] | null
}
