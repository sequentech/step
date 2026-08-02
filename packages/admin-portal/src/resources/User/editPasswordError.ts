// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IGraphQLActionError} from "@sequentech/ui-core"

export const PASSWORD_POLICY_VIOLATION_ERROR_CODE = "PasswordPolicyViolation"

export const isPasswordPolicyViolationError = (error: unknown): boolean => {
    const actionError = error as IGraphQLActionError | undefined
    const containsPasswordPolicyViolation = (value: string | null | undefined): boolean =>
        value?.includes(PASSWORD_POLICY_VIOLATION_ERROR_CODE) === true

    return (
        actionError?.graphQLErrors?.some(
            (graphQLError) =>
                graphQLError.extensions?.code === PASSWORD_POLICY_VIOLATION_ERROR_CODE ||
                containsPasswordPolicyViolation(graphQLError.message) ||
                containsPasswordPolicyViolation(graphQLError.extensions?.internal?.response?.body)
        ) === true || containsPasswordPolicyViolation(actionError?.message)
    )
}
