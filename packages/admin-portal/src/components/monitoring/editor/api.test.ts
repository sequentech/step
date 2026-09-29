// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {interpretSaveError} from "./api"
import {EMonitoringProblemSeverity, EMonitoringSaveStatus} from "./types"

describe("interpretSaveError", () => {
    it("reads a conflict from the extensions Hasura promoted", () => {
        expect(
            interpretSaveError({
                graphQLErrors: [
                    {
                        message: "conflict",
                        extensions: {
                            code: "CONFLICT",
                            current_revision: 7,
                            author: {id: "u1", name: "Ana"},
                            time: "2026-09-29T10:00:00Z",
                        },
                    },
                ],
            })
        ).toEqual({
            status: EMonitoringSaveStatus.CONFLICT,
            current_revision: 7,
            author: {id: "u1", name: "Ana"},
            time: "2026-09-29T10:00:00Z",
        })
    })

    it("reads a conflict from the original body when Hasura kept it", () => {
        expect(
            interpretSaveError({
                graphQLErrors: [
                    {
                        message: "http exception when calling webhook",
                        extensions: {
                            code: "unexpected",
                            internal: {
                                response: {
                                    status: 409,
                                    body: JSON.stringify({current_revision: 3, author: "u2"}),
                                },
                            },
                        },
                    },
                ],
            })
        ).toEqual(
            expect.objectContaining({
                status: EMonitoringSaveStatus.CONFLICT,
                current_revision: 3,
                author: {id: "u2"},
            })
        )
    })

    it("reads the problems of a refused save", () => {
        const outcome = interpretSaveError({
            graphQLErrors: [
                {
                    extensions: {
                        internal: {
                            response: {
                                status: 422,
                                body: JSON.stringify({
                                    problems: [
                                        {
                                            severity: "error",
                                            code: "forbidden_key",
                                            path: "query.sql",
                                            message: "No SQL",
                                        },
                                    ],
                                }),
                            },
                        },
                    },
                },
            ],
        })
        expect(outcome).toEqual({
            status: EMonitoringSaveStatus.INVALID,
            problems: [expect.objectContaining({severity: EMonitoringProblemSeverity.ERROR})],
        })
    })

    it("leaves any other failure to the caller", () => {
        expect(interpretSaveError(new Error("network"))).toBeUndefined()
        expect(
            interpretSaveError({
                graphQLErrors: [{extensions: {internal: {response: {status: 503}}}}],
            })
        ).toBeUndefined()
    })
})
