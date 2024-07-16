// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IVotingPortalCountdownPolicy} from "./CoreTypes"
import {ILanguageConf} from "./LanguageConf"

export interface IElectionEventMaterials {
    activated?: boolean
}

export interface IElectionEventPresentation {
    i18n?: Record<string, Record<string, string>>
    materials?: IElectionEventMaterials
    language_conf?: ILanguageConf
    logo_url?: string
    redirect_finish_url?: string
    css?: string
    hide_audit?: boolean
    skip_election_list?: boolean
    show_user_profile?: boolean
    voting_portal_countdown_policy?: IVotingPortalCountdownPolicy
    cast_vote_confirm?: boolean
}
