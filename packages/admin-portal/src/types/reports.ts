// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ITemplateType} from "./templates"

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
    OVCS_EVENTS = "OVCS_EVENTS",
    AUDIT_LOGS = "AUDIT_LOGS",
    STATUS = "STATUS",
    OVCS_INFORMATION = "OVCS_INFORMATION",
    OVERSEAS_VOTERS = "OVERSEAS_VOTERS",
    ELECTION_RETURNS_FOR_NATIONAL_POSITIONS = "ELECTION_RETURNS_FOR_NATIONAL_POSITIONS",
    OV_USERS_WHO_VOTED = "OV_USERS_WHO_VOTED",
    OV_USERS = "OV_USERS",
    OVCS_STATISTICS = "OVCS_STATISTICS",
    PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION = "PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION",
    PRE_ENROLLED_OV_BUT_DISAPPROVED = "PRE_ENROLLED_OV_BUT_DISAPPROVED",
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
        associatedTemplateType?: ITemplateType
    }
} = {
    [EReportType.BALLOT_RECEIPT]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW, ReportActions.GENERATE],
        templateRequired: true,
        electionPolicy: EReportElectionPolicy.ELECTION_ALLOWED,
        associatedTemplateType: ITemplateType.BALLOT_RECEIPT,
    },
    [EReportType.ELECTORAL_RESULTS]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW, ReportActions.GENERATE],
        templateRequired: true,
        electionPolicy: EReportElectionPolicy.ELECTION_ALLOWED,
        associatedTemplateType: ITemplateType.ELECTORAL_RESULTS,
    },
    [EReportType.MANUAL_VERIFICATION]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW, ReportActions.GENERATE],
        templateRequired: true,
        electionPolicy: EReportElectionPolicy.ELECTION_NOT_ALLOWED,
        associatedTemplateType: ITemplateType.MANUALLY_VERIFY_VOTER,
    },
    [EReportType.STATISTICAL_REPORT]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW, ReportActions.GENERATE],
        templateRequired: true,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.OVCS_EVENTS]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW, ReportActions.GENERATE],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.AUDIT_LOGS]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW, ReportActions.GENERATE],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.STATUS]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW, ReportActions.GENERATE],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.OVCS_INFORMATION]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW, ReportActions.GENERATE],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.OVERSEAS_VOTERS]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW, ReportActions.GENERATE],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_ALLOWED,
    },
    [EReportType.ELECTION_RETURNS_FOR_NATIONAL_POSITIONS]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW, ReportActions.GENERATE],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.OV_USERS_WHO_VOTED]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW, ReportActions.GENERATE],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.OV_USERS]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW, ReportActions.GENERATE],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.OVCS_STATISTICS]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW, ReportActions.GENERATE],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.PRE_ENROLLED_OV_BUT_DISAPPROVED]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW, ReportActions.GENERATE],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW, ReportActions.GENERATE],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_ALLOWED,
    },
    default: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW, ReportActions.GENERATE],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_ALLOWED,
    },
}

export enum EGenerateReportMode {
    REAL = "REAL",
    PREVIEW = "PREVIEW",
}
