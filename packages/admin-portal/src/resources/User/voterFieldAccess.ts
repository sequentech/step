// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const MOBILE_PHONE_ATTR_NAME = "sequent.read-only.mobile-number"
export const EMAIL_AND_OR_MOBILE_ATTR_NAME = "emailAndOrMobile"

/**
 * Fields an admin with voter-email-tlf-edit but without voter-write may
 * change. Harvest's edit_user enforces the same set (EMAIL_TLF_ATTRIBUTES in
 * harvest routes/users.rs) and rejects anything else.
 */
export const EMAIL_TLF_FIELDS: ReadonlySet<string> = new Set([
    "email",
    MOBILE_PHONE_ATTR_NAME,
    EMAIL_AND_OR_MOBILE_ATTR_NAME,
])

export interface VoterFieldAccess {
    createMode: boolean
    /** Unset when the form edits a tenant user instead of a voter. */
    electionEventId?: string
    hasVoted: boolean
    /** voter-write */
    canEditVoters: boolean
    /** voter-email-tlf-edit */
    canEditVotersEmailTlf: boolean
    /** voter-voted-edit: lifts the "already voted" lock, grants no field. */
    canEditVotersWhoVoted: boolean
}

/** Whether the voter form lets the current admin change `field`. */
export const canEditVoterField = (access: VoterFieldAccess, field: string): boolean => {
    if (access.createMode || !access.electionEventId) {
        return true
    }
    if (access.hasVoted && !access.canEditVotersWhoVoted) {
        return false
    }
    if (access.canEditVoters) {
        return true
    }
    return access.canEditVotersEmailTlf && EMAIL_TLF_FIELDS.has(field)
}
