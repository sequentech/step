// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum IExecutionStatus {
    NOT_STARTED = "NOT_STARTED",
    IN_PROCESS = "IN_PROCESS",
    SUCCESS = "SUCCESS",
    CANCELLED = "CANCELLED",
}

export interface ILog {
    created_date: string
    log_text: string
}

export enum ITrusteeStatus {
    WAITING = "WAITING",
    KEY_GENERATED = "KEY_GENERATED",
    KEY_RETRIEVED = "KEY_RETRIEVED",
    KEY_CHECKED = "KEY_CHECKED",
}

export interface ITrustee {
    name: string
    status: ITrusteeStatus
}

export interface ICeremonyStatus {
    stop_date?: string
    public_key: string
    logs: Array<ILog>
    trustees: Array<ITrustee>
}

export enum ITallyExecutionStatus {
    NOT_STARTED = "NOT_STARTED",
    STARTED = "STARTED",
    CONNECTED = "CONNECTED",
    IN_PROGRESS = "IN_PROGRESS",
    SUCCESS = "SUCCESS",
    CANCELLED = "CANCELLED",
}

export enum ITallyTrusteeStatus {
    WAITING = "WAITING",
    KEY_RESTORED = "KEY_RESTORED",
    KEY_CHECKED = "KEY_CHECKED",
}

export interface ITallyTrustee {
    name: string
    status: ITallyTrusteeStatus
}

export enum ITallyElectionStatus {
    WAITING = "WAITING",
    MIXING = "MIXING",
    DECRYPTING = "DECRYPTING",
    SUCCESS = "SUCCESS",
    ERROR = "ERROR",
}

export interface ITallyElection {
    election_id: string
    status: ITallyElectionStatus
    progress: number
}

export interface ITallyCeremonyStatus {
    stop_date?: string
    logs: Array<ILog>
    trustees: Array<ITallyTrustee>
    elections_status: Array<ITallyElection>
}

export enum ETallyType {
    ELECTORAL_RESULTS = "ELECTORAL_RESULTS",
    INITIALIZATION_REPORT = "INITIALIZATION_REPORT",
}

export enum ETallyTypeCssClass {
    ELECTORAL_RESULTS = "electoral-results",
    INITIALIZATION_REPORT = "init-report",
}

export enum CreateKeysError {
    PERMISSION_LABELS = "permission-labels",
}
