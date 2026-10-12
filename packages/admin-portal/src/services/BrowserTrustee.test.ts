// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {parseBoards, runBrowserCycle, type TrusteeEngine} from "./BrowserTrustee"
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
})
it.each([
    {boards: [{...board, name: "../board"}]},
    {boards: [{...board, parent: board.name}]},
    {boards: [{...board, kind: "TALLY"}]},
    {boards: [{name: "tally_0f6a3b2c9d8e4f10a1b2c3d4e5f60718_3", kind: "TALLY", parent: null}]},
    {},
])("rejects malformed or mismatched board hints before joining", (value) => {
    expect(() => parseBoards(value)).toThrow("Invalid trustee board response")
})
it("discovers authorized boards and preserves their DKG parents", async () => {
    const e = engine(),
        remember = jest.fn()
    global.fetch = jest.fn().mockResolvedValue({ok: true, json: async () => ({boards: [board]})})
    expect(await runBrowserCycle(e, "https://platform/", "issued-token", remember)).toEqual([board])
    expect(global.fetch).toHaveBeenCalledWith("https://platform/trustee/boards", {
        headers: {Authorization: "Bearer issued-token"},
    })
    expect(remember).toHaveBeenCalledWith([board])
    expect(e.advance).toHaveBeenCalledWith(JSON.stringify([board]))
})
it("keeps halt reports pending until the platform records them", async () => {
    const e = engine(),
        report = {board: board.name, kind: "HALTED", detail: "rewritten board"}
    ;(e.advance as jest.Mock).mockResolvedValue(JSON.stringify([report]))
    global.fetch = jest
        .fn()
        .mockResolvedValueOnce({ok: true, json: async () => ({boards: [board]})})
        .mockResolvedValueOnce({ok: false, status: 503})
    await expect(runBrowserCycle(e, "https://platform", "token", jest.fn())).rejects.toThrow("503")
    expect(e.settle).not.toHaveBeenCalled()
    global.fetch = jest
        .fn()
        .mockResolvedValueOnce({ok: true, json: async () => ({boards: [board]})})
        .mockResolvedValueOnce({ok: true})
    await runBrowserCycle(e, "https://platform", "token", jest.fn())
    expect(e.settle).toHaveBeenCalledWith(board.name)
})
it("does not run without authentication or with untrusted malformed hints", async () => {
    const e = engine()
    global.fetch = jest.fn().mockResolvedValue({
        ok: true,
        json: async () => ({boards: [{...board, parent: "foreign"}]}),
    })
    await expect(runBrowserCycle(e, "url", "", jest.fn())).rejects.toThrow("Sign in")
    expect(global.fetch).not.toHaveBeenCalled()
    await expect(runBrowserCycle(e, "url", "token", jest.fn())).rejects.toThrow(
        "Invalid trustee board response"
    )
    expect(e.advance).not.toHaveBeenCalled()
})
