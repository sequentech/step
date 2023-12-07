// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum IExecutionStatus {
    NOT_STARTED = "NotStarted",
    IN_PROCESS = "InProcess",
    SUCCESS = "Success",
    CANCELLED = "Cancelled",
}


export interface ILog {
    created_date: string;
    log_text: string;
}


export enum ITrusteeStatus {
    WAITING = "Waiting",
    KEY_GENERATED = "KeyGenerated",
    KEY_RETRIEVED = "KeyRetrieved",
    KEY_CHECKED = "KeyChecked",
}


export interface ITrustee {
    name: string;
    status: ITrusteeStatus;
}


export interface ICeremonyStatus {
    stop_date?: string;
    public_key: string;
    logs: Array<ILog>;
    trustees: Array<ITrustee>;
}


export enum ITallyExecutionStatus {
    NOT_STARTED = "NotStarted",
    STARTED = "Started",
    CONNECTED = "Connected",
    IN_PROGRESS = "InProgress",
    SUCCESS = "Success",
    CANCELLED = "Cancelled",
}


export enum ITallyTrusteeStatus {
    WAITING = "Waiting",
    KEY_RESTORED = "KeyRestored",
    KEY_CHECKED = "KeyChecked",
}


export interface ITallyTrustee {
    name: string;
    status: ITallyTrusteeStatus;
}


export enum ITallyElectionStatus {
    WAITING = "Waiting",
    MIXING = "Mixing",
    DECRYPTING = "Decrypting",
    COUNTING = "Counting",
    SUCCESS = "Success",
    ERROR = "Error",
}


export interface ITallyElection {
    election_id: string;
    status: ITallyElectionStatus;
    progress: number;
}


export interface ITallyCeremonyStatus {
    stop_date?: string;
    logs: Array<ILog>;
    trustees: Array<ITallyTrustee>;
    elections_status: Array<ITallyElection>;
}

