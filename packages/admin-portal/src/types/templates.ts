// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum IAudienceSelection {
    ALL_USERS = "ALL_USERS",
    NOT_VOTED = "NOT_VOTED",
    VOTED = "VOTED",
    SELECTED = "SELECTED",
}

export enum ETemplateType {
    CREDENTIALS = "CREDENTIALS",
    PARTICIPATION_REPORT = "PARTICIPATION_REPORT",
    OTP = "OTP",
    STATUS = "STATUS",
    TALLY_REPORT = "TALLY_REPORT",
    BALLOT_RECEIPT = "BALLOT_RECEIPT",
    ELECTORAL_RESULTS = "ELECTORAL_RESULTS",
    MANUAL_VERIFICATION = "MANUAL_VERIFICATION",
    STATISTICAL_REPORT = "STATISTICAL_REPORT",
    INITIALIZATION_REPORT = "INITIALIZATION_REPORT",
    TRANSMISSION_REPORTS = "TRANSMISSION_REPORTS",
    AUDIT_LOGS = "AUDIT_LOGS",
    ACTIVITY_LOGS = "ACTIVITY_LOGS",
    OVCS_INFORMATION = "OVCS_INFORMATION",
    OVCS_EVENTS = "OVCS_EVENTS",
    OVERSEAS_VOTERS = "OVERSEAS_VOTERS",
    OVERSEAS_VOTERS_TURNOUT = "OVERSEAS_VOTERS_TURNOUT",
    OVERSEAS_VOTING_MONITORING_OVCS_EVENTS = "OVERSEAS_VOTING_MONITORING_OVCS_EVENTS",
    OVCS_STATISTICS = "OVCS_STATISTICS",
    OVERSEAS_VOTING_MONITORING_OVCS_STATISTICS = "OVERSEAS_VOTING_MONITORING_OVCS_STATISTICS",
    OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_AND_SEX = "OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_AND_SEX",
    OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_SEX_AND_WITH_PERCENTAGE = "OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_SEX_AND_WITH_PERCENTAGE",
    OV_USERS = "OV_USERS",
    OV_USERS_WHO_VOTED = "OV_USERS_WHO_VOTED",
    PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION = "PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION",
    PRE_ENROLLED_OV_BUT_DISAPPROVED = "PRE_ENROLLED_OV_BUT_DISAPPROVED",
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
    type?: ETemplateType
    communication_method?: ITemplateMethod
    schedule_now?: boolean
    schedule_date?: string
    name?: string
    alias?: string
    document?: string
    email?: IEmail
    sms?: ISmsConfig
    pdf_options?: IPdfOptions
    selected_methods?: IMethods
}

export interface ICommTemplates {
    email_config?: IEmail
    sms_config?: ISmsConfig
}

export interface IExtraConfig {
    pdf_options?: JSON
    communication_templates?: ICommTemplates
}

export interface IRECEIPTS {
    [key: string]: {
        allowed?: boolean
        template?: string | null
    }
}

export interface IPdfOptions {
    landscape?: boolean
    displayHeaderFooter?: boolean
    printBackground?: boolean
    scale?: number
    paperWidth?: number
    paperHeight?: number
    marginTop?: number
    marginBottom?: number
    marginLeft?: number
    marginRight?: number
    pageRanges?: string
    ignoreInvalidPageRanges?: boolean
    headerTemplate?: string
    footerTemplate?: string
    preferCssPageSize?: boolean
    transferMode?: string
}
