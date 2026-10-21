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
const custodyRequest = {
    tenant_id: "tenant",
    election_event_id: "event",
    keys_ceremony_id: "keys",
    tally_session_id: "active-tally",
    trustee_name: "alice",
    signing_public_key: "signing",
    share_encryption_public_key: "encryption",
}
const action = {id: "action", kind: "CREATE_CUSTODY_PROOF", request: custodyRequest}
afterEach(() => {
    global.fetch = originalFetch
})
it("creates the exact queued tally proof locally while continuing its protocol boards", async () => {
    const e = engine()
    const proof = JSON.stringify({request: custodyRequest, signature: "signed locally"})
    ;(e.attest as jest.Mock).mockReturnValue(proof)
    global.fetch = jest
        .fn()
        .mockResolvedValueOnce({
            ok: true,
            json: async () => ({
                data: {
                    trustee_event_updates: {boards: [board], actions: [action]},
                },
            }),
        })
        .mockResolvedValueOnce({
            ok: true,
            json: async () => ({data: {submit_trustee_action_result: {accepted: true}}}),
        })
    expect(await runBrowserCycle(e, "https://platform", "token", jest.fn())).toEqual([board])
    expect(e.attest).toHaveBeenCalledWith(JSON.stringify(custodyRequest))
    const submission = JSON.parse((global.fetch as jest.Mock).mock.calls[1][1].body)
    expect(submission.query).toContain("submit_trustee_action_result")
    expect(submission.variables).toEqual({
        result: {
            id: "action",
            request: custodyRequest,
            status: "READY",
            custody_proof_json: proof,
        },
    })
    expect(e.advance).toHaveBeenCalledWith(JSON.stringify([board]))
    expect((global.fetch as jest.Mock).mock.calls).toHaveLength(2)
})
it("reports a local key mismatch without authorizing custody or stopping its boards", async () => {
    const e = engine()
    ;(e.attest as jest.Mock).mockImplementation(() => {
        throw new Error("local trustee keys do not match the custody request")
    })
    global.fetch = jest
        .fn()
        .mockImplementation(async (_url: unknown, options?: {body?: string}) => {
            const query: string = JSON.parse(String(options?.body)).query
            return {
                ok: true,
                json: async () => ({
                    data: query.includes("trustee_event_updates")
                        ? {trustee_event_updates: {boards: [board], actions: [action]}}
                        : {submit_trustee_action_result: {accepted: true}},
                }),
            }
        })
    await runBrowserCycle(e, "platform", "token", jest.fn())
    const submitted = JSON.parse((global.fetch as jest.Mock).mock.calls[1][1].body)
    expect(submitted.variables.result).toEqual({
        id: "action",
        request: custodyRequest,
        status: "FAILED",
        detail: "local trustee keys do not match the custody request",
    })
    expect(e.advance).toHaveBeenCalledWith(JSON.stringify([board]))
    expect(global.fetch).toHaveBeenCalledTimes(2)
})
it("skips future action kinds while running assigned boards", async () => {
    const e = engine()
    global.fetch = jest.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
            data: {
                trustee_event_updates: {
                    boards: [board],
                    actions: [{id: "future", kind: "FUTURE_ACTION"}],
                },
            },
        }),
    })
    await runBrowserCycle(e, "platform", "token", jest.fn())
    expect(e.attest).not.toHaveBeenCalled()
    expect(e.advance).toHaveBeenCalledWith(JSON.stringify([board]))
})
it.each([
    {errors: [{message: "result rejected"}]},
    {data: {submit_trustee_action_result: {accepted: false}}},
    {data: {submit_trustee_action_result: null}},
])("does not discard unacknowledged custody work and can retry it", async (result) => {
    const e = engine()
    ;(e.attest as jest.Mock).mockReturnValue("proof")
    const updates = {
        ok: true,
        json: async () => ({
            data: {trustee_event_updates: {boards: [board], actions: [action]}},
        }),
    }
    global.fetch = jest
        .fn()
        .mockResolvedValueOnce(updates)
        .mockResolvedValueOnce({ok: true, json: async () => result})
        .mockResolvedValueOnce(updates)
        .mockResolvedValueOnce({
            ok: true,
            json: async () => ({data: {submit_trustee_action_result: {accepted: true}}}),
        })
    await expect(runBrowserCycle(e, "platform", "token", jest.fn())).rejects.toThrow()
    expect(e.advance).not.toHaveBeenCalled()
    await runBrowserCycle(e, "platform", "token", jest.fn())
    const calls = (global.fetch as jest.Mock).mock.calls
    expect(JSON.parse(calls[1][1].body).variables).toEqual(JSON.parse(calls[3][1].body).variables)
    expect(e.attest).toHaveBeenCalledTimes(2)
    expect(e.advance).toHaveBeenCalledTimes(1)
})
it.each([
    {actions: [{...action, request: {...custodyRequest, tally_session_id: 123}}]},
    {actions: [{...action, request: {...custodyRequest, signing_public_key: null}}]},
    {actions: [{...action, id: null}]},
    {actions: "invalid"},
])("rejects malformed custody actions before signing", async ({actions}) => {
    const e = engine()
    global.fetch = jest.fn().mockResolvedValue({
        ok: true,
        json: async () => ({data: {trustee_event_updates: {boards: [board], actions}}}),
    })
    await expect(runBrowserCycle(e, "platform", "token", jest.fn())).rejects.toThrow()
    expect(e.attest).not.toHaveBeenCalled()
    expect(e.advance).not.toHaveBeenCalled()
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
        json: async () => ({data: {trustee_event_updates: {boards: [board]}}}),
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
        "trustee_event_updates"
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
            json: async () => ({data: {trustee_event_updates: {boards: [board]}}}),
        })
        .mockResolvedValueOnce({ok: false, status: 503})
    await expect(runBrowserCycle(e, "https://platform", "token", jest.fn())).rejects.toThrow("503")
    expect(e.settle).not.toHaveBeenCalled()
    global.fetch = jest
        .fn()
        .mockResolvedValueOnce({
            ok: true,
            json: async () => ({data: {trustee_event_updates: {boards: [board]}}}),
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
            data: {trustee_event_updates: {boards: [{...board, parent: "foreign"}]}},
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
            json: async () => ({data: {trustee_event_updates: {boards: [board]}}}),
        })
        .mockResolvedValueOnce({ok: true, json: async () => result})
    await expect(
        runBrowserCycle(e, "https://platform/v1/graphql", "token", jest.fn())
    ).rejects.toThrow()
    expect(e.settle).not.toHaveBeenCalled()
})
