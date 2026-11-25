/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor, within} from "@testing-library/react"
import {TallyPublicVerification} from "./TallyPublicVerification"
import {loadBrowserTrustee, type PublicVerificationReport} from "@/services/BrowserTrustee"
import type {TallyVerificationBoard, TallyVerifier} from "@/services/TallyPublicVerification"
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
jest.mock("@/services/BrowserTrustee", () => ({
    ...jest.requireActual("@/services/BrowserTrustee"),
    loadBrowserTrustee: jest.fn(),
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

let verifier: jest.Mocked<TallyVerifier>

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
    render(
        <TallyPublicVerification
            electionEventId={EVENT}
            tallySessionId={SESSION}
            finished={finished}
        />
    )
}

beforeEach(() => {
    jest.clearAllMocks()
    verifier = {
        configurationHash: jest.fn((_configuration: string) => "configuration-hash"),
        ballotsHash: jest.fn((_ballots: string) => "ballots-hash"),
        tallyIdentifier: jest.fn((_configuration: string, _ballots: string) => "42"),
        fetchTranscript: jest.fn(),
        verifyTranscript: jest.fn((_evidence: string, _anchors: string) =>
            JSON.stringify(report("verified"))
        ),
    }
    jest.mocked(loadBrowserTrustee).mockResolvedValue({
        WasmVerifier: verifier,
    } as unknown as Awaited<ReturnType<typeof loadBrowserTrustee>>)
    answerBoards = async () => boards([named])
    answerTranscript = async (tallyBoard) => transcript(tallyBoard)
    mockQuery.mockImplementation((options: {query: unknown; variables: {tallyBoard: string}}) =>
        options.query === GET_TALLY_PUBLIC_VERIFICATION_TRANSCRIPT
            ? answerTranscript(options.variables.tallyBoard)
            : answerBoards()
    )
})

it("cannot verify a tally that has not finished", () => {
    renderWidget(false)
    expect((verifyButton() as HTMLButtonElement).disabled).toBe(true)
    expect(screen.getByText(`${KEY}.unavailable`)).toBeTruthy()
})

it("verifies every batch with the tally-results role and lists the checks when all pass", async () => {
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
    const rows = screen.getAllByRole("row").slice(1)
    expect(rows.map((row) => within(row).getAllByRole("cell")[0].textContent)).toEqual([
        `General · Mayor · North${named.tally_board}`,
        `${KEY}.batch {"batch":32}${unnamed.tally_board}`,
    ])
    expect(within(rows[0]).getByText("7")).toBeTruthy()
    expect(within(rows[1]).getByText(`${KEY}.status.verified`)).toBeTruthy()
    expect(screen.getByRole("button", {name: `${KEY}.verifyAgain`})).toBeTruthy()
    expect(screen.queryByRole("status")).toBeNull()
    expect(JSON.parse(verifier.verifyTranscript.mock.calls[0][0])).toEqual({
        version: 1,
        dkg_messages: ["ZGtn"],
        tally_messages: [named.tally_board],
    })
    expect(verifier.fetchTranscript).not.toHaveBeenCalled()
})

it("reports each failed proof or unreadable batch, and the batches still verified", async () => {
    answerBoards = async () => boards([named, unnamed, {...named, tally_board: "third"}])
    answerTranscript = async (tallyBoard) => {
        if (tallyBoard === unnamed.tally_board)
            throw new Error("the board service could not be read")
        return transcript(tallyBoard)
    }
    verifier.verifyTranscript.mockReturnValueOnce(JSON.stringify(report("invalid")))
    renderWidget()

    fireEvent.click(verifyButton())

    expect(await screen.findByText(`${KEY}.summary.invalid {"count":1,"total":3}`)).toBeTruthy()
    expect(screen.queryByText(`${KEY}.checks.title`)).toBeNull()
    const rows = screen.getAllByRole("row").slice(1)
    const cells = rows.map((row) =>
        within(row)
            .getAllByRole("cell")
            .slice(1)
            .map((cell) => cell.textContent)
    )
    expect(cells).toEqual([
        ["7", `${KEY}.status.invalid`, "invalid detail"],
        ["", `${KEY}.status.error`, "the board service could not be read"],
        ["7", `${KEY}.status.verified`, "verified detail"],
    ])
})

it("says when a tally has no boards to verify", async () => {
    answerBoards = async () => boards([])
    renderWidget()
    fireEvent.click(verifyButton())
    expect(await screen.findByText(`${KEY}.empty`)).toBeTruthy()
    expect(loadBrowserTrustee).not.toHaveBeenCalled()
})

it("says why the verification could not run", async () => {
    answerBoards = async () => {
        throw new Error("network down")
    }
    renderWidget()
    fireEvent.click(verifyButton())
    expect(await screen.findByText(`${KEY}.failed {"message":"network down"}`)).toBeTruthy()

    // The browser cannot load the verifier, e.g. without cross-origin isolation.
    answerBoards = async () => boards([named])
    jest.mocked(loadBrowserTrustee).mockRejectedValueOnce("not isolated")
    fireEvent.click(verifyButton())
    expect(await screen.findByText(`${KEY}.failed {"message":"not isolated"}`)).toBeTruthy()
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
