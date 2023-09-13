// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export interface IElectionEventStatus {
    config_created?: boolean
}

export const getConfigCreatedStatus = (data?: IElectionEventStatus): boolean => {
    return data?.config_created || false
}
