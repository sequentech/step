// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum IApplicationsStatus {
    PENDING = "PENDING",
    ACCEPTED = "ACCEPTED",
    REJECTED = "REJECTED",
}

// TODO: This will be updated — ensure to update the translations as well.
export enum RejectReason {
    MISSING_DATA = "missing-data",
    NOT_VALIDATED = "not-validated",
}
