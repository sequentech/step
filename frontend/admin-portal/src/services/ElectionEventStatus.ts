// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum IKeysGenerationStatus {
    NOT_CREATED = "NOT_CREATED",
    CREATED = "CREATED",
}

export interface IElectionEventStatus {
    keys_generation?: IKeysGenerationStatus
}

export const getKeysGenerationStatus = (data?: IElectionEventStatus): IKeysGenerationStatus => {
    return data?.keys_generation || IKeysGenerationStatus.NOT_CREATED
}
