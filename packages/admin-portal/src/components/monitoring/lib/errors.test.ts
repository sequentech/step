// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ApolloError} from "@apollo/client"
import {GraphQLError} from "graphql"
import {
    BUSY_RETRY_MS,
    EMonitoringErrorCode,
    monitoringErrorCode,
    monitoringErrorMessage,
    monitoringProblems,
} from "./errors"

const refusal = (extensions: Record<string, unknown>) =>
    new ApolloError({graphQLErrors: [new GraphQLError("refused", {extensions})]})

describe("Harvest's monitoring errors", () => {
    it("reads the code Hasura passes on in the GraphQL error's extensions", () => {
        expect(monitoringErrorCode(refusal({code: "MONITORING_SNAPSHOT_PRUNED"}))).toBe(
            EMonitoringErrorCode.SNAPSHOT_PRUNED
        )
        expect(monitoringErrorCode(refusal({code: "MONITORING_BUSY"}))).toBe(
            EMonitoringErrorCode.BUSY
        )
    })

    it("reads the code from the handler's body when Hasura wraps it", () => {
        const wrapped = refusal({
            code: "unexpected",
            internal: {
                response: {
                    status: 403,
                    body: {message: "no", extensions: {code: "MONITORING_FORBIDDEN_SCOPE"}},
                },
            },
        })
        expect(monitoringErrorCode(wrapped)).toBe(EMonitoringErrorCode.FORBIDDEN_SCOPE)
        const text = refusal({
            internal: {response: {body: '{"extensions":{"code":"MONITORING_LOCKED_DOWN"}}'}},
        })
        expect(monitoringErrorCode(text)).toBe(EMonitoringErrorCode.LOCKED_DOWN)
    })

    it("reads the editor's codes too, from the one list the view and the editor share", () => {
        expect(monitoringErrorCode(refusal({code: "MONITORING_CONFLICT"}))).toBe(
            EMonitoringErrorCode.CONFLICT
        )
        expect(monitoringErrorMessage(refusal({code: "MONITORING_CONFLICT"}))).toBe(
            "monitoring.errors.conflict"
        )
    })

    it("has no code for a network failure or an unknown code", () => {
        expect(monitoringErrorCode(new ApolloError({networkError: new Error("down")}))).toBe(
            undefined
        )
        expect(monitoringErrorCode(refusal({code: "SOMETHING_ELSE"}))).toBe(undefined)
        expect(monitoringErrorCode(undefined)).toBe(undefined)
    })

    it("names a translated message for every code", () => {
        for (const code of Object.values(EMonitoringErrorCode)) {
            expect(monitoringErrorMessage(refusal({code}))).toMatch(/^monitoring\.errors\./)
        }
        expect(monitoringErrorMessage(new Error("?"))).toBe("monitoring.errors.unknown")
        expect(BUSY_RETRY_MS).toBeGreaterThan(0)
    })
})

describe("the problems of a refusal", () => {
    const problem = {
        severity: "ERROR",
        code: "unknown_option",
        path: "widget_selector_values.turnout-by-group.breakdown",
        message: "'region' is not an option of 'breakdown'.",
    }

    it("reads MONITORING_INVALID and its problems", () => {
        const error = refusal({code: "MONITORING_INVALID", problems: [problem]})
        expect(monitoringErrorCode(error)).toBe(EMonitoringErrorCode.INVALID)
        expect(monitoringProblems(error)).toEqual([
            {code: problem.code, path: problem.path, message: problem.message},
        ])
    })

    it("reads the problems from the handler's body when Hasura wraps it", () => {
        const body = JSON.stringify({
            message: "The configuration is not valid.",
            extensions: {code: "MONITORING_INVALID", problems: [problem]},
        })
        const error = refusal({internal: {response: {status: 422, body}}})
        expect(monitoringErrorCode(error)).toBe(EMonitoringErrorCode.INVALID)
        expect(monitoringProblems(error).map(({code}) => code)).toEqual(["unknown_option"])
    })

    it("has none for other errors, and skips what is not a problem", () => {
        expect(monitoringProblems(refusal({code: "MONITORING_BUSY"}))).toEqual([])
        expect(monitoringProblems(refusal({problems: [null, "x", {code: 3}]}))).toEqual([])
        expect(monitoringProblems(undefined)).toEqual([])
    })
})
