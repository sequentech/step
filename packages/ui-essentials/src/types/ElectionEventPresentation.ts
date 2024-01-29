// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export interface IElectionEventMaterials {
    activated?: boolean
}

export interface IElectionEventLanguageConf {
    enabled_language_codes?: Array<string>
}

export interface IElectionEventPresentation {
    i18n?: Record<string, string>
    materials?: IElectionEventMaterials
    language_conf?: IElectionEventLanguageConf
    logo_url?: string
    redirect_finish_url?: string
}
