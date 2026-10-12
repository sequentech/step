// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {IBallotStyle} from "@sequentech/ui-core"
import {GetPublishedBallotStylesQuery} from "./BallotStyles"
import {
    EBallotStyleCheck,
    checkAuditedBallotStyle,
    parsePublishedBallotStyles,
} from "./ballotStyleBinding"

const ballotStyle = (overrides: Partial<IBallotStyle> = {}): IBallotStyle =>
    ({
        id: "style-1",
        election_id: "election-1",
        area_id: "area-1",
        public_key: {public_key: "key-1", is_demo: false},
        contests: [
            {id: "contest-1", candidates: [{id: "cand-1"}, {id: "cand-2"}, {id: "cand-3"}]},
            {id: "contest-2", candidates: [{id: "cand-4"}]},
        ],
        ...overrides,
    }) as unknown as IBallotStyle

describe("checkAuditedBallotStyle", () => {
    it("accepts a ballot style equal to the published one", () => {
        expect(checkAuditedBallotStyle(ballotStyle(), [ballotStyle()])).toBe(
            EBallotStyleCheck.MATCHES
        )
    })

    it("accepts a ballot style equal to any published one with its id", () => {
        const older = ballotStyle({public_key: {public_key: "key-0", is_demo: false}})

        expect(checkAuditedBallotStyle(ballotStyle(), [older, ballotStyle()])).toBe(
            EBallotStyleCheck.MATCHES
        )
    })

    it("rejects a ballot style with another public key", () => {
        const audited = ballotStyle({public_key: {public_key: "forged", is_demo: false}})

        expect(checkAuditedBallotStyle(audited, [ballotStyle()])).toBe(EBallotStyleCheck.MISMATCH)
    })

    it("rejects a ballot style without a public key", () => {
        const audited = ballotStyle({public_key: undefined})

        expect(checkAuditedBallotStyle(audited, [ballotStyle()])).toBe(EBallotStyleCheck.MISMATCH)
    })

    it("rejects a ballot style with reordered candidates", () => {
        const audited = ballotStyle()
        audited.contests[0].candidates.reverse()

        expect(checkAuditedBallotStyle(audited, [ballotStyle()])).toBe(EBallotStyleCheck.MISMATCH)
    })

    it("rejects a ballot style with reordered contests", () => {
        const audited = ballotStyle()
        audited.contests.reverse()

        expect(checkAuditedBallotStyle(audited, [ballotStyle()])).toBe(EBallotStyleCheck.MISMATCH)
    })

    it("rejects a ballot style with another election", () => {
        const audited = ballotStyle({election_id: "election-2"})

        expect(checkAuditedBallotStyle(audited, [ballotStyle()])).toBe(EBallotStyleCheck.MISMATCH)
    })

    it("reports a ballot style that is not published", () => {
        expect(checkAuditedBallotStyle(ballotStyle(), [ballotStyle({id: "style-2"})])).toBe(
            EBallotStyleCheck.NOT_PUBLISHED
        )
        expect(checkAuditedBallotStyle(ballotStyle(), [])).toBe(EBallotStyleCheck.NOT_PUBLISHED)
    })
})

describe("parsePublishedBallotStyles", () => {
    const row = (publicationId: string, eml: string | null) => ({
        ballot_publication_id: publicationId,
        ballot_eml: eml,
    })

    it("reads the ballot styles of published publications only", () => {
        const data = {
            sequent_backend_ballot_publication: [{id: "pub-1", published_at: "2026-01-01"}],
            sequent_backend_ballot_style: [
                row("pub-1", JSON.stringify(ballotStyle())),
                row("pub-2", JSON.stringify(ballotStyle({id: "style-2"}))),
            ],
        } as unknown as GetPublishedBallotStylesQuery

        expect(parsePublishedBallotStyles(data).map((style) => style.id)).toEqual(["style-1"])
    })

    it("leaves out rows that cannot be read and handles missing data", () => {
        const data = {
            sequent_backend_ballot_publication: [{id: "pub-1", published_at: "2026-01-01"}],
            sequent_backend_ballot_style: [row("pub-1", "not json"), row("pub-1", null)],
        } as unknown as GetPublishedBallotStylesQuery

        expect(parsePublishedBallotStyles(data)).toEqual([])
        expect(parsePublishedBallotStyles(undefined)).toEqual([])
    })
})
