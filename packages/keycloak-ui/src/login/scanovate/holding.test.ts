// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it, vi} from "vitest"
import {CAPTURE_TOKEN_HEADER, uploadHoldingPhoto} from "./holding"
import {CaptureProblem} from "./types"

const upload = {url: "/realms/r/identity-verification/capture/holding", token: "one-time"}
const photo = new Blob([new Uint8Array([0xff, 0xd8, 0xff])], {type: "image/jpeg"})

function respond(status: number) {
    return vi.fn(async () => new Response("{}", {status}))
}

describe("uploadHoldingPhoto", () => {
    it("puts the photo with the page's one-time token", async () => {
        const fetcher = respond(200)

        await expect(uploadHoldingPhoto(upload, photo, fetcher)).resolves.toBeNull()

        expect(fetcher).toHaveBeenCalledTimes(1)
        const [url, init] = fetcher.mock.calls[0] as unknown as [string, RequestInit]
        expect(url).toBe(upload.url)
        expect(init.method).toBe("PUT")
        expect(init.body).toBe(photo)
        expect(init.credentials).toBe("omit")
        expect(init.headers).toEqual({
            [CAPTURE_TOKEN_HEADER]: "one-time",
            "Content-Type": "image/jpeg",
        })
    })

    it("asks for a new page when Keycloak no longer knows the token", async () => {
        await expect(uploadHoldingPhoto(upload, photo, respond(401))).resolves.toBe(
            CaptureProblem.InvalidToken
        )
    })

    it("reports a failed upload otherwise", async () => {
        await expect(uploadHoldingPhoto(upload, photo, respond(413))).resolves.toBe(
            CaptureProblem.Upload
        )
        const offline = vi.fn(async () => {
            throw new TypeError("Failed to fetch")
        })
        await expect(uploadHoldingPhoto(upload, photo, offline)).resolves.toBe(
            CaptureProblem.Upload
        )
    })
})
