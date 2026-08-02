// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IGraphQLActionError} from "@sequentech/ui-core"

export const PASSWORD_POLICY_VIOLATION_ERROR_CODE = "PasswordPolicyViolation"
export const PASSWORD_POLICY_NOT_CONFIGURED_ERROR_CODE = "PasswordPolicyNotConfigured"

export const hasGraphQLActionErrorCode = (
    error: unknown,
    errorCode: string,
    legacyMessages: readonly string[] = []
): boolean => {
    const actionError = error as IGraphQLActionError | undefined
    const markers = [errorCode, ...legacyMessages]
    const containsMarker = (value: string | null | undefined): boolean =>
        markers.some((marker) => value?.includes(marker) === true)

    return (
        actionError?.graphQLErrors?.some(
            (graphQLError) =>
                graphQLError.extensions?.code === errorCode ||
                containsMarker(graphQLError.message) ||
                containsMarker(graphQLError.extensions?.internal?.response?.body)
        ) === true || containsMarker(actionError?.message)
    )
}

export const isPasswordPolicyViolationError = (error: unknown): boolean =>
    hasGraphQLActionErrorCode(error, PASSWORD_POLICY_VIOLATION_ERROR_CODE)

export const isPasswordPolicyNotConfiguredError = (error: unknown): boolean =>
    hasGraphQLActionErrorCode(error, PASSWORD_POLICY_NOT_CONFIGURED_ERROR_CODE, [
        "Password Policy is not configured.",
    ])
