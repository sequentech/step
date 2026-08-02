// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    hasGraphQLActionErrorCode,
    isPasswordPolicyNotConfiguredError,
    isPasswordPolicyViolationError,
    PASSWORD_POLICY_NOT_CONFIGURED_ERROR_CODE,
    PASSWORD_POLICY_VIOLATION_ERROR_CODE,
} from "./editPasswordError"

describe("isPasswordPolicyViolationError", () => {
    it("recognizes the structured Hasura action error code", () => {
        expect(
            isPasswordPolicyViolationError({
                graphQLErrors: [
                    {
                        extensions: {
                            code: PASSWORD_POLICY_VIOLATION_ERROR_CODE,
                        },
                    },
                ],
            })
        ).toBe(true)
    })

    it("recognizes the legacy message marker", () => {
        expect(
            isPasswordPolicyViolationError({
                message: `Request failed: ${PASSWORD_POLICY_VIOLATION_ERROR_CODE}`,
            })
        ).toBe(true)
    })

    it("recognizes the marker in a legacy Hasura webhook response body", () => {
        expect(
            isPasswordPolicyViolationError({
                graphQLErrors: [
                    {
                        message: "unexpected",
                        extensions: {
                            code: "unexpected",
                            internal: {
                                response: {
                                    body: PASSWORD_POLICY_VIOLATION_ERROR_CODE,
                                },
                            },
                        },
                    },
                ],
            })
        ).toBe(true)
    })

    it("does not classify unrelated errors as password policy violations", () => {
        expect(isPasswordPolicyViolationError(new Error("Error editing user"))).toBe(false)
        expect(isPasswordPolicyViolationError(undefined)).toBe(false)
    })
})

describe("isPasswordPolicyNotConfiguredError", () => {
    it("recognizes the structured Hasura action error code", () => {
        expect(
            isPasswordPolicyNotConfiguredError({
                graphQLErrors: [
                    {
                        extensions: {
                            code: PASSWORD_POLICY_NOT_CONFIGURED_ERROR_CODE,
                        },
                    },
                ],
            })
        ).toBe(true)
    })

    it("recognizes the code in a Hasura webhook response body", () => {
        expect(
            isPasswordPolicyNotConfiguredError({
                graphQLErrors: [
                    {
                        message: "unexpected",
                        extensions: {
                            code: "unexpected",
                            internal: {
                                response: {
                                    body: JSON.stringify({
                                        extensions: {
                                            code: PASSWORD_POLICY_NOT_CONFIGURED_ERROR_CODE,
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

    it("recognizes the legacy Harvest message", () => {
        expect(
            isPasswordPolicyNotConfiguredError(
                new Error("Password Policy is not configured. Set it under Election Event Data.")
            )
        ).toBe(true)
    })

    it("does not classify another action error code as a missing policy", () => {
        expect(
            isPasswordPolicyNotConfiguredError({
                graphQLErrors: [
                    {
                        extensions: {
                            code: PASSWORD_POLICY_VIOLATION_ERROR_CODE,
                        },
                    },
                ],
            })
        ).toBe(false)
    })
})

describe("hasGraphQLActionErrorCode", () => {
    it("recognizes an error code in a GraphQL error message", () => {
        expect(
            hasGraphQLActionErrorCode(
                {
                    graphQLErrors: [
                        {
                            message: `Request failed: ${PASSWORD_POLICY_NOT_CONFIGURED_ERROR_CODE}`,
                        },
                    ],
                },
                PASSWORD_POLICY_NOT_CONFIGURED_ERROR_CODE
            )
        ).toBe(true)
    })
})
