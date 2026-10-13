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
    global.fetch = jest.fn().mockResolvedValue({
        ok: true,
        json: async () => ({data: {list_trustee_boards: {boards: [board]}}}),
    })
    expect(
        await runBrowserCycle(e, "https://platform/v1/graphql", "issued-token", remember)
    ).toEqual([board])
    expect(global.fetch).toHaveBeenCalledWith("https://platform/v1/graphql", {
        method: "POST",
        headers: {
            "Authorization": "Bearer issued-token",
            "Content-Type": "application/json",
            "x-hasura-role": "trustee-ceremony",
        },
        body: expect.any(String),
    })
    expect(JSON.parse((global.fetch as jest.Mock).mock.calls[0][1].body).query).toContain(
        "list_trustee_boards"
    )
    expect(remember).toHaveBeenCalledWith([board])
    expect(e.advance).toHaveBeenCalledWith(JSON.stringify([board]))
})
it("keeps halt reports pending until the platform records them", async () => {
    const e = engine(),
        report = {board: board.name, kind: "HALTED", detail: "rewritten board"}
    ;(e.advance as jest.Mock).mockResolvedValue(JSON.stringify([report]))
    global.fetch = jest
        .fn()
        .mockResolvedValueOnce({
            ok: true,
            json: async () => ({data: {list_trustee_boards: {boards: [board]}}}),
        })
        .mockResolvedValueOnce({ok: false, status: 503})
    await expect(runBrowserCycle(e, "https://platform", "token", jest.fn())).rejects.toThrow("503")
    expect(e.settle).not.toHaveBeenCalled()
    global.fetch = jest
        .fn()
        .mockResolvedValueOnce({
            ok: true,
            json: async () => ({data: {list_trustee_boards: {boards: [board]}}}),
        })
        .mockResolvedValueOnce({
            ok: true,
            json: async () => ({data: {report_trustee_board: {accepted: true}}}),
        })
    await runBrowserCycle(e, "https://platform", "token", jest.fn())
    expect(e.settle).toHaveBeenCalledWith(board.name)
    const request = JSON.parse((global.fetch as jest.Mock).mock.calls[1][1].body)
    expect(request.query).toContain("report_trustee_board")
    expect(request.variables).toEqual({report})
})
it("does not run without authentication or with untrusted malformed hints", async () => {
    const e = engine()
    global.fetch = jest.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
            data: {list_trustee_boards: {boards: [{...board, parent: "foreign"}]}},
        }),
    })
    await expect(runBrowserCycle(e, "url", "", jest.fn())).rejects.toThrow("Sign in")
    expect(global.fetch).not.toHaveBeenCalled()
    await expect(runBrowserCycle(e, "url", "token", jest.fn())).rejects.toThrow(
        "Invalid trustee board response"
    )
    expect(e.advance).not.toHaveBeenCalled()
})

it("does not advance when Hasura returns an authorization error over HTTP 200", async () => {
    const e = engine()
    global.fetch = jest.fn().mockResolvedValue({
        ok: true,
        json: async () => ({errors: [{message: "permission denied"}]}),
    })
    await expect(
        runBrowserCycle(e, "https://platform/v1/graphql", "token", jest.fn())
    ).rejects.toThrow("permission denied")
    expect(e.advance).not.toHaveBeenCalled()
})

it.each([
    {errors: [{message: "report rejected"}]},
    {data: {report_trustee_board: {accepted: false}}},
    {data: {report_trustee_board: null}},
])("retains a halt report when the GraphQL mutation does not acknowledge it", async (result) => {
    const e = engine()
    ;(e.advance as jest.Mock).mockResolvedValue(
        JSON.stringify([{board: board.name, kind: "HALTED", detail: "invalid message"}])
    )
    global.fetch = jest
        .fn()
        .mockResolvedValueOnce({
            ok: true,
            json: async () => ({data: {list_trustee_boards: {boards: [board]}}}),
        })
        .mockResolvedValueOnce({ok: true, json: async () => result})
    await expect(
        runBrowserCycle(e, "https://platform/v1/graphql", "token", jest.fn())
    ).rejects.toThrow()
    expect(e.settle).not.toHaveBeenCalled()
})
