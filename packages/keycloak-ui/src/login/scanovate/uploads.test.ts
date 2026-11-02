// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {afterEach, describe, expect, it, vi} from "vitest"
import {CapturePart} from "./form"
import {CaptureProblem} from "./types"
import {
    UploadError,
    UploadRejection,
    fetchUploads,
    uploadCaptures,
    type CaptureUploader,
} from "./uploads"

const SETTINGS = {url: "/realms/r/identity-verification/capture/", token: "capture-token"}
const jpeg = new Blob([new Uint8Array([0xff, 0xd8, 0xff])], {type: "image/jpeg"})
// Media type parameters are left out of the upload
const holding = new Blob([new Uint8Array([0xff, 0xd8, 0xff])], {type: "image/jpeg;q=0.9"})

function stubFetch(...responses: (Response | Error)[]) {
    const fetchMock = vi.fn()
    for (const response of responses) {
        if (response instanceof Error) fetchMock.mockRejectedValueOnce(response)
        else fetchMock.mockResolvedValueOnce(response)
    }
    vi.stubGlobal("fetch", fetchMock)
    return fetchMock
}

const ok = () => new Response('{"status":"OK"}', {status: 200})

afterEach(() => {
    vi.unstubAllGlobals()
})

describe("fetchUploads", () => {
    it("puts each file under its part with the capture token", async () => {
        const fetchMock = stubFetch(ok(), ok())
        const uploader = fetchUploads(SETTINGS)
        await uploader.upload(CapturePart.Front, jpeg)
        await uploader.upload(CapturePart.Holding, holding)

        const [url, init] = fetchMock.mock.calls[0]
        expect(url).toBe("/realms/r/identity-verification/capture/front")
        expect(init.method).toBe("PUT")
        expect(init.headers).toEqual({
            "X-Capture-Token": "capture-token",
            "Content-Type": "image/jpeg",
        })
        expect(init.body).toBe(jpeg)
        expect(init.credentials).toBe("omit")
        const [holdingUrl, holdingInit] = fetchMock.mock.calls[1]
        expect(holdingUrl).toBe("/realms/r/identity-verification/capture/holding")
        expect(holdingInit.headers["Content-Type"]).toBe("image/jpeg")
    })

    it("rejects a stale token apart from other failures", async () => {
        stubFetch(new Response("{}", {status: 401}))
        await expect(fetchUploads(SETTINGS).upload(CapturePart.Front, jpeg)).rejects.toEqual(
            new UploadError(UploadRejection.InvalidToken)
        )
        for (const failure of [new Response("{}", {status: 413}), new TypeError("offline")]) {
            stubFetch(failure)
            await expect(fetchUploads(SETTINGS).upload(CapturePart.Front, jpeg)).rejects.toEqual(
                new UploadError(UploadRejection.Failed)
            )
        }
    })
})

describe("uploadCaptures", () => {
    function recording(fail?: UploadRejection): {
        uploader: CaptureUploader
        parts: CapturePart[]
    } {
        const parts: CapturePart[] = []
        return {
            parts,
            uploader: {
                upload: async (part) => {
                    parts.push(part)
                    if (fail !== undefined) throw new UploadError(fail)
                },
            },
        }
    }

    it("uploads every part and reports no problem", async () => {
        const {uploader, parts} = recording()
        await expect(
            uploadCaptures(uploader, [
                {part: CapturePart.Front, blob: jpeg},
                {part: CapturePart.Holding, blob: jpeg},
            ])
        ).resolves.toBeNull()
        expect(parts).toEqual([CapturePart.Front, CapturePart.Holding])
    })

    it("maps failures to the problem to show", async () => {
        await expect(
            uploadCaptures(recording(UploadRejection.Failed).uploader, [
                {part: CapturePart.Front, blob: jpeg},
            ])
        ).resolves.toBe(CaptureProblem.UploadFailed)
        await expect(
            uploadCaptures(recording(UploadRejection.InvalidToken).uploader, [
                {part: CapturePart.Front, blob: jpeg},
            ])
        ).resolves.toBe(CaptureProblem.CaptureExpired)
        await expect(
            uploadCaptures(
                {
                    upload: async () => {
                        throw new Error("unexpected")
                    },
                },
                [{part: CapturePart.Front, blob: jpeg}]
            )
        ).resolves.toBe(CaptureProblem.UploadFailed)
    })
})
