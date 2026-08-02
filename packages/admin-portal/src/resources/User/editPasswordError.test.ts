// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    isPasswordPolicyViolationError,
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
