// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
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
    BALLOT_RECEIPT = ETemplateType.BALLOT_RECEIPT,
    ELECTORAL_RESULTS = ETemplateType.ELECTORAL_RESULTS,
    MANUAL_VERIFICATION = ETemplateType.MANUAL_VERIFICATION,
    STATISTICAL_REPORT = ETemplateType.STATISTICAL_REPORT,
    OVCS_EVENTS = ETemplateType.OVCS_EVENTS,
    AUDIT_LOGS = ETemplateType.AUDIT_LOGS,
    ACTIVITY_LOGS = ETemplateType.ACTIVITY_LOGS,
    STATUS = ETemplateType.STATUS,
    OVCS_INFORMATION = ETemplateType.OVCS_INFORMATION,
    OVERSEAS_VOTERS = ETemplateType.OVERSEAS_VOTERS,
    OV_USERS = ETemplateType.OV_USERS,
    OV_USERS_WHO_VOTED = ETemplateType.OV_USERS_WHO_VOTED,
    OV_USERS_WHO_PRE_ENROLLED = ETemplateType.OV_USERS_WHO_PRE_ENROLLED,
    OVCS_STATISTICS = ETemplateType.OVCS_STATISTICS,
    PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION = ETemplateType.PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION,
    PRE_ENROLLED_OV_BUT_DISAPPROVED = ETemplateType.PRE_ENROLLED_OV_BUT_DISAPPROVED,
    INITIALIZATION = ETemplateType.INITIALIZATION,
    TRANSMISSION_REPORTS = ETemplateType.TRANSMISSION_REPORTS,
    OVERSEAS_VOTERS_TURNOUT = ETemplateType.OVERSEAS_VOTERS_TURNOUT,
    OVERSEAS_VOTERS_TURNOUT_WITH_PERCENTAGE = ETemplateType.OVERSEAS_VOTERS_TURNOUT_WITH_PERCENTAGE,
    OVERSEAS_VOTERS_TURNOUT_BY_POST_PER_COUNTRY = ETemplateType.OVERSEAS_VOTERS_TURNOUT_BY_POST_PER_COUNTRY,
    OVERSEAS_VOTING_MONITORING_OVCS_EVENTS = ETemplateType.OVERSEAS_VOTING_MONITORING_OVCS_EVENTS,
    OVERSEAS_VOTING_MONITORING_OVCS_STATISTICS = ETemplateType.OVERSEAS_VOTING_MONITORING_OVCS_STATISTICS,
    // OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_AND_SEX = ETemplateType.OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_AND_SEX,
    // OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_SEX_AND_WITH_PERCENTAGE = ETemplateType.OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_SEX_AND_WITH_PERCENTAGE,
    LIST_OF_OV_WHO_PRE_ENROLLED_APPROVED = ETemplateType.LIST_OF_OV_WHO_PRE_ENROLLED_APPROVED,
    LIST_OF_OV_WHO_PRE_ENROLLED_BUT_SUBJECT_FOR_MANUAL_VALIDATION = ETemplateType.LIST_OF_OV_WHO_PRE_ENROLLED_BUT_SUBJECT_FOR_MANUAL_VALIDATION,
    LIST_OF_OV_WHO_HAVE_NOT_YET_PRE_ENROLLED = ETemplateType.LIST_OF_OV_WHO_HAVE_NOT_YET_PRE_ENROLLED,
    LIST_OF_OVERSEAS_VOTERS_WITH_VOTING_STATUS = ETemplateType.LIST_OF_OVERSEAS_VOTERS_WITH_VOTING_STATUS,
    NUMBER_OF_OV_WHO_HAVE_NOT_YET_PRE_ENROLLED = ETemplateType.NUMBER_OF_OV_WHO_HAVE_NOT_YET_PRE_ENROLLED,
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
    [EReportType.BALLOT_RECEIPT]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.BALLOT_RECEIPT,
    },
    [EReportType.ELECTORAL_RESULTS]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.ELECTORAL_RESULTS,
    },
    [EReportType.MANUAL_VERIFICATION]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_NOT_ALLOWED,
        associatedTemplateType: ETemplateType.MANUAL_VERIFICATION,
    },
    [EReportType.STATISTICAL_REPORT]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.STATISTICAL_REPORT,
    },
    [EReportType.OVCS_EVENTS]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_ALLOWED,
        associatedTemplateType: ETemplateType.OVCS_EVENTS,
    },
    [EReportType.AUDIT_LOGS]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.AUDIT_LOGS,
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
    [EReportType.STATUS]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.STATUS,
    },
    [EReportType.OVCS_STATISTICS]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.OVCS_STATISTICS,
    },
    [EReportType.OVCS_INFORMATION]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.OVCS_INFORMATION,
    },
    [EReportType.OVERSEAS_VOTERS]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.OVERSEAS_VOTERS,
    },
    [EReportType.OV_USERS_WHO_VOTED]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.OV_USERS_WHO_VOTED,
    },
    [EReportType.OV_USERS]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.OV_USERS,
    },
    [EReportType.OV_USERS_WHO_PRE_ENROLLED]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.OV_USERS_WHO_PRE_ENROLLED,
    },
    [EReportType.PRE_ENROLLED_OV_BUT_DISAPPROVED]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.PRE_ENROLLED_OV_BUT_DISAPPROVED,
    },
    [EReportType.PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION,
    },
    [EReportType.INITIALIZATION]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.INITIALIZATION,
    },
    [EReportType.TRANSMISSION_REPORTS]: {
        actions: [ReportActions.EDIT, ReportActions.DELETE, ReportActions.PREVIEW],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.TRANSMISSION_REPORTS,
    },
    [EReportType.OVERSEAS_VOTERS_TURNOUT]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_ALLOWED,
        associatedTemplateType: ETemplateType.OVERSEAS_VOTERS_TURNOUT,
    },
    [EReportType.OVERSEAS_VOTING_MONITORING_OVCS_EVENTS]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.OVERSEAS_VOTING_MONITORING_OVCS_EVENTS,
    },
    [EReportType.OVERSEAS_VOTING_MONITORING_OVCS_STATISTICS]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.OVERSEAS_VOTING_MONITORING_OVCS_STATISTICS,
    },
    // [EReportType.OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_AND_SEX]: {
    //     actions: [
    //         ReportActions.EDIT,
    //         ReportActions.DELETE,
    //         ReportActions.PREVIEW,
    //         ReportActions.GENERATE,
    //         ReportActions.GENERATE_SCHEDULED,
    //     ],
    //     templateRequired: false,
    //     electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    //     associatedTemplateType: ETemplateType.OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_AND_SEX,
    // },
    // [EReportType.OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_SEX_AND_WITH_PERCENTAGE]: {
    //     actions: [
    //         ReportActions.EDIT,
    //         ReportActions.DELETE,
    //         ReportActions.PREVIEW,
    //         ReportActions.GENERATE,
    //         ReportActions.GENERATE_SCHEDULED,
    //     ],
    //     templateRequired: false,
    //     electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
    //     associatedTemplateType:
    //         ETemplateType.OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_SEX_AND_WITH_PERCENTAGE,
    // },
    [EReportType.LIST_OF_OV_WHO_PRE_ENROLLED_APPROVED]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.LIST_OF_OV_WHO_PRE_ENROLLED_APPROVED,
    },
    [EReportType.LIST_OF_OV_WHO_PRE_ENROLLED_BUT_SUBJECT_FOR_MANUAL_VALIDATION]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType:
            ETemplateType.LIST_OF_OV_WHO_PRE_ENROLLED_BUT_SUBJECT_FOR_MANUAL_VALIDATION,
    },
    [EReportType.LIST_OF_OV_WHO_HAVE_NOT_YET_PRE_ENROLLED]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.LIST_OF_OV_WHO_HAVE_NOT_YET_PRE_ENROLLED,
    },
    [EReportType.LIST_OF_OVERSEAS_VOTERS_WITH_VOTING_STATUS]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.LIST_OF_OVERSEAS_VOTERS_WITH_VOTING_STATUS,
    },
    [EReportType.NUMBER_OF_OV_WHO_HAVE_NOT_YET_PRE_ENROLLED]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_ALLOWED,
        associatedTemplateType: ETemplateType.NUMBER_OF_OV_WHO_HAVE_NOT_YET_PRE_ENROLLED,
    },
    [EReportType.OVERSEAS_VOTERS_TURNOUT_WITH_PERCENTAGE]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.OVERSEAS_VOTERS_TURNOUT_WITH_PERCENTAGE,
    },
    [EReportType.OVERSEAS_VOTERS_TURNOUT_BY_POST_PER_COUNTRY]: {
        actions: [
            ReportActions.EDIT,
            ReportActions.DELETE,
            ReportActions.PREVIEW,
            ReportActions.GENERATE,
            ReportActions.GENERATE_SCHEDULED,
        ],
        templateRequired: false,
        electionPolicy: EReportElectionPolicy.ELECTION_REQUIRED,
        associatedTemplateType: ETemplateType.OVERSEAS_VOTERS_TURNOUT_BY_POST_PER_COUNTRY,
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
