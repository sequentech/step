// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum IAudienceSelection {
    ALL_USERS = "ALL_USERS",
    NOT_VOTED = "NOT_VOTED",
    VOTED = "VOTED",
    SELECTED = "SELECTED",
}

export enum ICommunicationType {
    CREDENTIALS = "CREDENTIALS",
    BALLOT_RECEIPT = "BALLOT_RECEIPT",
    PARTICIPATION_REPORT = "PARTICIPATION_REPORT",
    ELECTORAL_RESULTS = "ELECTORAL_RESULTS",
    OTP = "OTP",
    TALLY_REPORT = "TALLY_REPORT",
}

export enum ICommunicationMethod {
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

export interface ISendCommunicationBody {
    audience_selection?: IAudienceSelection
    audience_voter_ids?: Array<string>
    type?: ICommunicationType
    communication_method?: ICommunicationMethod
    schedule_now?: boolean
    schedule_date?: string
    email?: IEmail
    sms?: ISmsConfig
    name?: string
    alias?: string
    document?: string
}

export interface IRECEIPTS {
    [key: string]: {
        allowed?: boolean
        template?: string | null
    }
}
