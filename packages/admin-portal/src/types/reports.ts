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
}

export const reportTypeConfig: {
    [key: string]: {
        tranlationKey: string
        actions: ReportActions[]
        templateRequired?: boolean
        isElectionRequired?: boolean
    }
} = {
    [EReportType.BALLOT_RECEIPT]: {
        tranlationKey: "Ballot Receipt",
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.GENERATE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: true,
        isElectionRequired: false,
    },
    [EReportType.ELECTORAL_RESULTS]: {
        tranlationKey: "Electoral Results",
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: true,
        isElectionRequired: false,
    },
    [EReportType.MANUAL_VERIFICATION]: {
        tranlationKey: "Manual Verification",
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: true,
        isElectionRequired: false,
    },
}

export enum EGenerateReportMode {
    REAL = "REAL",
    PREVIEW = "PREVIEW",
}
