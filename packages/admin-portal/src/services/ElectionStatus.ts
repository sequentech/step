// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum IVotingStatus {
    NOT_STARTED = "NOT_STARTED",
    OPEN = "OPEN",
    PAUSED = "PAUSED",
    CLOSED = "CLOSED",
}

export interface IElectionStatus {
    voting_status?: IVotingStatus
}

export const getVotingStatus = (data?: IElectionStatus): IVotingStatus => {
    return data?.voting_status || IVotingStatus.NOT_STARTED
}
