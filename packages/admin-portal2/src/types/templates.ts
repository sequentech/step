// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
    INITIALIZATION_REPORT = "INITIALIZATION_REPORT",
    ELECTORAL_RESULTS = "ELECTORAL_RESULTS",
    BALLOT_IMAGES = "BALLOT_IMAGES",
    BALLOT_RECEIPT = "BALLOT_RECEIPT",
    ACTIVITY_LOGS = "ACTIVITY_LOGS",
    MANUAL_VERIFICATION = "MANUAL_VERIFICATION",
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
