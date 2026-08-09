// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {RealmPasswordPolicy} from "@/queries/RealmPasswordPolicy"

export const MIN_PASSWORD_LENGTH = 1
export const MAX_PASSWORD_LENGTH = 256

export type PasswordPolicyValidationError =
    | "lengthRange"
    | "minimumExceedsMaximum"
    | "characterClassRequired"

export const validatePasswordPolicy = (
    policy: RealmPasswordPolicy
): PasswordPolicyValidationError | undefined => {
    if (
        !Number.isInteger(policy.minimum_length) ||
        !Number.isInteger(policy.maximum_length) ||
        policy.minimum_length < MIN_PASSWORD_LENGTH ||
        policy.minimum_length > MAX_PASSWORD_LENGTH ||
        policy.maximum_length < MIN_PASSWORD_LENGTH ||
        policy.maximum_length > MAX_PASSWORD_LENGTH
    ) {
        return "lengthRange"
    }
    if (policy.minimum_length > policy.maximum_length) {
        return "minimumExceedsMaximum"
    }
    if (
        !policy.include_uppercase &&
        !policy.include_lowercase &&
        !policy.include_digits &&
        !policy.include_special_characters
    ) {
        return "characterClassRequired"
    }
    return undefined
}
