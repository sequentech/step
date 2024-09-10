// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IVotingPortalCountdownPolicy} from "./CoreTypes"
import {ILanguageConf} from "./LanguageConf"

export interface IElectionEventMaterials {
    activated?: boolean
}

export interface ICustomUrls {
    login?: string
    enrollment?: string
}

export enum ElectionsOrder {
    RANDOM = "random",
    CUSTOM = "custom",
    ALPHABETICAL = "alphabetical",
}

export interface IElectionEventPresentation {
    i18n?: Record<string, Record<string, string>>
    materials?: IElectionEventMaterials
    language_conf?: ILanguageConf
    logo_url?: string
    redirect_finish_url?: string
    css?: string
    skip_election_list?: boolean
    show_user_profile?: boolean
    elections_order?: ElectionsOrder
    voting_portal_countdown_policy?: IVotingPortalCountdownPolicy
    custom_urls?: ICustomUrls
}
