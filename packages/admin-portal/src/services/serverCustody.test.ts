// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {resolveCustodyFile} from "./serverCustody"

const file = JSON.stringify({
    tenant_id: "tenant",
    election_event_id: "event",
    keys_ceremony_id: "keys",
    tally_session_id: null,
    trustee_name: "alice",
    signing_public_key: "signing",
    share_encryption_public_key: "encryption",
})
const scope = {election_event_id: "event", tally_session_id: "current-tally"}

test("uploading the saved ceremony file waits for a fresh server proof for this tally", async () => {
    const request = jest
        .fn()
        .mockResolvedValueOnce({status: "PENDING"})
        .mockResolvedValueOnce({status: "READY", custody_proof_json: "signed proof"})
    expect(await resolveCustodyFile(file, scope, request, async () => undefined)).toBe(
        "signed proof"
    )
    expect(request.mock.calls[0][0]).toEqual({
        ...scope,
        custody_request_json: file,
        retry_policy: "RETRY_FAILED",
    })
    expect(request.mock.calls[1][0]).toEqual({
        ...scope,
        custody_request_json: file,
        retry_policy: "KEEP",
    })
})

test("existing offline custody proofs go directly to the signing flow", async () => {
    const proof = JSON.stringify({request: JSON.parse(file), signature: "signature"})
    const request = jest.fn()
    expect(await resolveCustodyFile(proof, scope, request)).toBe(proof)
    expect(request).not.toHaveBeenCalled()
})

test("server failures and incomplete ready replies cannot confirm custody", async () => {
    for (const reply of [{status: "FAILED", detail: "Keys do not match"}, {status: "READY"}]) {
        const request = jest.fn().mockResolvedValue(reply)
        await expect(resolveCustodyFile(file, scope, request)).rejects.toThrow()
    }
})

test("an offline server leaves a pending request and asks the user to retry", async () => {
    const request = jest.fn().mockResolvedValue({status: "PENDING"})
    await expect(
        resolveCustodyFile(file, scope, request, async () => undefined, 2)
    ).rejects.toThrow("server")
    expect(request).toHaveBeenCalledTimes(2)
})

test("cancelling an upload prevents a stale proof from being submitted", async () => {
    const controller = new AbortController()
    const request = jest.fn().mockImplementation(async () => {
        controller.abort()
        return {status: "READY", custody_proof_json: "stale proof"}
    })
    await expect(
        resolveCustodyFile(file, scope, request, async () => undefined, 2, controller.signal)
    ).rejects.toMatchObject({reason: "cancelled"})
    request.mockClear()
    await expect(
        resolveCustodyFile(file, scope, request, async () => undefined, 2, controller.signal)
    ).rejects.toMatchObject({reason: "cancelled"})
    expect(request).not.toHaveBeenCalled()
})

test("a cancelled upload cannot submit an offline proof either", async () => {
    const controller = new AbortController()
    controller.abort()
    const request = jest.fn()
    await expect(
        resolveCustodyFile("offline proof", scope, request, undefined, undefined, controller.signal)
    ).rejects.toMatchObject({reason: "cancelled"})
    expect(request).not.toHaveBeenCalled()
})
