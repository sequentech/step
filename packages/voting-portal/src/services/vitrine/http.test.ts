// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {httpGet, MAX_RESPONSE_BYTES} from "./http"

describe("bounded receipt requests", () => {
    it("cancels a streamed body as soon as it crosses the limit", async () => {
        const chunk = new Uint8Array(MAX_RESPONSE_BYTES / 2)
        const read = jest
            .fn()
            .mockResolvedValueOnce({done: false, value: chunk})
            .mockResolvedValueOnce({done: false, value: chunk})
            .mockResolvedValueOnce({done: false, value: new Uint8Array(1)})
        const cancel = jest.fn(async () => undefined)
        const arrayBuffer = jest.fn()
        const fetch = jest.fn(
            async () =>
                ({
                    status: 200,
                    headers: {get: () => null},
                    body: {getReader: () => ({read, cancel})},
                    arrayBuffer,
                }) as unknown as Response
        )
        expect(await httpGet("https://log.example/receipt/one", {fetch})).toEqual({status: 413})
        expect(read).toHaveBeenCalledTimes(3)
        expect(cancel).toHaveBeenCalledTimes(1)
        expect(arrayBuffer).not.toHaveBeenCalled()
        expect(fetch).toHaveBeenCalledWith(
            expect.any(String),
            expect.objectContaining({
                credentials: "omit",
                redirect: "error",
                referrerPolicy: "no-referrer",
            })
        )
    })

    it("refuses oversized declared bodies without reading them", async () => {
        const arrayBuffer = jest.fn()
        const cancel = jest.fn(async () => undefined)
        const fetch = jest.fn(
            async () =>
                ({
                    status: 200,
                    headers: {get: () => String(MAX_RESPONSE_BYTES + 1)},
                    body: {cancel},
                    arrayBuffer,
                }) as unknown as Response
        )
        expect(await httpGet("https://log.example/receipt/one", {fetch})).toEqual({status: 413})
        expect(cancel).toHaveBeenCalled()
        expect(arrayBuffer).not.toHaveBeenCalled()
    })
})
