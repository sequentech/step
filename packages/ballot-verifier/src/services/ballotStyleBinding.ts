// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {IBallotStyle} from "@sequentech/ui-core"
import type {GetPublishedBallotStylesQuery} from "./BallotStyles"

/**
 * Version of the auditable ballots whose tracker does not cover the ballot
 * style they carry.
 */
export const LEGACY_BALLOT_VERSION = 2

/**
 * The outcome of comparing the ballot style carried by an auditable ballot
 * with the ballot styles published for the election.
 */
export enum EBallotStyleCheck {
    MATCHES = "MATCHES",
    MISMATCH = "MISMATCH",
    NOT_PUBLISHED = "NOT_PUBLISHED",
}

const layoutOf = (ballotStyle: IBallotStyle): string =>
    JSON.stringify({
        election_id: ballotStyle.election_id,
        area_id: ballotStyle.area_id,
        public_key: ballotStyle.public_key?.public_key ?? null,
        contests: ballotStyle.contests.map((contest) => ({
            id: contest.id,
            candidates: contest.candidates.map((candidate) => candidate.id),
        })),
    })

/**
 * Compares the election, area, public key and the order of contests and
 * candidates of the ballot style carried by an auditable ballot with the
 * published ballot style that has the same id. Only MATCHES lets the ballot
 * through, so the caller fails closed on the other two outcomes.
 */
export const checkAuditedBallotStyle = (
    auditedStyle: IBallotStyle,
    publishedStyles: Array<IBallotStyle>
): EBallotStyleCheck => {
    const sameId = publishedStyles.filter((published) => published.id === auditedStyle.id)
    if (0 === sameId.length) {
        return EBallotStyleCheck.NOT_PUBLISHED
    }
    const auditedLayout = layoutOf(auditedStyle)
    return sameId.some((published) => layoutOf(published) === auditedLayout)
        ? EBallotStyleCheck.MATCHES
        : EBallotStyleCheck.MISMATCH
}

/**
 * The ballot styles of the published ballot publications, read from the
 * ballot_eml of each row. A row that cannot be read is left out, so the ballot
 * style it carried cannot be matched.
 */
export const parsePublishedBallotStyles = (
    data: GetPublishedBallotStylesQuery | undefined
): Array<IBallotStyle> => {
    if (!data) {
        return []
    }
    const publishedIds = new Set(
        data.sequent_backend_ballot_publication.map((publication) => publication.id)
    )
    const styles: Array<IBallotStyle> = []
    for (const row of data.sequent_backend_ballot_style) {
        if (!publishedIds.has(row.ballot_publication_id) || "string" !== typeof row.ballot_eml) {
            continue
        }
        try {
            styles.push(JSON.parse(row.ballot_eml) as IBallotStyle)
        } catch (error) {
            console.log(`Error loading EML: ${error}`)
        }
    }
    return styles
}
