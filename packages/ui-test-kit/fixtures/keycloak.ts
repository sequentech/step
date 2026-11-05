// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

export const KEYCLOAK_SYNTHETIC_USER = {
    username: "synthetic-voter",
    email: "synthetic-voter@example.test",
    firstName: "Synthetic",
    lastName: "Voter",
} as const

export const KEYCLOAK_MESSAGE_OTP = {
    address: "sy***@*****le.test",
    isOtl: false,
    codeJustSent: false,
    resendTimer: "60",
    ttl: "300",
    codeLength: "6",
} as const

export const KEYCLOAK_PROFILE_ATTRIBUTES = [
    {
        name: "synthetic-colour",
        displayName: "Synthetic colour",
        annotations: {inputType: "select", inputHelperTextBefore: "Synthetic helper text"},
        validations: {options: {options: ["red", "green"]}},
        permissions: {view: ["admin", "user"], edit: ["admin", "user"]},
    },
    {
        name: "synthetic-phone",
        displayName: "Synthetic phone",
        annotations: {inputType: "html5-tel"},
        permissions: {view: ["admin", "user"], edit: ["admin", "user"]},
    },
] as const
