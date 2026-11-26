// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {PublicVerificationReport, VerificationAnchors} from "./BrowserTrustee"
import type {Verifier} from "./VerifierWorker"
import {
    batchStatus,
    evidenceOf,
    overallStatus,
    transcriptOf,
    verifyTallyBoards,
    type BatchVerification,
    type BoardMessages,
    type TallyVerificationBoard,
} from "./TallyPublicVerification"

const board = (batch: number): TallyVerificationBoard => ({
    dkg_board: "dkg_0123456789abcdef0123456789abcdef",
    tally_board: `tally_fedcba9876543210fedcba9876543210_${batch}`,
    batch,
    election_presentation: {i18n: {en: {name: "General"}}},
    contest_presentation: {i18n: {en: {name: "Mayor"}}},
    area_name: `Area ${batch}`,
    configuration_message: "Y29uZmlndXJhdGlvbg==",
    ballots_message: `YmFsbG90cw${batch}=`,
})

const report = (status: PublicVerificationReport["status"]): PublicVerificationReport => ({
    status,
    detail: `${status} detail`,
    configuration_hash: "c".repeat(128),
    tally_id: "42",
    public_key_hash: null,
    ballot_count: 3,
    plaintexts: [],
    ballot_inclusions: [],
})

const verified = (
    batch: number,
    status: PublicVerificationReport["status"]
): BatchVerification => ({
    board: board(batch),
    anchors: {configuration_hash: "c", tally_id: "42", ballots_hash: "b", ballot_ciphertexts: []},
    transcript: JSON.stringify({version: 1, dkg_messages: ["AA=="], tally_messages: null}),
    report: report(status),
})
const failed = (batch: number): BatchVerification => ({board: board(batch), error: "unreachable"})

/** The worker's answers, derived from what it is asked. */
function fakeVerifier(): jest.Mocked<Verifier> {
    return {
        ready: jest.fn(async () => undefined),
        anchors: jest.fn(async (board: TallyVerificationBoard) => ({
            configuration_hash: `hash(${board.configuration_message})`,
            tally_id: `id(${board.ballots_message})`,
            ballots_hash: `ballots(${board.ballots_message})`,
            ballot_ciphertexts: [],
        })),
        verify: jest.fn(async (_transcript: string, _anchors: VerificationAnchors) =>
            report("verified")
        ),
        close: jest.fn(),
    }
}

/** The platform's messages of a board: its DKG board's, then its own. */
const loader = () =>
    jest.fn(
        async (board: TallyVerificationBoard): Promise<BoardMessages> => ({
            dkg_messages: [board.dkg_board],
            tally_messages: [board.tally_board],
        })
    )

describe("statuses", () => {
    it("reads a batch's status from its report, and a batch without one as an error", () => {
        expect(batchStatus(verified(0, "incomplete"))).toBe("incomplete")
        expect(batchStatus(failed(0))).toBe("error")
    })

    it("reports the most serious status: an invalid proof, then an error, then a missing publication", () => {
        expect(overallStatus([verified(0, "verified"), verified(1, "verified")])).toBe("verified")
        expect(overallStatus([verified(0, "verified"), verified(1, "incomplete")])).toBe(
            "incomplete"
        )
        expect(overallStatus([verified(0, "incomplete"), failed(1)])).toBe("error")
        expect(overallStatus([failed(0), verified(1, "invalid"), verified(2, "incomplete")])).toBe(
            "invalid"
        )
    })
})

describe("transcriptOf", () => {
    it("is the public transcript version 1 that verifyTranscript and trustee verify read", () => {
        expect(
            JSON.parse(transcriptOf({dkg_messages: ["ZGtn"], tally_messages: ["dGFsbHk="]}))
        ).toEqual({version: 1, dkg_messages: ["ZGtn"], tally_messages: ["dGFsbHk="]})
    })
})

describe("verifyTallyBoards", () => {
    it("verifies each board in order against its own anchors and reports progress", async () => {
        const verifier = fakeVerifier()
        const load = loader()
        const progress: number[] = []
        const batches = await verifyTallyBoards(verifier, load, [board(0), board(1)], (done) =>
            progress.push(done)
        )
        expect(progress).toEqual([1, 2])
        expect(load.mock.calls).toEqual([[board(0)], [board(1)]])
        const [transcript, anchors] = verifier.verify.mock.calls[1]
        expect(JSON.parse(transcript)).toEqual({
            version: 1,
            dkg_messages: [board(1).dkg_board],
            tally_messages: [board(1).tally_board],
        })
        expect(anchors.tally_id).toBe("id(YmFsbG90cw1=)")
        expect(batches.map(batchStatus)).toEqual(["verified", "verified"])
        expect(batches[0]).toMatchObject({board: board(0), report: report("verified")})
    })

    it("keeps going after a board that cannot be read or whose messages are refused", async () => {
        const verifier = fakeVerifier()
        const load = loader()
        load.mockRejectedValueOnce(new Error("board unreachable"))
        // The WASM verifier rejects with strings.
        verifier.anchors.mockRejectedValueOnce("invalid configuration")
        const batches = await verifyTallyBoards(
            verifier,
            load,
            [board(0), board(1), board(2)],
            () => undefined
        )
        // Board 0's anchors are refused, then board 1's messages cannot be read.
        expect(
            batches.map((batch) => ("error" in batch ? batch.error : batchStatus(batch)))
        ).toEqual(["invalid configuration", "board unreachable", "verified"])
    })

    it("reports only the first line of an error, without the WASM wrapper", async () => {
        const verifier = fakeVerifier()
        verifier.verify
            // An error the WASM verifier rethrows with its stack.
            .mockRejectedValueOnce(
                "JsValue(RuntimeError: unreachable\nRuntimeError: unreachable\n    at protocol_board.wasm (index_bg.wasm:1:2))"
            )
            .mockRejectedValueOnce("JsValue(transcript refused)")
            .mockRejectedValueOnce(new Error("first line\nsecond line"))
        const batches = await verifyTallyBoards(
            verifier,
            loader(),
            [board(0), board(1), board(2)],
            () => undefined
        )
        expect(batches.map((batch) => ("error" in batch ? batch.error : null))).toEqual([
            "RuntimeError: unreachable",
            "transcript refused",
            "first line",
        ])
    })

    it("stops before the next board once the run is cancelled", async () => {
        const verifier = fakeVerifier()
        let cancelled = false
        const batches = await verifyTallyBoards(
            verifier,
            loader(),
            [board(0), board(1), board(2)],
            () => (cancelled = true),
            () => cancelled
        )
        expect(batches.map((batch) => batch.board.batch)).toEqual([0])
        expect(verifier.verify).toHaveBeenCalledTimes(1)
    })
})

describe("evidenceOf", () => {
    it("holds each batch's anchors, report and transcript, or why it has none", () => {
        const evidence = JSON.parse(evidenceOf("session", [verified(0, "verified"), failed(1)]))
        expect(evidence).toEqual({
            tally_session_id: "session",
            batches: [
                {
                    dkg_board: board(0).dkg_board,
                    tally_board: board(0).tally_board,
                    batch: 0,
                    anchors: {
                        configuration_hash: "c",
                        tally_id: "42",
                        ballots_hash: "b",
                        ballot_ciphertexts: [],
                    },
                    report: report("verified"),
                    // The file `trustee verify --transcript` reads, not a string.
                    transcript: {version: 1, dkg_messages: ["AA=="], tally_messages: null},
                },
                {
                    dkg_board: board(1).dkg_board,
                    tally_board: board(1).tally_board,
                    batch: 1,
                    error: "unreachable",
                },
            ],
        })
    })
})
