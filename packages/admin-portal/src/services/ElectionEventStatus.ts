// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export interface IElectionEventStatus {
    config_created?: boolean
}

export enum IVotingStatus {
    NOT_STARTED = "NOT_STARTED",
    OPEN = "OPEN",
    PAUSED = "PAUSED",
    CLOSED = "CLOSED",
}

export const getStatus = (data: IElectionEventStatus): IElectionEventStatus => {
    return data
}

export const getConfigCreatedStatus = (data?: IElectionEventStatus): boolean => {
    return data?.config_created || false
}
