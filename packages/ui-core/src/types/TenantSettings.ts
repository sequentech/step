// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ILanguageConf} from "./LanguageConf"

export interface ITenantScheduledEvent {
    id: string | number
    date: string
    name: string
}

export interface ITenantSettings {
    language_conf?: ILanguageConf
    sms?: boolean
    mail?: boolean
    schedules?: Array<ITenantScheduledEvent>
    schedulesIds?: Array<string>
}
