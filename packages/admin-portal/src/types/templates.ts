// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum EIntegrityCheckError {
    IO_ERROR = "io-error",
    HASH_MISSMATCH = "hash-mismatch",
    HASH_COMPUTING_ERROR = "hash-computing-error",
}

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
    TALLY_REPORT = "TALLY_REPORT",
    BALLOT_RECEIPT = "BALLOT_RECEIPT",
    VOTE_RECEIPT = "VOTE_RECEIPT",
    MANUAL_VERIFICATION = "MANUAL_VERIFICATION",
    ACTIVITY_LOGS = "ACTIVITY_LOGS",
    INITIALIZATION_REPORT = "INITIALIZATION_REPORT",
    STATUS = "STATUS",
    ELECTORAL_RESULTS = "ELECTORAL_RESULTS",
    TRANSMISSION_REPORT = "TRANSMISSION_REPORT",
    STATISTICAL_REPORT = "STATISTICAL_REPORT",
    AUDIT_LOGS = "AUDIT_LOGS",
    OVCS_INFORMATION = "OVCS_INFORMATION",
    OV_TURNOUT_PERCENTAGE = "OV_TURNOUT_PERCENTAGE",
    OVCS_EVENTS = "OVCS_EVENTS",
    OVCS_STATISTICS = "OVCS_STATISTICS",
    OV_TURNOUT_PER_ABOARD_STATUS_SEX = "OV_TURNOUT_PER_ABOARD_STATUS_SEX",
    OV_TURNOUT_PER_ABOARD_STATUS_SEX_PERCENTAGE = "OV_TURNOUT_PER_ABOARD_STATUS_SEX_PERCENTAGE",
    LIST_OF_OVERSEAS_VOTERS = "LIST_OF_OVERSEAS_VOTERS",
    OV_PRE_ENROLLED_APPROVED = "OV_PRE_ENROLLED_APPROVED",
    PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION = "PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION",
    PRE_ENROLLED_OV_BUT_DISAPPROVED = "PRE_ENROLLED_OV_BUT_DISAPPROVED",
    OV_NOT_YET_PRE_ENROLLED_LIST = "OV_NOT_YET_PRE_ENROLLED_LIST",
    OV_WITH_VOTING_STATUS = "OV_WITH_VOTING_STATUS",
    OV_WHO_VOTED = "OV_WHO_VOTED",
    OV_NOT_YET_PRE_ENROLLED_NUMBER = "OV_NOT_YET_PRE_ENROLLED_NUMBER",
    BALLOT_IMAGES = "BALLOT_IMAGES",
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

export interface IReportOptions {
    max_items_per_report: number
    max_threads: number
}

export interface IExtraConfig {
    pdf_options?: JSON
    report_options?: IReportOptions
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
