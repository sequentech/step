// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    isCustodyRequestDownloadUnavailableError,
    CUSTODY_REQUEST_DOWNLOAD_UNAVAILABLE_ERROR_CODE,
} from "./custodyRequestDownloadError"

describe("isCustodyRequestDownloadUnavailableError", () => {
    it("recognizes a direct Hasura error code", () => {
        expect(
            isCustodyRequestDownloadUnavailableError({
                graphQLErrors: [
                    {extensions: {code: CUSTODY_REQUEST_DOWNLOAD_UNAVAILABLE_ERROR_CODE}},
                ],
            })
        ).toBe(true)
    })

    it("recognizes the code inside the body Hasura could not parse", () => {
        expect(
            isCustodyRequestDownloadUnavailableError({
                graphQLErrors: [
                    {
                        message: "unexpected",
                        extensions: {
                            code: "unexpected",
                            internal: {
                                response: {
                                    status: 409,
                                    body: JSON.stringify({
                                        message: "Private key download is no longer available",
                                        extensions: {
                                            code: CUSTODY_REQUEST_DOWNLOAD_UNAVAILABLE_ERROR_CODE,
                                        },
                                    }),
                                },
                            },
                        },
                    },
                ],
            })
        ).toBe(true)
    })

    it("rejects unrelated and malformed errors", () => {
        expect(
            isCustodyRequestDownloadUnavailableError({
                graphQLErrors: [
                    {extensions: {code: "InternalServerError"}},
                    {extensions: {internal: {response: {body: "not json"}}}},
                ],
            })
        ).toBe(false)
        expect(isCustodyRequestDownloadUnavailableError(new Error("Network error"))).toBe(false)
    })
})
