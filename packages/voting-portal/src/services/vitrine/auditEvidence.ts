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
    source: EContactSource
}

/** The election authority's contact for a voter whose audit does not match:
 * the pinned policy's `contact` lines, or else the election's support contact
 * from its settings. The pinned verifier does not parse `contact` lines yet
 * and the portal has no support contact to read, so there is none; this is
 * the one place to read them from when there is. */
export const authorityContact = (_policy: Policy | undefined): IAuthorityContact | undefined =>
    undefined

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
