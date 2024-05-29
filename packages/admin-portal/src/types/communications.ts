// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum ICommunicationType {
    CREDENTIALS = "CREDENTIALS",
    BALLOT_RECEIPT = "BALLOT_RECEIPT",
    PARTICIPATION_REPORT = "PARTICIPATION_REPORT",
    ELECTORAL_RESULTS = "ELECTORAL_RESULTS",
    OTP = "OTP",
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

export interface ISendCommunicationBody {
    audience_selection: any
    audience_voter_ids?: Array<string>
    communication_type: ICommunicationType
    communication_method: ICommunicationMethod
    schedule_now: boolean
    schedule_date?: string
    email?: IEmail
    sms?: string
    name: string
    alias: string
}

export interface IRECEIPTS {
    [key: string]: {
        allowed?: boolean
        template?: string | null
    }
}
