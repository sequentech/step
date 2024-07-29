// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ILanguageConf} from "./LanguageConf"

export interface IElectionDates {
    start_date?: string
    end_date?: string
}

export interface IElectionPresentation {
    i18n?: Record<string, Record<string, string>>
    language_conf?: ILanguageConf
    cast_vote_confirm?: boolean
    // more missing
}
