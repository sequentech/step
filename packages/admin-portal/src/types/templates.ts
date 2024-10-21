// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum IAudienceSelection {
    ALL_USERS = "ALL_USERS",
    NOT_VOTED = "NOT_VOTED",
    VOTED = "VOTED",
    SELECTED = "SELECTED",
}

export enum ITemplateType {
    CREDENTIALS = "CREDENTIALS",
    BALLOT_RECEIPT = "BALLOT_RECEIPT",
    PARTICIPATION_REPORT = "PARTICIPATION_REPORT",
    ELECTORAL_RESULTS = "ELECTORAL_RESULTS",
    OTP = "OTP",
    TALLY_REPORT = "TALLY_REPORT",
    MANUALLY_VERIFY_VOTER = "MANUALLY_VERIFY_VOTER",
    INITIALIZATION_REPORT = "INITIALIZATION_REPORT",
    STATUS_REPORT = "STATUS_REPORT",
    ELECTION_RETURNS_FOR_NATIONAL_POSITIONS = "ELECTION_RETURNS_FOR_NATIONAL_POSITIONS",
    TRANSMISSION_REPORTS = "TRANSMISSION_REPORTS",
    AUDIT_LOGS = "AUDIT_LOGS",
    OVCS_INFORMATION = "OVCS_INFORMATION",
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

export enum ITemplateMethod {
    EMAIL = "EMAIL",
    SMS = "SMS",
    DOCUMENT = "DOCUMENT",
}

export interface IEmail {
    subject: string
    plaintext_body: string
    html_body: string
}

export interface IEmail {
    subject: string
    plaintext_body: string
    html_body: string
}

export interface ISmsConfig {
    message: string
}

export interface IMethods {
    [ITemplateMethod.EMAIL]: boolean
    [ITemplateMethod.SMS]: boolean
    [ITemplateMethod.DOCUMENT]: boolean
}

export interface ISendTemplateBody {
    audience_selection?: IAudienceSelection
    audience_voter_ids?: Array<string>
    type?: ITemplateType
    communication_method?: ITemplateMethod
    schedule_now?: boolean
    schedule_date?: string
    email?: IEmail
    sms?: ISmsConfig
    name?: string
    alias?: string
    document?: string
    selected_methods?: IMethods
}

export interface IRECEIPTS {
    [key: string]: {
        allowed?: boolean
        template?: string | null
    }
}
