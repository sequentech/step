// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum ReportActions {
    EDIT = "edit",
    DELETE = "delete",
    GENERATE = "generate",
    PREVIEW = "preview",
    GENERATE_SCHEDULED = "generate-scheduled",
}

export enum EReportType {
    BALLOT_RECEIPT = "BALLOT_RECEIPT",
    ELECTORAL_RESULTS = "ELECTORAL_RESULTS",
    MANUAL_VERIFICATION = "MANUAL_VERIFICATION",
    STATISTICAL_REPORT = "STATISTICAL_REPORT",
}

export enum EReportElectionPolicy {
    ELECTION_REQUIRED = "ELECTION_REQUIRED",
    ELECTION_ALLOWED = "ELECTION_ALLOWED",
    ELECTION_NOT_ALLOWED = "ELECTION_NOT_ALLOWED",
}

export const reportTypeConfig: {
    [key: string]: {
        actions: ReportActions[]
        templateRequired?: boolean
        electionPolicy?: EReportElectionPolicy
    }
} = {
    [EReportType.BALLOT_RECEIPT]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.GENERATE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: true,
        electionPolicy: EReportElectionPolicy.ELECTION_ALLOWED,
    },
    [EReportType.ELECTORAL_RESULTS]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: true,
        electionPolicy: EReportElectionPolicy.ELECTION_ALLOWED,
    },
    [EReportType.MANUAL_VERIFICATION]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: true,
        electionPolicy: EReportElectionPolicy.ELECTION_NOT_ALLOWED,
    },
    [EReportType.STATISTICAL_REPORT]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: true,
        electionPolicy: EReportElectionPolicy.ELECTION_NOT_ALLOWED,
    },
}

export enum EGenerateReportMode {
    REAL = "REAL",
    PREVIEW = "PREVIEW",
}
