// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ETemplateType} from "./templates"

export enum ReportActions {
    EDIT = "edit",
    DELETE = "delete",
    GENERATE = "generate",
    PREVIEW = "preview",
    GENERATE_SCHEDULED = "generate-scheduled",
}

export enum EReportType {
    INITIALIZATION_REPORT = ETemplateType.INITIALIZATION_REPORT,
    ELECTORAL_RESULTS = ETemplateType.ELECTORAL_RESULTS,
    BALLOT_IMAGES = ETemplateType.BALLOT_IMAGES,
    BALLOT_RECEIPT = ETemplateType.BALLOT_RECEIPT,
    ACTIVITY_LOGS = ETemplateType.ACTIVITY_LOGS,
    MANUAL_VERIFICATION = ETemplateType.MANUAL_VERIFICATION,
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
        associatedTemplateType?: ETemplateType
    }
} = {
    [EReportType.INITIALIZATION_REPORT]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.INITIALIZATION_REPORT,
    },
    [EReportType.ELECTORAL_RESULTS]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.ELECTORAL_RESULTS,
    },
    [EReportType.BALLOT_IMAGES]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.BALLOT_IMAGES,
    },
    [EReportType.BALLOT_RECEIPT]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.BALLOT_RECEIPT,
    },
    [EReportType.ACTIVITY_LOGS]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_NOT_ALLOWED,
        associatedTemplateType: ETemplateType.ACTIVITY_LOGS,
    },
    [EReportType.MANUAL_VERIFICATION]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_NOT_ALLOWED,
        associatedTemplateType: ETemplateType.MANUAL_VERIFICATION,
    },
    default: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
}

export enum EGenerateReportMode {
    REAL = "REAL",
    PREVIEW = "PREVIEW",
}
