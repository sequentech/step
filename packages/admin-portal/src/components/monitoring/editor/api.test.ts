// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    EMonitoringErrorCode,
    interpretSaveError,
    monitoringErrorCode,
    monitoringErrorMessage,
} from "./api"
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

    /** What Hasura answers for Harvest's `{message, extensions: {code, ...}}` refusal. */
    const refusal = (extensions: Record<string, unknown>, message = "refused") => ({
        graphQLErrors: [{message, extensions: {path: "$", ...extensions}}],
    })

    it("reads Harvest's 409 MONITORING_CONFLICT", () => {
        expect(
            interpretSaveError(
                refusal({
                    code: EMonitoringErrorCode.CONFLICT,
                    current_revision: 12,
                    author: {id: "u-ana", name: "Ana Reyes"},
                    time: "2026-09-29T10:00:00Z",
                })
            )
        ).toEqual({
            status: EMonitoringSaveStatus.CONFLICT,
            current_revision: 12,
            author: {id: "u-ana", name: "Ana Reyes"},
            time: "2026-09-29T10:00:00Z",
        })
    })

    it("keeps a conflict with a removed document as no revision, not revision 0", () => {
        expect(
            interpretSaveError(
                refusal({
                    code: EMonitoringErrorCode.CONFLICT,
                    current_revision: null,
                    author: null,
                    time: null,
                })
            )
        ).toEqual({
            status: EMonitoringSaveStatus.CONFLICT,
            current_revision: null,
            author: null,
            time: null,
        })
    })

    it("reads Harvest's 422 MONITORING_INVALID", () => {
        expect(
            interpretSaveError(
                refusal({
                    code: EMonitoringErrorCode.INVALID,
                    problems: [
                        {
                            severity: "ERROR",
                            code: "forbidden_key",
                            path: "query.sql",
                            message: "No SQL",
                            engine_code: null,
                        },
                    ],
                })
            )
        ).toEqual({
            status: EMonitoringSaveStatus.INVALID,
            problems: [
                {
                    severity: EMonitoringProblemSeverity.ERROR,
                    code: "forbidden_key",
                    path: "query.sql",
                    message: "No SQL",
                    engine_code: undefined,
                },
            ],
        })
    })
})

describe("monitoringErrorMessage", () => {
    it.each([
        [EMonitoringErrorCode.CHECKS_UNAVAILABLE, "monitoring.editor.errors.checksUnavailable"],
        [EMonitoringErrorCode.BUSY, "monitoring.editor.errors.busy"],
        [EMonitoringErrorCode.LOCKED_DOWN, "monitoring.editor.errors.lockedDown"],
        [EMonitoringErrorCode.FORBIDDEN_SCOPE, "monitoring.editor.errors.forbiddenScope"],
    ])("explains %s", (code, key) => {
        const error = {graphQLErrors: [{message: "x", extensions: {code}}]}
        expect(monitoringErrorCode(error)).toBe(code)
        expect(monitoringErrorMessage(error)).toBe(key)
        expect(interpretSaveError(error)).toBeUndefined()
    })

    it("finds the code in the original body when Hasura could not promote it", () => {
        const error = {
            graphQLErrors: [
                {
                    extensions: {
                        code: "unexpected",
                        internal: {
                            response: {
                                status: 503,
                                body: JSON.stringify({
                                    message: "busy",
                                    extensions: {code: EMonitoringErrorCode.BUSY},
                                }),
                            },
                        },
                    },
                },
            ],
        }
        expect(monitoringErrorMessage(error)).toBe("monitoring.editor.errors.busy")
    })

    it("says nothing of its own for a failure Harvest did not explain", () => {
        expect(monitoringErrorMessage(new Error("network"))).toBeUndefined()
    })
})
