// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export interface IElectionLanguageConf {
    enabled_language_codes?: Array<string>
    default_language_code: string
}

export interface IElectionPresentation {
    i18n?: Record<string, Record<string, string>>
    language_conf?: IElectionLanguageConf
    // more missing
}
