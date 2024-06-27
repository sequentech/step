// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ILanguageConf} from "./LanguageConf"

export enum ContestsOrder {
    RANDOM = "random",
    CUSTOM = "custom",
    ALPHABETICAL = "alphabetical",
}
export interface IElectionDates {
    start_date?: string
    end_date?: string
    scheduled_opening?: boolean
    scheduled_closing?: boolean
}

export interface IElectionPresentation {
    i18n?: Record<string, Record<string, string>>
    dates?: IElectionDates
    language_conf?: ILanguageConf
    contests_order?: ContestsOrder
    sort_order?:number
}
