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
}

export interface ISendCommunicationBody {
    audience_selection: any
    audience_voter_ids?: Array<string>
    communication_type: ICommunicationType
    communication_method: ICommunicationMethod
    schedule_now: boolean
    schedule_date?: string
    email?: any
    sms?: any
}

export interface IRECEIPTS {
    [key: string]: {
        allowed?: boolean
        template?: string | null
    }
}