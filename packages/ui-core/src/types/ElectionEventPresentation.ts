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
    saml?: string
}

export enum EVoterSigningPolicy {
    NO_SIGNATURE = "no-signature",
    WITH_SIGNATURE = "with-signature",
}

export enum ElectionsOrder {
    RANDOM = "random",
    CUSTOM = "custom",
    ALPHABETICAL = "alphabetical",
}

export enum KeysCeremonyPolicy {
    ELECTION_EVENT,
    ELECTION,
}

export interface IActiveTemplateIds {
    manual_verification?: string
}

export enum EElectionEventLockedDown {
    LOCKED_DOWN = "locked-down",
    NOT_LOCKED_DOWN = "not-locked-down",
}

export enum EElectionEventDecodedBallots {
    INCLUDED = "included",
    NOT_INCLUDED = "not-included",
}

export enum EElectionEventContestEncryptionPolicy {
    MULTIPLE_CONTESTS = "multiple-contests",
    SINGLE_CONTEST = "single-contest",
}

export enum EElectionEventPublishPolicy {
    ALWAYS = "always",
    AFTER_LOCKDOWN = "after-lockdown",
}

export enum EElectionEventEnrollment {
    ENABLED = "enabled",
    DISABLED = "disabled",
}

export enum EElectionEventOTP {
    ENABLED = "enabled",
    DISABLED = "disabled",
}

export enum EElectionEventCeremoniesPolicy {
    MANUAL_CEREMONIES = "manual-ceremonies",
    AUTOMATED_CEREMONIES = "automated-ceremonies",
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
    keys_ceremony_policy?: KeysCeremonyPolicy
    locked_down: EElectionEventLockedDown
    contest_encryption_policy: EElectionEventContestEncryptionPolicy
    publish_policy: EElectionEventPublishPolicy
    enrollment: EElectionEventEnrollment
    otp: EElectionEventOTP
    ceremonies_policy: EElectionEventCeremoniesPolicy
}
