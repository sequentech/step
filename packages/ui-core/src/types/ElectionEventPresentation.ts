// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IVotingPortalCountdownPolicy} from "./CoreTypes"
import {ILanguageConf} from "./LanguageConf"

export enum ESupportMaterialsPolicy {
    OFF = "off",
    OPTIONAL = "optional",
    MANDATORY_FOR_VOTING = "mandatory_for_voting",
}

export interface IElectionEventMaterials {
    policy?: ESupportMaterialsPolicy
}

/** Mirrors `ElectionEventMaterials::effective_policy` in `sequent-core`. */
export const getEffectiveSupportMaterialsPolicy = (
    materials?: IElectionEventMaterials
): ESupportMaterialsPolicy => {
    return materials?.policy ?? ESupportMaterialsPolicy.OFF
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

export enum EShowCastVoteLogsPolicy {
    SHOW_LOGS_TAB = "show-logs-tab",
    HIDE_LOGS_TAB = "hide-logs-tab",
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

/** Whether each ballot box is sealed when voting closes (VOTE-FREEZE). Locked once voting has opened. */
export enum EBallotBoxSealPolicy {
    DO_NOT_SEAL = "do-not-seal",
    SEAL_AT_CLOSE = "seal-at-close",
}

/**
 * Who can download a sealed ballot box's seal record (VOTE-FREEZE): Restricted
 * (the default) keeps it a private event document for administrators; Public
 * puts it in the public bucket. Locked once voting has opened on a
 * seal-at-close event.
 */
export enum EBallotBoxSealRecordPolicy {
    RESTRICTED = "restricted",
    PUBLIC = "public",
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

export enum EElectionEventAutomaticRecountPolicy {
    ENABLED = "enabled",
    DISABLED = "disabled",
}

export enum EElectionEventWeightedVotingPolicy {
    AREAS_WEIGHTED_VOTING = "areas-weighted-voting",
    DISABLED_WEIGHTED_VOTING = "disabled-weighted-voting",
    VOTERS_WEIGHTED_VOTING = "voters-weighted-voting",
}

export enum EElectionEventDelegatedVotingPolicy {
    ENABLED = "enabled",
    DISABLED = "disabled",
}

export enum EVoterCertificatePolicy {
    ENABLED = "enabled",
    DISABLED = "disabled",
}

export enum EResultsWebsiteStatus {
    ENABLED = "enabled",
    DISABLED = "disabled",
}

export enum EResultsWebsiteAccess {
    PUBLIC = "public",
    AUTHENTICATED = "authenticated",
}

export enum EResultsWebsiteVisibilityScope {
    FULL_EVENT = "full_event",
    AREA_BASED = "area_based",
}

export enum EResultsRouteScope {
    EVENT = "event",
    ELECTION = "election",
}

export enum EResultsPublicationStatus {
    PUBLISHING = "Publishing",
    PUBLISHED = "Published",
    FAILED = "Failed",
    REVOKED = "Revoked",
    SUPERSEDED = "Superseded",
}

export interface IResultsWebsitePolicy {
    status: EResultsWebsiteStatus
    access: EResultsWebsiteAccess
    visibility_scope: EResultsWebsiteVisibilityScope
}

export const defaultResultsWebsitePolicy = (): IResultsWebsitePolicy => ({
    status: EResultsWebsiteStatus.DISABLED,
    access: EResultsWebsiteAccess.PUBLIC,
    visibility_scope: EResultsWebsiteVisibilityScope.FULL_EVENT,
})

const isRecord = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

export const parseResultsWebsitePolicy = (value: unknown): IResultsWebsitePolicy | undefined => {
    try {
        const parsed: unknown = typeof value === "string" ? JSON.parse(value) : value
        if (!isRecord(parsed)) return undefined

        const {status, access, visibility_scope: visibilityScope} = parsed
        if (status !== EResultsWebsiteStatus.ENABLED && status !== EResultsWebsiteStatus.DISABLED) {
            return undefined
        }
        if (
            access !== EResultsWebsiteAccess.PUBLIC &&
            access !== EResultsWebsiteAccess.AUTHENTICATED
        ) {
            return undefined
        }
        if (
            visibilityScope !== EResultsWebsiteVisibilityScope.FULL_EVENT &&
            visibilityScope !== EResultsWebsiteVisibilityScope.AREA_BASED
        ) {
            return undefined
        }

        return {status, access, visibility_scope: visibilityScope}
    } catch {
        return undefined
    }
}
export enum EVotingPortalDateTimeFormat {
    LEGACY_GB_24H = "legacy-gb-24h",
    ISO_LOCAL = "iso-local",
    US_12H = "us-12h",
    LOCALE_MEDIUM = "locale-medium",
    DATE_ONLY = "date-only",
    CUSTOM = "custom",
}

/**
 * The `CUSTOM` policy carries the operator-supplied pattern inline, mirroring the
 * Rust `VotingPortalDateTimeFormat::Custom(String)` variant. Stored in the same
 * `voting_portal_datetime_format` field as `{custom: "<pattern>"}`; presets remain
 * plain strings.
 */
export interface IVotingPortalCustomDateTimeFormat {
    custom: string
}

export type VotingPortalDateTimeFormat =
    | EVotingPortalDateTimeFormat
    | IVotingPortalCustomDateTimeFormat

export interface IElectionEventPresentation {
    i18n?: Record<string, Record<string, string>>
    materials?: IElectionEventMaterials
    language_conf?: ILanguageConf
    logo_url?: string
    redirect_finish_url?: string
    kiosk_redirect_finish_url?: string
    css?: string
    skip_election_list?: boolean
    show_user_profile?: boolean
    elections_order?: ElectionsOrder
    voting_portal_countdown_policy?: IVotingPortalCountdownPolicy
    custom_urls?: ICustomUrls
    keys_ceremony_policy?: KeysCeremonyPolicy
    locked_down?: EElectionEventLockedDown
    contest_encryption_policy?: EElectionEventContestEncryptionPolicy
    decoded_ballot_inclusion_policy?: EElectionEventDecodedBallots
    publish_policy?: EElectionEventPublishPolicy
    enrollment?: EElectionEventEnrollment
    otp?: EElectionEventOTP
    ceremonies_policy?: EElectionEventCeremoniesPolicy
    automatic_recount_policy?: EElectionEventAutomaticRecountPolicy
    weighted_voting_policy?: EElectionEventWeightedVotingPolicy
    voter_signing_policy?: EVoterSigningPolicy
    voter_certificate_policy?: EVoterCertificatePolicy
    results_website?: string
    delegated_voting_policy: EElectionEventDelegatedVotingPolicy
    voting_portal_datetime_format?: VotingPortalDateTimeFormat
    /** Configured timezones and the primary one (VOTE-LIFECYCLE). */
    timezones?: IElectionEventTimeZones
    /** Lifecycle decisions that are part of the (signed) configuration. */
    lifecycle_policies?: ILifecyclePolicies
    /** Unset means do-not-seal (VOTE-FREEZE). */
    ballot_box_seal_policy?: EBallotBoxSealPolicy
    /** Unset means restricted (VOTE-FREEZE). */
    ballot_box_seal_record_policy?: EBallotBoxSealRecordPolicy
}

/** Which timezone the Logs tab and log exports show. */
export enum ELogTimeZonePolicy {
    PRIMARY = "primary",
    ELECTION = "election",
}

/** IANA zone names in tzdata canonical form (Asia/Kolkata, not Asia/Calcutta). */
export interface IElectionEventTimeZones {
    /** At least one; elections choose theirs from this list. */
    configured: Array<string>
    /** One of `configured`. */
    primary: string
    logs?: ELogTimeZonePolicy
}

export enum EInitializationScope {
    POST = "post",
    EVENT = "event",
    POST_AND_COUNTRY = "post-and-country",
}

export enum EUnsignedScheduledClosePolicy {
    REFUSE = "refuse",
    RUN_AS_SYSTEM = "run-as-system",
}

export interface ILifecyclePolicies {
    initialization_scope?: EInitializationScope
    unsigned_scheduled_close?: EUnsignedScheduledClosePolicy
}
