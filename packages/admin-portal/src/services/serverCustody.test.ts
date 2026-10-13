// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {readFileSync} from "node:fs"
import {resolve} from "node:path"
import {runInNewContext} from "node:vm"
import ts from "typescript"
import {resolveCustodyFile, ServerCustodyError} from "./serverCustody"

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

test("cancelling a stalled server request finishes without waiting for the network", async () => {
    const controller = new AbortController()
    const request = jest.fn(() => new Promise<never>(() => undefined))
    const result = resolveCustodyFile(file, scope, request, undefined, undefined, controller.signal)
    controller.abort()
    await expect(result).rejects.toMatchObject({reason: "cancelled"})
})

test("a deadline abort reports an unavailable server", async () => {
    const controller = new AbortController()
    const request = jest.fn(() => new Promise<never>(() => undefined))
    const result = resolveCustodyFile(file, scope, request, undefined, undefined, controller.signal)
    controller.abort(new ServerCustodyError("unavailable"))
    await expect(result).rejects.toMatchObject({reason: "unavailable"})
})

// The production portal emits ES5; native Error subclasses must retain their
// identity there so UI error handling and deadline reasons work after bundling.
test("production ES5 errors retain their class and unavailable deadline reason", async () => {
    const source = readFileSync(resolve(__dirname, "serverCustody.ts"), "utf8")
    const compiled = ts.transpileModule(source, {
        compilerOptions: {target: ts.ScriptTarget.ES5, module: ts.ModuleKind.CommonJS},
    })
    const exports = {} as {
        ServerCustodyError: typeof ServerCustodyError
        resolveCustodyFile: typeof resolveCustodyFile
    }
    runInNewContext(compiled.outputText, {exports, Error})
    const error = new exports.ServerCustodyError("unavailable")
    expect(error).toBeInstanceOf(exports.ServerCustodyError)
    const controller = new AbortController()
    controller.abort(error)
    await expect(
        exports.resolveCustodyFile(file, scope, jest.fn(), undefined, undefined, controller.signal)
    ).rejects.toMatchObject({reason: "unavailable"})
})
