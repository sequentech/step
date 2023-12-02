// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum IKeyCeremonyStatusStatus {
    NOT_STARTED = "NOT_STARTED",
    IN_PROCESS = "IN_PROCESS",
    SUCCESS = "SUCCESS",
    CANCELLED = "CANCELLED",
}

export interface IKeyCeremonyLog {
    created_date: String
    log_text: String
}

export enum IKeyCeremonyTrusteeStatus {
    WAITING = "WAITING",
    KEY_GENERATED = "KEY_GENERATED",
    KEY_RETRIEVED = "KEY_RETRIEVED",
    KEY_CHECKED = "KEY_CHECKED",
}

export interface IKeyCeremonyTrustee {
    name: String
    status: IKeyCeremonyTrusteeStatus
}

export interface IKeyCeremonyStatus {
    start_date: String
    stop_date: String
    threshold: number
    public_key: String
    logs: IKeyCeremonyLog[]
    trustees: IKeyCeremonyTrustee[]
}
