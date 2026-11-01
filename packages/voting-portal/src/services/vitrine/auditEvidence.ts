// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What a voter has to report when an audited ballot decodes to something
// other than what they chose (design board AuditCompare): the evidence file,
// who to report it to, and the email. Nothing here is sent anywhere: the file
// is saved on the voter's device and the email is the voter's own.

import type {IDecodedVoteContest} from "@sequentech/ui-core"
import type {Policy} from "@vitrine/verify"
import type {TAuditedRecord} from "./ballotAudit"
import {ballotIdChars, groupBallotId, SHORT_BALLOT_ID_LENGTH} from "./ballotId"

export const EVIDENCE_MEDIA_TYPE = "application/json"

const EVIDENCE_TYPE = "ballot-audit-evidence"

/** What the file says about itself, for whoever opens it. */
const EVIDENCE_ABOUT =
    "Evidence from a ballot audit. record is the audited ballot as the election's public record holds it: " +
    "each contest in contests carries its ciphertext and the secret (plaintext and randomness) that the " +
    "voting portal revealed when the voter chose to audit. decoded is what the voter's browser read from it."

export interface IAuditEvidenceInput {
    /** The audited ballot's full Tracking ID. */
    ballotId: string
    /** The audited ballot, as the record holds it. */
    record: TAuditedRecord
    /** What the browser decoded the ballot into. */
    selections: IDecodedVoteContest[]
}

/** The evidence as a JSON file's text: the encrypted ballot and the secret
 * the portal revealed, exactly as the record has them, and the choices they
 * decode to. */
export const auditEvidence = ({ballotId, record, selections}: IAuditEvidenceInput): string =>
    JSON.stringify(
        {
            type: EVIDENCE_TYPE,
            about: EVIDENCE_ABOUT,
            trackingId: groupBallotId(ballotId),
            record: record.ballot,
            decoded: selections,
        },
        null,
        2
    )

/** A file name for the evidence, from the short Tracking ID:
 * `ballot-audit-FTBEMHRX.json`. */
export const evidenceFileName = (ballotId: string): string => {
    const short = ballotIdChars(ballotId).slice(0, SHORT_BALLOT_ID_LENGTH)
    return `ballot-audit${short ? `-${short}` : ""}.json`
}

/** Where a contact comes from: the election's published policy, which a
 * dishonest portal cannot swap, or the election's settings, which the portal
 * serves and the page must not present as the policy's. */
export enum EContactSource {
    POLICY = "policy",
    SETTINGS = "settings",
}

export interface IAuthorityContact {
    /** The office to report to. */
    name?: string
    email?: string
    phone?: string
    /** A page of the authority's, shown as text. */
    url?: string
    source: EContactSource
}

/** A contact as the policy or the election's settings give it. */
export type TContactDetails = Omit<IAuthorityContact, "source">

const hasDetails = (contact: TContactDetails | undefined): contact is TContactDetails =>
    contact !== undefined && [contact.name, contact.email, contact.phone, contact.url].some(Boolean)

const detailsOf = ({name, email, phone, url}: TContactDetails): TContactDetails => ({
    ...(name ? {name} : {}),
    ...(email ? {email} : {}),
    ...(phone ? {phone} : {}),
    ...(url ? {url} : {}),
})

/** The election authority's contact for a voter whose audit does not match:
 * the pinned policy's `contact` lines, which a dishonest portal cannot swap,
 * or else the election's support contact from its settings, which the page
 * labels as the settings'. Step's election event has no support contact to
 * read yet, so no caller passes `settings` and an election whose policy has
 * no `contact` line gets no contact; the page then says to report it to the
 * authority. */
export const authorityContact = (
    policy: Policy | undefined,
    settings?: TContactDetails
): IAuthorityContact | undefined => {
    if (hasDetails(policy?.contact)) {
        return {...detailsOf(policy.contact), source: EContactSource.POLICY}
    }
    if (hasDetails(settings)) return {...detailsOf(settings), source: EContactSource.SETTINGS}
    return undefined
}

/** One address, without the characters that would add recipients, fields or a
 * fragment to a mailto link. */
const ADDRESS = /^[^\s@,;<>?&#%]+@[^\s@,;<>?&#%]+$/

/** A `mailto:` link to the contact's address, or none if it has no address
 * that is safe to put in one. */
export const mailtoOf = (
    contact: IAuthorityContact,
    subject: string,
    body: string
): string | undefined => {
    if (!contact.email || !ADDRESS.test(contact.email)) return undefined
    return `mailto:${contact.email}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`
}
