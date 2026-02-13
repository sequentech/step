// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum IKeysCeremonyExecutionStatus {
    USER_CONFIGURATION = "USER_CONFIGURATION",
    STARTED = "STARTED",
    IN_PROGRESS = "IN_PROGRESS",
    SUCCESS = "SUCCESS",
    CANCELLED = "CANCELLED",
}

export interface IKeysCeremonyLog {
    created_date: string
    log_text: string
}

export enum IKeysCeremonyTrusteeStatus {
    WAITING = "WAITING",
    KEY_GENERATED = "KEY_GENERATED",
    KEY_RETRIEVED = "KEY_RETRIEVED",
    KEY_CHECKED = "KEY_CHECKED",
}

export interface IKeysCeremonyTrustee {
    name: string
    status: IKeysCeremonyTrusteeStatus
}

export interface IExecutionStatus {
    stop_date?: string
    public_key?: string
    logs: Array<IKeysCeremonyLog>
    trustees: Array<IKeysCeremonyTrustee>
}
