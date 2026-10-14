// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {VotingStatusChannel} from "@sequentech/ui-core"

export interface ICronConfig {
    cron?: string
    /** The instant the scheduler runs, RFC 3339 with an offset. */
    scheduled_date?: string
    /** The wall time as entered, `YYYY-MM-DDTHH:MM`, in `timezone` (VOTE-LIFECYCLE). */
    local?: string
    /** The IANA zone of `local`. */
    timezone?: string
}

export interface IManageElectionDatePayload {
    election_id?: string
    voting_channels?: VotingStatusChannel[] | null
}
