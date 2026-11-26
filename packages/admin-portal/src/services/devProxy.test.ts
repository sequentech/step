// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// eslint-disable-next-line @typescript-eslint/no-require-imports
const {UNWRAP_PATH, devProxy, isUnwrapRequest} = require("../../dev-proxy.cjs")
import {SERVER_UNWRAP_PATH} from "./SignedDeliveryWorkerClient"

describe("development proxy to Harvest", () => {
    it("keeps /b4/ and forwards only the exact unwrap path and method", () => {
        expect(UNWRAP_PATH).toBe(SERVER_UNWRAP_PATH)
        const [b4, unwrap] = devProxy(undefined)
        expect(b4).toEqual({context: ["/b4/"], target: "http://harvest:8400"})
        expect(unwrap.target).toBe("http://harvest:8400")
        expect(devProxy("localhost:8400")[1].target).toBe("http://localhost:8400")
        expect(isUnwrapRequest(UNWRAP_PATH, {method: "POST"})).toBe(true)
        expect(isUnwrapRequest(UNWRAP_PATH, {method: "post"})).toBe(true)
        for (const [path, method] of [
            [UNWRAP_PATH, "GET"],
            [`${UNWRAP_PATH}/x`, "POST"],
            [`${UNWRAP_PATH}x`, "POST"],
            ["/configuration-package-intake/", "POST"],
            ["/graphql", "POST"],
        ]) {
            expect(isUnwrapRequest(path, {method})).toBe(false)
        }
    })

    it("overwrites the scheme as plain HTTP and keeps the browser's Host", () => {
        const [, unwrap] = devProxy(undefined)
        // One lowercase key: Node's incoming header is replaced, not duplicated.
        expect(unwrap.headers).toEqual({"x-forwarded-proto": "http"})
        expect(unwrap.changeOrigin).toBe(false)
        expect(unwrap.xfwd).toBe(false)
    })
})
