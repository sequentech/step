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

    const profileErrors = (
        errors: Array<Record<string, unknown>>,
        total?: number
    ): Record<string, unknown> => ({
        code: USER_PROFILE_VALIDATION_ERROR_CODE,
        user_profile_errors: errors,
        user_profile_errors_total: total ?? errors.length,
    })

    it("names the refused attribute and the bounds it broke", () => {
        const message = getSaveUserErrorMessage(
            {
                graphQLErrors: [
                    {
                        extensions: profileErrors([
                            {
                                field: "roll",
                                error: "error-invalid-length",
                                params: ["roll", 1, 2],
                            },
                        ]),
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

    it("names every refused attribute, in the order Keycloak reported them", () => {
        const message = getSaveUserErrorMessage(
            {
                graphQLErrors: [
                    {
                        extensions: profileErrors([
                            {field: "ward", error: "error-invalid-value", params: ["ward"]},
                            {
                                field: "roll",
                                error: "error-invalid-length",
                                params: ["roll", 1, 2],
                            },
                        ]),
                    },
                ],
            },
            MESSAGE_KEY,
            REASON_KEY,
            translate
        )

        expect(message).toContain("field=ward")
        expect(message).toContain("field=roll")
        expect(message.indexOf("field=ward")).toBeLessThan(message.indexOf("field=roll"))
    })

    it("says how many refused attributes it did not list", () => {
        const message = getSaveUserErrorMessage(
            {
                graphQLErrors: [
                    {
                        extensions: profileErrors(
                            [{field: "ward", error: "error-invalid-value", params: ["ward"]}],
                            4
                        ),
                    },
                ],
            },
            MESSAGE_KEY,
            REASON_KEY,
            translate
        )

        expect(message).toContain("attribute.andMore")
        expect(message).toContain("count=3")
    })

    it("does not claim there are more when everything was listed", () => {
        const message = getSaveUserErrorMessage(
            {
                graphQLErrors: [
                    {
                        extensions: profileErrors([
                            {field: "ward", error: "error-invalid-value", params: ["ward"]},
                        ]),
                    },
                ],
            },
            MESSAGE_KEY,
            REASON_KEY,
            translate
        )

        expect(message).not.toContain("andMore")
    })

    it("reads the refused attributes out of a webhook response body", () => {
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
                                        extensions: profileErrors([
                                            {
                                                field: "ward",
                                                error: "error-user-attribute-required",
                                                params: ["ward"],
                                            },
                                        ]),
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

    // Keycloak's generic error shape carries no field, and is not a refused
    // attribute; reporting it as one would name no field at all.
    it("ignores a reported entry that names no attribute", () => {
        const message = getSaveUserErrorMessage(
            {
                graphQLErrors: [
                    {
                        message: "The password does not comply with the policy",
                        extensions: profileErrors([
                            {error: "invalidPasswordMessage", params: ["8"]},
                        ]),
                    },
                ],
            },
            MESSAGE_KEY,
            REASON_KEY,
            translate
        )

        expect(message).not.toContain("attribute.")
        expect(message).toContain("The password does not comply with the policy")
    })

    // One constraint without a wording of its own must not cost every other
    // refused attribute in the same refusal its own.
    it("still localizes the attributes it knows alongside one it does not", () => {
        const message = getSaveUserErrorMessage(
            {
                graphQLErrors: [
                    {
                        message: "Invalid value for two attributes",
                        extensions: profileErrors([
                            {
                                field: "roll",
                                error: "error-invalid-length",
                                params: ["roll", 1, 2],
                            },
                            {field: "username", error: "error-username-exists", params: []},
                        ]),
                    },
                ],
            },
            MESSAGE_KEY,
            REASON_KEY,
            translate
        )

        expect(message).toContain("attribute.invalidLength")
        expect(message).toContain("min=1")
        expect(message).toContain("attribute.invalidNamed")
        expect(message).toContain("constraint=error-username-exists")
    })

    // Saying only that something is invalid is less than the message already on
    // the wire, which names the field and the constraint.
    it("keeps the backend message for a constraint it does not know", () => {
        const message = getSaveUserErrorMessage(
            {
                graphQLErrors: [
                    {
                        message: 'Invalid value for "roll": error-email-exists',
                        extensions: profileErrors([
                            {field: "roll", error: "error-email-exists", params: ["roll"]},
                        ]),
                    },
                ],
            },
            MESSAGE_KEY,
            REASON_KEY,
            translate
        )

        expect(message).toContain("attribute.invalidNamed")
        expect(message).toContain("constraint=error-email-exists")
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
