/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {
    boardAccessToken,
    createBrowserTrustee,
    runBrowserCycle,
    storedAccessToken,
    type TrusteeEngine,
} from "./BrowserTrustee"
const board = {name: "dkg_d9792af071b849528aac94bc0fead5f7", kind: "DKG", parent: null}
const engine = (): TrusteeEngine => ({
    publicKeys: jest.fn(),
    attest: jest.fn(),
    backup: jest.fn(),
    restoreBackup: jest.fn(),
    free: jest.fn(),
    advance: jest.fn().mockResolvedValue("[]"),
    settle: jest.fn(),
})
const originalFetch = global.fetch
afterEach(() => {
    global.fetch = originalFetch
    localStorage.clear()
})
it("reads the access token the portal stores with its expiry", async () => {
    expect(storedAccessToken()).toBe("")
    await expect(boardAccessToken()).rejects.toThrow("Sign in")
    localStorage.setItem("token", "header.payload.signature:1893456000")
    expect(storedAccessToken()).toBe("header.payload.signature")
    await expect(boardAccessToken()).resolves.toBe("header.payload.signature")
})
it("constructs every browser engine with the board token provider", () => {
    const e = engine()
    const BrowserTrustee = Object.assign(
        jest.fn(() => e),
        {generateKeys: jest.fn()}
    )
    expect(createBrowserTrustee({BrowserTrustee}, "alice", "keys", "/b4")).toBe(e)
    expect(BrowserTrustee).toHaveBeenCalledWith("alice", "keys", "/b4", boardAccessToken)
})
it("reads a fresh token before each platform request, including after advancing", async () => {
    const e = engine()
    const action = {
        id: "action",
        kind: "CREATE_CUSTODY_PROOF",
        request: {
            tenant_id: "tenant",
            election_event_id: "event",
            keys_ceremony_id: "keys",
            tally_session_id: null,
            trustee_name: "alice",
            signing_public_key: "signing",
            share_encryption_public_key: "encryption",
        },
    }
    // The portal renews the token while the engine signs and advances.
    let current = "first"
    ;(e.attest as jest.Mock).mockImplementation(() => {
        current = "second"
        return "proof"
    })
    ;(e.advance as jest.Mock).mockImplementation(async () => {
        current = "renewed"
        return JSON.stringify([{board: board.name, kind: "HALTED", detail: "invalid message"}])
    })
    global.fetch = jest
        .fn()
        .mockResolvedValueOnce({
            ok: true,
            json: async () => ({
                data: {trustee_event_updates: {boards: [board], actions: [action]}},
            }),
        })
        .mockResolvedValueOnce({
            ok: true,
            json: async () => ({data: {submit_trustee_action_result: {accepted: true}}}),
        })
        .mockResolvedValueOnce({
            ok: true,
            json: async () => ({data: {report_trustee_board: {accepted: true}}}),
        })
    const token = jest.fn(() => current)
    await runBrowserCycle(e, "https://platform", token, jest.fn())
    const calls = (global.fetch as jest.Mock).mock.calls
    expect(calls.map((call) => call[1].headers.Authorization)).toEqual([
        "Bearer first",
        "Bearer second",
        "Bearer renewed",
    ])
    expect(JSON.parse(calls[2][1].body).query).toContain("report_trustee_board")
    expect(token).toHaveBeenCalledTimes(3)
    expect(e.settle).toHaveBeenCalledWith(board.name)
})
