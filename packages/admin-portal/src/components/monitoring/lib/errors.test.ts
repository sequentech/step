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
