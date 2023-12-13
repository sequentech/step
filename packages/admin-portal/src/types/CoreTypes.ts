// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum EVotingStatus {
    NOT_STARTED = "NOT_STARTED",
    OPEN ="OPEN",
    PAUSED ="PAUSED",
    CLOSED ="CLOSED",
}

export interface IElectionEventStatus {
    config_created?: boolean
    keys_ceremony_finished?: boolean
    tally_ceremony_finished?: boolean
    is_published?: boolean
    voting_status: EVotingStatus
}