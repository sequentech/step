// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum IApplicationsStatus {
    PENDING = "PENDING",
    ACCEPTED = "ACCEPTED",
    REJECTED = "REJECTED",
}

export enum RejectReason {
    INSUFFICIENT_INFORMATION = "insufficient-information",
    NO_VOTER = "no-matching-voter",
    ALREADY_APPROVED = "voter-already-approved",
    OTHER = "other", //mandatory comment
}

export enum ApplicationsError {
    APPROVED_VOTER = "Approved_Voter",
}
