// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {PASSWORD_POLICY_VIOLATION_ERROR_CODE} from "./editPasswordError"
import {getSaveUserErrorMessage} from "./saveUserError"

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
