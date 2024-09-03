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
}

export interface IElectionPresentation {
    i18n?: Record<string, Record<string, string>>
    language_conf?: ILanguageConf
    contests_order?: ContestsOrder
    sort_order?: number
    cast_vote_confirm?: boolean
    audit_button_cfg?: EVotingPortalAuditButtonCfg
    // more missing
}

export enum EVotingPortalAuditButtonCfg {
    SHOW = "show",
    NOT_SHOW = "not_show",
    SHOW_IN_HELP = "show_in_help",
}
