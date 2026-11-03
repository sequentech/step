/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import {TrusteeDashboard} from "./TrusteeDashboard"
import {
    listBrowserTrusteeBoards,
    loadBrowserTrustee,
    type TrusteeEngine,
} from "@/services/BrowserTrustee"

let mockToken: string | undefined
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        trustee: "trustee1",
        username: "trustee1",
        getAccessToken: () => mockToken,
    }),
}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {HASURA_URL: "https://platform"},
    }),
}))
jest.mock("@/services/BrowserTrustee", () => ({
    ...jest.requireActual("@/services/BrowserTrustee"),
    loadBrowserTrustee: jest.fn(),
    listBrowserTrusteeBoards: jest.fn(),
}))

const assigned = [
    {name: "tally", kind: "TALLY" as const, parent: "dkg"},
    {name: "dkg", kind: "DKG" as const, parent: null},
]
let engine: jest.Mocked<TrusteeEngine>
const construct = jest.fn()
const catalogKey = "sequent-wbraid-catalog:/b4:signing-key"
const history = (stores: unknown[] = []) => JSON.stringify({board_url: "/b4", stores})
const generate = () => fireEvent.click(screen.getByRole("button", {name: "Generate browser keys"}))
const refreshBoards = () => fireEvent.click(screen.getByRole("button", {name: "Refresh boards"}))

beforeEach(() => {
    localStorage.clear()
    jest.clearAllMocks()
    mockToken = "current-token"
    engine = {
        publicKeys: jest.fn().mockReturnValue(
            JSON.stringify({
                signing_public_key: "signing-key",
                share_encryption_public_key: "encryption-key",
            })
        ),
        attest: jest.fn(),
        advance: jest.fn(),
        settle: jest.fn(),
        backup: jest.fn(),
        sessionHistory: jest.fn().mockResolvedValue(history()),
        restoreBackup: jest.fn(),
        free: jest.fn(),
    }
    construct.mockReturnValue(engine)
    jest.mocked(loadBrowserTrustee).mockResolvedValue({
        BrowserTrustee: Object.assign(construct, {generateKeys: () => "private keys"}),
    } as unknown as Awaited<ReturnType<typeof loadBrowserTrustee>>)
    jest.mocked(listBrowserTrusteeBoards).mockResolvedValue([])
})

afterEach(() => localStorage.clear())

it("rejects signed-out refreshes and reads the current in-memory token for every refresh", async () => {
    localStorage.setItem("token", "stale-token")
    mockToken = undefined
    render(<TrusteeDashboard />)
    refreshBoards()
    expect(await screen.findByRole("alert")).toHaveProperty(
        "textContent",
        "Sign in as a trustee to list assigned boards."
    )
    expect(listBrowserTrusteeBoards).not.toHaveBeenCalled()
    mockToken = "first-token"
    refreshBoards()
    await screen.findByText("No boards are currently assigned.")
    mockToken = "renewed-token"
    refreshBoards()
    await waitFor(() => expect(listBrowserTrusteeBoards).toHaveBeenCalledTimes(2))
    expect(listBrowserTrusteeBoards).toHaveBeenNthCalledWith(1, "https://platform", "first-token")
    expect(listBrowserTrusteeBoards).toHaveBeenNthCalledWith(2, "https://platform", "renewed-token")
})

it("reads saved and assigned sessions once, warns about missing parent history, and clears keys on lock", async () => {
    localStorage.setItem(catalogKey, JSON.stringify(["saved", "dkg"]))
    jest.mocked(listBrowserTrusteeBoards).mockResolvedValue(assigned)
    engine.sessionHistory.mockResolvedValue(
        history([
            {board: "tally", database: "tally-db", predicates: 3, own_posts: 1},
            {board: "dkg", database: "parent-db", predicates: 0, own_posts: 0},
        ])
    )
    render(<TrusteeDashboard />)
    refreshBoards()
    await screen.findByText("TALLY · Parent: dkg")
    generate()
    await screen.findByRole("button", {name: "Session entries (2)"})
    expect(engine.sessionHistory).toHaveBeenCalledWith(JSON.stringify(["saved", "dkg", "tally"]))
    expect(construct).toHaveBeenCalledWith("trustee1", "private keys", "/b4", expect.any(Function))
    const tokenProvider = construct.mock.calls[0][3] as () => Promise<string>
    mockToken = "renewed-token"
    await expect(tokenProvider()).resolves.toBe("renewed-token")
    fireEvent.click(screen.getByRole("button", {name: "Session entries (2)"}))
    await screen.findByText("tally-db")
    expect(screen.getByText("History available · 3 records · 1 own posts")).toBeTruthy()
    expect(screen.getByText(/Parent DKG history is empty/)).toBeTruthy()
    fireEvent.click(screen.getByRole("button", {name: "Lock keys"}))
    expect(engine.free).toHaveBeenCalledTimes(1)
    expect(screen.queryByRole("textbox", {name: "Signing public key"})).toBeNull()
    expect(screen.queryByText("tally-db")).toBeNull()
})

it.each([
    {kind: "Error", error: new Error("unreadable session")},
    {kind: "string", error: "unreadable session"},
])(
    "keeps loaded keys after a history read failure and allows a successful retry ($kind)",
    async ({error}) => {
        engine.sessionHistory.mockRejectedValueOnce(error)
        render(<TrusteeDashboard />)
        generate()
        expect(await screen.findByRole("alert")).toHaveProperty(
            "textContent",
            "Unable to read local history: unreadable session"
        )
        expect(screen.getByRole("textbox", {name: "Signing public key"})).toHaveProperty(
            "value",
            "signing-key"
        )
        fireEvent.click(screen.getByRole("button", {name: "Refresh local history"}))
        await screen.findByText(/No known sessions/)
        expect(screen.queryByRole("alert")).toBeNull()
        expect(engine.sessionHistory).toHaveBeenCalledTimes(2)
    }
)

it("refreshes assigned history on the loaded engine without reconstructing it", async () => {
    render(<TrusteeDashboard />)
    generate()
    await screen.findByText(/No known sessions/)
    jest.mocked(listBrowserTrusteeBoards).mockResolvedValue(assigned)
    refreshBoards()
    await waitFor(() => expect(engine.sessionHistory).toHaveBeenCalledTimes(2))
    expect(engine.sessionHistory).toHaveBeenLastCalledWith(JSON.stringify(["tally", "dkg"]))
    expect(construct).toHaveBeenCalledTimes(1)
})
