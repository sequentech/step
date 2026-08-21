// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {PASSWORD_POLICY_VIOLATION_ERROR_CODE} from "./editPasswordError"
import {getSaveUserErrorMessage, USER_PROFILE_VALIDATION_ERROR_CODE} from "./saveUserError"

const MESSAGE_KEY = "usersAndRolesScreen.voters.errors.editError"
const REASON_KEY = "usersAndRolesScreen.voters.errors.editErrorReason"

// Stands in for i18next: renders the key and whatever was interpolated into it.
const translate = (key: string, options?: Record<string, unknown>): string => {
    const args = Object.entries(options ?? {})
        .map(([name, value]) => `${name}=${String(value)}`)
        .join(",")

    return args ? `${key}(${args})` : key
}

describe("getSaveUserErrorMessage", () => {
    it("reports the reason the backend gave", () => {
        expect(
            getSaveUserErrorMessage(
                {graphQLErrors: [{message: "Cannot change tenant-id attribute"}]},
                MESSAGE_KEY,
                REASON_KEY,
                translate
            )
        ).toBe(`${REASON_KEY}(reason=Cannot change tenant-id attribute)`)
    })

    it("falls back to the bare message when the error explains nothing", () => {
        expect(
            getSaveUserErrorMessage({graphQLErrors: []}, MESSAGE_KEY, REASON_KEY, translate)
        ).toBe(MESSAGE_KEY)
    })

    it("prefers the localized password policy rule over the backend text", () => {
        const message = getSaveUserErrorMessage(
            {
                graphQLErrors: [
                    {
                        message: "The password does not comply with the policy",
                        extensions: {
                            code: PASSWORD_POLICY_VIOLATION_ERROR_CODE,
                            password_policy_rule: "minimumLength",
                            password_policy_required_count: 12,
                        },
                    },
                ],
            },
            MESSAGE_KEY,
            REASON_KEY,
            translate
        )

        expect(message).toContain("passwordPolicyRules.minimumLength")
        expect(message).toContain("count=12")
        expect(message).not.toContain("does not comply")
    })

    it("names the refused attribute and the bounds it broke", () => {
        const message = getSaveUserErrorMessage(
            {
                graphQLErrors: [
                    {
                        message: 'Invalid value for "roll": error-invalid-length (1, 2)',
                        extensions: {
                            code: USER_PROFILE_VALIDATION_ERROR_CODE,
                            user_profile_field: "roll",
                            user_profile_error: "error-invalid-length",
                            user_profile_params: ["roll", 1, 2],
                        },
                    },
                ],
            },
            MESSAGE_KEY,
            REASON_KEY,
            translate,
            (field) => (field === "roll" ? "Roll" : field)
        )

        expect(message).toContain("attribute.invalidLength")
        expect(message).toContain("field=Roll")
        expect(message).toContain("min=1")
        expect(message).toContain("max=2")
        // The attribute name Keycloak repeats as the first argument is not a bound.
        expect(message).not.toContain("min=roll")
    })

    it("reads the refused attribute out of a webhook response body", () => {
        const message = getSaveUserErrorMessage(
            {
                graphQLErrors: [
                    {
                        message: "unexpected response from webhook",
                        extensions: {
                            code: "unexpected",
                            internal: {
                                response: {
                                    body: JSON.stringify({
                                        message: "Invalid value",
                                        extensions: {
                                            code: USER_PROFILE_VALIDATION_ERROR_CODE,
                                            user_profile_field: "ward",
                                            user_profile_error: "error-user-attribute-required",
                                            user_profile_params: ["ward"],
                                        },
                                    }),
                                },
                            },
                        },
                    },
                ],
            },
            MESSAGE_KEY,
            REASON_KEY,
            translate
        )

        expect(message).toContain("attribute.required")
        expect(message).toContain("field=ward")
    })

    it("falls back to a generic wording for a constraint it does not know", () => {
        const message = getSaveUserErrorMessage(
            {
                graphQLErrors: [
                    {
                        extensions: {
                            code: USER_PROFILE_VALIDATION_ERROR_CODE,
                            user_profile_field: "roll",
                            user_profile_error: "error-something-new",
                            user_profile_params: ["roll"],
                        },
                    },
                ],
            },
            MESSAGE_KEY,
            REASON_KEY,
            translate
        )

        expect(message).toContain("attribute.invalid")
        expect(message).not.toContain("error-something-new")
    })

    it("uses the generic policy message when the violation carries no details", () => {
        const message = getSaveUserErrorMessage(
            {graphQLErrors: [{extensions: {code: PASSWORD_POLICY_VIOLATION_ERROR_CODE}}]},
            MESSAGE_KEY,
            REASON_KEY,
            translate
        )

        expect(message).toContain("usersAndRolesScreen.editPassword.passwordPolicyViolation")
    })
})
