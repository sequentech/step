import {Identifier} from "react-admin"

export type TTenant = {
    setting: ITenantSettings
    voting_channels: {
        online: boolean
        kiosk: boolean
    }
}

export interface ITenantScheduledEvent {
    id: Identifier
    date: string
    name: string
}

export interface ITenantSettings {
    spanish?: boolean
    english?: boolean
    sms?: boolean
    mail?: boolean
    schedules?: Array<ITenantScheduledEvent>
}

export type TVotingSetting = {
    [key: string]: boolean
}
