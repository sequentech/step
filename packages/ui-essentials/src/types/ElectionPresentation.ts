// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ILanguageConf} from "./LanguageConf"

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
    // more missing
}
