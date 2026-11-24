// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {createHash} from "node:crypto"

const mockDecode = jest.fn()
const mockLimit = jest.fn()
jest.mock(
    "sequent-core",
    () => ({
        __esModule: true,
        default: async () => undefined,
        decryptSignedDelivery: (...args: unknown[]) => mockDecode(...args),
        encryptedSignedDeliveryFileLimit: () => mockLimit(),
    }),
    {virtual: true}
)

const mockReply = jest.fn()
const scope = globalThis as unknown as {
    onmessage: (event: {data: {file: unknown; password: string}}) => Promise<void>
    postMessage: typeof mockReply
}

beforeAll(() => {
    scope.postMessage = mockReply
    jest.isolateModules(() => require("./SignedDeliveryWorker"))
})
beforeEach(() => {
    jest.resetAllMocks()
    mockLimit.mockReturnValue(102)
})

it("enforces the canonical limit before reading an oversized file or using a password", async () => {
    const arrayBuffer = jest.fn()
    await scope.onmessage({data: {file: {size: 103, arrayBuffer}, password: "synthetic-pass"}})
    expect(mockReply).toHaveBeenCalledWith({ok: false, reason: "size"})
    expect(arrayBuffer).not.toHaveBeenCalled()
    expect(mockDecode).not.toHaveBeenCalled()
})

it("accepts exactly the canonical limit and hashes the decrypted raw ZIP, not the envelope", async () => {
    const arrayBuffer = jest.fn(async () => new ArrayBuffer(102))
    const zip = new Uint8Array([80, 75, 3, 4, 25, 27])
    mockDecode.mockReturnValue(zip)
    await scope.onmessage({data: {file: {size: 102, arrayBuffer}, password: "synthetic-pass"}})
    expect(mockDecode.mock.calls[0][0]).toBeInstanceOf(Uint8Array)
    expect(mockDecode.mock.calls[0][0].byteLength).toBe(102)
    expect(mockReply).toHaveBeenCalledWith(
        {
            ok: true,
            bytes: zip,
            sha256: createHash("sha256").update(zip).digest("hex"),
        },
        [expect.any(ArrayBuffer)]
    )
})

it("does not expose codec errors or bytes on decryption failure", async () => {
    const arrayBuffer = jest.fn(async () => new ArrayBuffer(102))
    mockDecode.mockImplementation(() => {
        throw new Error("sensitive synthetic input")
    })
    await scope.onmessage({data: {file: {size: 102, arrayBuffer}, password: "synthetic-pass"}})
    expect(mockReply).toHaveBeenCalledWith({ok: false, reason: "decrypt"})
})
