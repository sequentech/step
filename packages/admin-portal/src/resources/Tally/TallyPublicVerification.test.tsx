/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor, within} from "@testing-library/react"
import {TallyPublicVerification} from "./TallyPublicVerification"
import type {PublicVerificationReport, VerificationAnchors} from "@/services/BrowserTrustee"
import type {TallyVerificationBoard} from "@/services/TallyPublicVerification"
import {workerVerifier, type Verifier} from "@/services/VerifierWorker"
import {GET_TALLY_PUBLIC_VERIFICATION_BOARDS} from "@/queries/GetTallyPublicVerificationBoards"
import {GET_TALLY_PUBLIC_VERIFICATION_TRANSCRIPT} from "@/queries/GetTallyPublicVerificationTranscript"

const mockQuery = jest.fn()
jest.mock("@apollo/client", () => ({
    ...jest.requireActual("@apollo/client"),
    useApolloClient: () => ({query: mockQuery}),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: object) => (options ? `${key} ${JSON.stringify(options)}` : key),
    }),
}))
// The English name, as the interface language picks it.
jest.mock("@/hooks/usePresentationName", () => ({
    usePresentationName:
        () => (record?: {presentation?: {i18n?: Record<string, {name?: string}>} | null}) =>
            record?.presentation?.i18n?.en?.name ?? "",
}))
jest.mock("@/services/VerifierWorker", () => ({
    ...jest.requireActual("@/services/VerifierWorker"),
    workerVerifier: jest.fn(),
}))

const EVENT = "e7d6c5b4-a392-4180-9f6e-5d4c3b2a1908"
const SESSION = "5a4b3c2d-1e0f-4a9b-8c7d-6e5f4a3b2c1d"
const KEY = "tally.publicVerification"

const named: TallyVerificationBoard = {
    dkg_board: "dkg_0123456789abcdef0123456789abcdef",
    tally_board: "tally_fedcba9876543210fedcba9876543210_0",
    batch: 0,
    election_presentation: {i18n: {en: {name: "General"}}},
    contest_presentation: {i18n: {en: {name: "Mayor"}}},
    area_name: "North",
    configuration_message: "Y29uZmlndXJhdGlvbg==",
    ballots_message: "bm9ydGg=",
}
const unnamed: TallyVerificationBoard = {
    ...named,
    tally_board: "tally_fedcba9876543210fedcba9876543210_32",
    batch: 32,
    election_presentation: null,
    contest_presentation: null,
    area_name: null,
    ballots_message: "c291dGg=",
}

const report = (status: PublicVerificationReport["status"]): PublicVerificationReport => ({
    status,
    detail: `${status} detail`,
    configuration_hash: "c".repeat(128),
    tally_id: "42",
    public_key_hash: null,
    ballot_count: 7,
    plaintexts: [],
    ballot_inclusions: [],
})

let verifier: jest.Mocked<Verifier>

const boards = (list: TallyVerificationBoard[]) => ({
    data: {get_tally_public_verification_boards: {boards: list}},
})

/** The messages the platform reads for a tally board. */
const transcript = (tallyBoard: string) => ({
    data: {
        get_tally_public_verification_transcript: {
            dkg_messages: ["ZGtn"],
            tally_messages: [tallyBoard],
        },
    },
})

// Each query is answered by its own function, which a test may replace.
let answerBoards: () => Promise<unknown>
let answerTranscript: (tallyBoard: string) => Promise<unknown>

const verifyButton = () => screen.getByRole("button", {name: `${KEY}.verify`})

function renderWidget(finished = true) {
    return render(
        <TallyPublicVerification
            electionEventId={EVENT}
            tallySessionId={SESSION}
            finished={finished}
        />
    )
}

const cells = () =>
    screen
        .getAllByRole("row")
        .slice(1)
        .map((row) =>
            within(row)
                .getAllByRole("cell")
                .map((cell) => cell.textContent)
        )

beforeEach(() => {
    jest.clearAllMocks()
    verifier = {
        ready: jest.fn(async () => undefined),
        anchors: jest.fn(async (_board: TallyVerificationBoard) => ({
            configuration_hash: "configuration-hash",
            tally_id: "42",
            ballots_hash: "ballots-hash",
            ballot_ciphertexts: [],
        })),
        verify: jest.fn(async (_transcript: string, _anchors: VerificationAnchors) =>
            report("verified")
        ),
        close: jest.fn(),
    }
    jest.mocked(workerVerifier).mockReturnValue(verifier)
    answerBoards = async () => boards([named])
    answerTranscript = async (tallyBoard) => transcript(tallyBoard)
    mockQuery.mockImplementation((options: {query: unknown; variables: {tallyBoard: string}}) =>
        options.query === GET_TALLY_PUBLIC_VERIFICATION_TRANSCRIPT
            ? answerTranscript(options.variables.tallyBoard)
            : answerBoards()
    )
})

it("cannot verify a tally that has not finished", () => {
    const {unmount} = renderWidget(false)
    expect((verifyButton() as HTMLButtonElement).disabled).toBe(true)
    expect(screen.getByText(`${KEY}.unavailable`)).toBeTruthy()
    unmount()
    expect(workerVerifier).not.toHaveBeenCalled()
})

it("verifies every batch in a worker, from messages the platform reads, and lists the checks", async () => {
    let answer: (value: unknown) => void = () => undefined
    answerBoards = () => new Promise((resolve) => (answer = resolve))
    let release: () => void = () => undefined
    answerTranscript = (tallyBoard) =>
        tallyBoard === named.tally_board
            ? new Promise((resolve) => (release = () => resolve(transcript(tallyBoard))))
            : Promise.resolve(transcript(tallyBoard))
    renderWidget()

    fireEvent.click(verifyButton())
    expect(screen.getByRole("status").textContent).toBe(`${KEY}.loading`)
    expect((verifyButton() as HTMLButtonElement).disabled).toBe(true)
    expect(mockQuery).toHaveBeenCalledWith({
        query: GET_TALLY_PUBLIC_VERIFICATION_BOARDS,
        variables: {electionEventId: EVENT, tallySessionId: SESSION},
        fetchPolicy: "network-only",
        context: {headers: {"x-hasura-role": "tally-results-read"}},
    })

    answer(boards([named, unnamed]))
    await waitFor(() =>
        expect(screen.getByRole("status").textContent).toBe(
            `${KEY}.progress {"current":1,"total":2}`
        )
    )
    // The worker loads the browser trustees' bundle from this origin.
    expect(workerVerifier).toHaveBeenCalledWith(
        `${window.location.origin}/wbraid-trustee/`,
        expect.any(Number)
    )
    expect(verifier.ready).toHaveBeenCalled()
    // The platform reads the board, uncached; the browser reads no board itself.
    expect(mockQuery).toHaveBeenCalledWith({
        query: GET_TALLY_PUBLIC_VERIFICATION_TRANSCRIPT,
        variables: {electionEventId: EVENT, tallySessionId: SESSION, tallyBoard: named.tally_board},
        fetchPolicy: "no-cache",
        context: {headers: {"x-hasura-role": "tally-results-read"}},
    })
    release()

    expect(await screen.findByText(`${KEY}.summary.verified {"count":2,"total":2}`)).toBeTruthy()
    for (const check of ["ballots", "mixing", "decryption"])
        expect(screen.getByText(`${KEY}.checks.${check}`)).toBeTruthy()
    // An owned batch is named by its owner and numbered; an ownerless one by its number.
    expect(cells()).toEqual([
        [
            `General · Mayor · North${KEY}.batch {"batch":0} · ${named.tally_board}`,
            "7",
            `${KEY}.status.verified`,
            `${KEY}.detail.verified`,
        ],
        [
            `${KEY}.batch {"batch":32}${unnamed.tally_board}`,
            "7",
            `${KEY}.status.verified`,
            `${KEY}.detail.verified`,
        ],
    ])
    expect(screen.getByRole("button", {name: `${KEY}.verifyAgain`})).toBeTruthy()
    expect(screen.queryByRole("status")).toBeNull()
    expect(JSON.parse(verifier.verify.mock.calls[0][0])).toEqual({
        version: 1,
        dkg_messages: ["ZGtn"],
        tally_messages: [named.tally_board],
    })
    // The run's worker is stopped when it ends.
    expect(verifier.close).toHaveBeenCalledTimes(1)
})

it("reports each failed proof, missing publication or unreadable batch", async () => {
    answerBoards = async () =>
        boards([
            named,
            unnamed,
            {...named, tally_board: "third"},
            {...named, tally_board: "fourth"},
        ])
    answerTranscript = async (tallyBoard) => {
        if (tallyBoard === unnamed.tally_board)
            throw new Error("the board service could not be read")
        return transcript(tallyBoard)
    }
    verifier.verify
        .mockResolvedValueOnce(report("invalid"))
        .mockResolvedValueOnce(report("incomplete"))
    renderWidget()

    fireEvent.click(verifyButton())

    expect(await screen.findByText(`${KEY}.summary.invalid {"count":1,"total":4}`)).toBeTruthy()
    expect(screen.queryByText(`${KEY}.checks.title`)).toBeNull()
    expect(cells().map((row) => row.slice(1))).toEqual([
        // The verifier's own reason for a failed proof.
        ["7", `${KEY}.status.invalid`, "invalid detail"],
        ["", `${KEY}.status.error`, "the board service could not be read"],
        ["7", `${KEY}.status.incomplete`, `${KEY}.detail.incomplete`],
        ["7", `${KEY}.status.verified`, `${KEY}.detail.verified`],
    ])
})

it("says when a tally has no boards to verify", async () => {
    answerBoards = async () => boards([])
    renderWidget()
    fireEvent.click(verifyButton())
    expect(await screen.findByText(`${KEY}.empty`)).toBeTruthy()
    expect(workerVerifier).not.toHaveBeenCalled()
})

it("says why the verification could not run, or that the browser cannot verify", async () => {
    answerBoards = async () => {
        throw new Error("network down")
    }
    renderWidget()
    fireEvent.click(verifyButton())
    expect(await screen.findByText(`${KEY}.failed {"message":"network down"}`)).toBeTruthy()

    // The worker cannot load the verifier, e.g. without cross-origin isolation.
    answerBoards = async () => boards([named])
    verifier.ready.mockRejectedValueOnce(new Error("SharedArrayBuffer is not defined"))
    fireEvent.click(verifyButton())
    expect(
        await screen.findByText(
            `${KEY}.verifierUnavailable {"message":"SharedArrayBuffer is not defined"}`
        )
    ).toBeTruthy()
    expect(verifier.anchors).not.toHaveBeenCalled()
    expect(verifier.close).toHaveBeenCalledTimes(1)

    // The WASM bundle rejects with strings.
    verifier.ready.mockRejectedValueOnce("no wasm")
    fireEvent.click(verifyButton())
    expect(await screen.findByText(`${KEY}.verifierUnavailable {"message":"no wasm"}`)).toBeTruthy()
})

it("stops the run and its worker when the section goes away", async () => {
    let release: () => void = () => undefined
    answerBoards = async () => boards([named, unnamed])
    answerTranscript = (tallyBoard) =>
        new Promise((resolve) => (release = () => resolve(transcript(tallyBoard))))
    const {unmount} = renderWidget()
    fireEvent.click(verifyButton())
    await waitFor(() => expect(verifier.anchors).toHaveBeenCalledTimes(1))

    unmount()
    expect(verifier.close).toHaveBeenCalled()
    release()
    // The board in progress finishes; the next one is never started.
    await waitFor(() => expect(verifier.verify).toHaveBeenCalledTimes(1))
    await new Promise((resolve) => setTimeout(resolve, 0))
    expect(verifier.anchors).toHaveBeenCalledTimes(1)
})

it("downloads the run's evidence", async () => {
    const createObjectURL = jest.fn(() => "blob:evidence")
    const revokeObjectURL = jest.fn()
    Object.assign(URL, {createObjectURL, revokeObjectURL})
    const saved: string[] = []
    const click = jest.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (
        this: HTMLAnchorElement
    ) {
        saved.push(`${this.download} ${this.href}`)
    })
    renderWidget()
    fireEvent.click(verifyButton())

    fireEvent.click(await screen.findByRole("button", {name: `${KEY}.download`}))

    expect(saved).toEqual([`tally-${SESSION}-public-verification.json blob:evidence`])
    expect(createObjectURL).toHaveBeenCalledWith(expect.any(Blob))
    await waitFor(() => expect(revokeObjectURL).toHaveBeenCalledWith("blob:evidence"))
    click.mockRestore()
})
