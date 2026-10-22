/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {
    boardTokenProvider,
    createBrowserTrustee,
    runBrowserCycle,
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
it("gives the board the in-memory access token on each request, never a stored one", async () => {
    // A stale copy in storage must not be used: only the getter counts.
    localStorage.setItem("token", "stored.copy.token:1893456000")
    let current = ""
    const provider = boardTokenProvider(() => current)
    await expect(provider()).rejects.toThrow("Sign in")
    current = "first"
    await expect(provider()).resolves.toBe("first")
    current = "renewed"
    await expect(provider()).resolves.toBe("renewed")
})
it("constructs every browser engine with a provider over the given token", async () => {
    const e = engine()
    const BrowserTrustee = Object.assign(
        jest.fn(() => e),
        {generateKeys: jest.fn()}
    )
    let current = "first"
    expect(createBrowserTrustee({BrowserTrustee}, "alice", "keys", "/b4", () => current)).toBe(e)
    expect(BrowserTrustee).toHaveBeenCalledWith("alice", "keys", "/b4", expect.any(Function))
    const provider = (BrowserTrustee.mock.calls[0] as unknown[])[3] as () => Promise<string>
    await expect(provider()).resolves.toBe("first")
    current = "renewed"
    await expect(provider()).resolves.toBe("renewed")
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
