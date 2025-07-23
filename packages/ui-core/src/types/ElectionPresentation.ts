// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ILanguageConf} from "./LanguageConf"

export enum ContestsOrder {
    RANDOM = "random",
    CUSTOM = "custom",
    ALPHABETICAL = "alphabetical",
}

export enum EVotingPeriodEnd {
    ALLOWED = "allowed",
    DISALLOWED = "disallowed",
}

export enum ECastVoteGoldLevelPolicy {
    GOLD_LEVEL = "gold-level",
    NO_GOLD_LEVEL = "no-gold-level",
}

export enum EStartScreenTitlePolicy {
    ELECTION = "election",
    ELECTION_EVENT = "election-event",
}

export interface IScheduledEventDates {
    scheduled_at?: string
    stopped_at?: string
}

export interface IElectionDates {
    first_started_at?: string
    last_started_at?: string
    first_paused_at?: string
    last_paused_at?: string
    first_stopped_at?: string
    last_stopped_at?: string
    scheduled_event_dates?: Record<string, IScheduledEventDates>
}

export interface IElectionPresentation {
    i18n?: Record<string, Record<string, string>>
    language_conf?: ILanguageConf
    contests_order?: ContestsOrder
    sort_order?: number
    cast_vote_confirm?: boolean
    cast_vote_gold_level?: ECastVoteGoldLevelPolicy
    audit_button_cfg?: EVotingPortalAuditButtonCfg
    is_grace_priod?: boolean
    grace_period_policy?: EGracePeriodPolicy
    grace_period_secs?: number
    initialization_report_generated?: EInitializeReportPolicy
    voting_period_end?: EVotingPeriodEnd
    // more missing
}

export enum EVotingPortalAuditButtonCfg {
    SHOW = "show",
    NOT_SHOW = "not-show",
    SHOW_IN_HELP = "show-in-help",
}

export enum EGracePeriodPolicy {
    NO_GRACE_PERIOD = "no-grace-period",
    GRACE_PERIOD_WITHOUT_ALERT = "grace-period-without-alert",
}

export enum EInitializeReportPolicy {
    REQUIRED = "required",
    NOT_REQUIRED = "not-required",
}
