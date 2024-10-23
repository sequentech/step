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
    ACTIVITY_LOG = "ACTIVITY_LOG",
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
    INITIALIZATION_REPORT = "INITIALIZATION_REPORT",
    STATUS_REPORT = "STATUS_REPORT",
    TRANSMISSION_REPORTS = "TRANSMISSION_REPORTS",
    OVERSEAS_VOTERS_TURNOUT = "OVERSEAS_VOTERS_TURNOUT",
    OVERSEAS_VOTING_MONITORING_OVCS_EVENTS = "OVERSEAS_VOTING_MONITORING_OVCS_EVENTS",
    OVERSEAS_VOTING_MONITORING_OVCS_STATISTICS = "OVERSEAS_VOTING_MONITORING_OVCS_STATISTICS",
    OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_AND_SEX = "OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_AND_SEX",
    OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_SEX_AND_WITH_PERCENTAGE = "OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_SEX_AND_WITH_PERCENTAGE",
    LIST_OF_OV_WHO_PRE_ENROLLED_APPROVED = "LIST_OF_OV_WHO_PRE_ENROLLED_APPROVED",
    LIST_OF_OV_WHO_PRE_ENROLLED_BUT_SUBJECT_FOR_MANUAL_VALIDATION = "LIST_OF_OV_WHO_PRE_ENROLLED_BUT_SUBJECT_FOR_MANUAL_VALIDATION",
    LIST_OF_OV_WHO_PRE_ENROLLED_BUT_DISAPPROVED = "LIST_OF_OV_WHO_PRE_ENROLLED_BUT_DISAPPROVED",
    LIST_OF_OV_WHO_HAVE_NOT_YET_PRE_ENROLLED = "LIST_OF_OV_WHO_HAVE_NOT_YET_PRE_ENROLLED",
    LIST_OF_OVERSEAS_VOTERS_WHO_VOTED = "LIST_OF_OVERSEAS_VOTERS_WHO_VOTED",
    LIST_OF_OVERSEAS_VOTERS_WITH_VOTING_STATUS = "LIST_OF_OVERSEAS_VOTERS_WITH_VOTING_STATUS",
    NUMBER_OF_OV_WHO_HAVE_NOT_YET_PRE_ENROLLED = "NUMBER_OF_OV_WHO_HAVE_NOT_YET_PRE_ENROLLED",
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
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_ALLOWED,
        associatedTemplateType: ITemplateType.BALLOT_RECEIPT,
    },
    [EReportType.ELECTORAL_RESULTS]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_ALLOWED,
        associatedTemplateType: ITemplateType.ELECTORAL_RESULTS,
    },
    [EReportType.MANUAL_VERIFICATION]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_NOT_ALLOWED,
        associatedTemplateType: ITemplateType.MANUALLY_VERIFY_VOTER,
    },
    [EReportType.STATISTICAL_REPORT]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.OVCS_EVENTS]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.AUDIT_LOGS]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.ACTIVITY_LOG]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_NOT_ALLOWED,
    },
    [EReportType.STATUS]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.OVCS_INFORMATION]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.OVERSEAS_VOTERS]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_ALLOWED,
    },
    [EReportType.ELECTION_RETURNS_FOR_NATIONAL_POSITIONS]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.OV_USERS_WHO_VOTED]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.OV_USERS]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.OVCS_STATISTICS]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.PRE_ENROLLED_OV_BUT_DISAPPROVED]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_ALLOWED,
    },
    [EReportType.INITIALIZATION_REPORT]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.STATUS_REPORT]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.TRANSMISSION_REPORTS]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.OVERSEAS_VOTERS_TURNOUT]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.OVERSEAS_VOTING_MONITORING_OVCS_EVENTS]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.OVERSEAS_VOTING_MONITORING_OVCS_STATISTICS]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_AND_SEX]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_SEX_AND_WITH_PERCENTAGE]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.LIST_OF_OV_WHO_PRE_ENROLLED_APPROVED]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.LIST_OF_OV_WHO_PRE_ENROLLED_BUT_SUBJECT_FOR_MANUAL_VALIDATION]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.LIST_OF_OV_WHO_PRE_ENROLLED_BUT_DISAPPROVED]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.LIST_OF_OV_WHO_HAVE_NOT_YET_PRE_ENROLLED]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.LIST_OF_OVERSEAS_VOTERS_WHO_VOTED]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.LIST_OF_OVERSEAS_VOTERS_WITH_VOTING_STATUS]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    [EReportType.NUMBER_OF_OV_WHO_HAVE_NOT_YET_PRE_ENROLLED]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    },
    default: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_ALLOWED,
    },
}

export enum EGenerateReportMode {
    REAL = "REAL",
    PREVIEW = "PREVIEW",
}
