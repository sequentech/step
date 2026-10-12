// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum IKeysCeremonyExecutionStatus {
    USER_CONFIGURATION = "USER_CONFIGURATION",
    STARTED = "STARTED",
    IN_PROGRESS = "IN_PROGRESS",
    SUCCESS = "SUCCESS",
    CANCELLED = "CANCELLED",
    FAILED = "FAILED",
}

export interface IKeysCeremonyLog {
    created_date: string
    log_text: string
}

export enum IKeysCeremonyTrusteeStatus {
    WAITING = "WAITING",
    SHARES_POSTED = "SHARES_POSTED",
    KEY_GENERATED = "KEY_GENERATED",
    KEY_RETRIEVED = "KEY_RETRIEVED",
    KEY_CHECKED = "KEY_CHECKED",
    HALTED = "HALTED",
}

export const isKeysCeremonyTerminal = (status: IKeysCeremonyExecutionStatus): boolean =>
    status === IKeysCeremonyExecutionStatus.SUCCESS ||
    status === IKeysCeremonyExecutionStatus.CANCELLED ||
    status === IKeysCeremonyExecutionStatus.FAILED

export interface IKeysCeremonyTrustee {
    name: string
    status: IKeysCeremonyTrusteeStatus
}

export interface IExecutionStatus {
    stop_date?: string
    public_key?: string | null
    public_key_hash?: string | null
    failure?: string | null
    logs: Array<IKeysCeremonyLog>
    trustees: Array<IKeysCeremonyTrustee>
}

interface CanTrusteeRecheckPrivateKeyParams {
    executionStatus: IKeysCeremonyExecutionStatus
    trusteeStatus?: IKeysCeremonyTrusteeStatus
    isAutomaticCeremony: boolean
}

export const canTrusteeRecheckPrivateKey = ({
    executionStatus,
    trusteeStatus,
    isAutomaticCeremony,
}: CanTrusteeRecheckPrivateKeyParams): boolean => {
    if (isAutomaticCeremony || trusteeStatus !== IKeysCeremonyTrusteeStatus.KEY_CHECKED) {
        return false
    }

    return (
        executionStatus === IKeysCeremonyExecutionStatus.IN_PROGRESS ||
        executionStatus === IKeysCeremonyExecutionStatus.SUCCESS
    )
}

/** Custody steps require the joint key verified by the native DKG board. */
export const hasGeneratedCeremonyKeys = (status: IExecutionStatus): boolean =>
    Boolean(status.public_key && status.public_key_hash) &&
    status.trustees.length > 0 &&
    status.trustees.every((trustee) =>
        [
            IKeysCeremonyTrusteeStatus.KEY_GENERATED,
            IKeysCeremonyTrusteeStatus.KEY_RETRIEVED,
            IKeysCeremonyTrusteeStatus.KEY_CHECKED,
        ].includes(trustee.status)
    )
