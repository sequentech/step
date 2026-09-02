// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IGraphQLActionError} from "@sequentech/ui-core"
import {parseActionResponseBody} from "@/services/graphqlActionError"

export const PASSWORD_POLICY_VIOLATION_ERROR_CODE = "PasswordPolicyViolation"
export const PASSWORD_POLICY_NOT_CONFIGURED_ERROR_CODE = "PasswordPolicyNotConfigured"
export const PASSWORD_POLICY_MINIMUM_LENGTH_MISSING_ERROR_CODE =
    "PasswordPolicyMinimumLengthMissing"
export const PASSWORD_POLICY_CHARACTER_CLASS_MISSING_ERROR_CODE =
    "PasswordPolicyCharacterClassMissing"

export type PasswordPolicyRule =
    | "minimumLength"
    | "maximumLength"
    | "uppercase"
    | "lowercase"
    | "digits"
    | "specialCharacters"

export interface PasswordPolicyViolationDetails {
    rule?: PasswordPolicyRule
    requiredCount?: number
}

export type VoterInformationLetterPasswordPolicyError =
    | "notConfigured"
    | "minimumLengthMissing"
    | "characterClassMissing"

const passwordPolicyRules: readonly PasswordPolicyRule[] = [
    "minimumLength",
    "maximumLength",
    "uppercase",
    "lowercase",
    "digits",
    "specialCharacters",
]

const getViolationDetails = (value: unknown): PasswordPolicyViolationDetails | undefined => {
    const extensions = value as
        | {
              password_policy_rule?: unknown
              password_policy_required_count?: unknown
          }
        | undefined
    const rule = passwordPolicyRules.find(
        (candidate) => candidate === extensions?.password_policy_rule
    )
    const requiredCount =
        typeof extensions?.password_policy_required_count === "number"
            ? extensions.password_policy_required_count
            : undefined

    return rule || requiredCount !== undefined ? {rule, requiredCount} : undefined
}

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

export const getPasswordPolicyViolation = (
    error: unknown
): PasswordPolicyViolationDetails | undefined => {
    if (!isPasswordPolicyViolationError(error)) {
        return undefined
    }

    const actionError = error as IGraphQLActionError | undefined
    for (const graphQLError of actionError?.graphQLErrors ?? []) {
        const directDetails = getViolationDetails(graphQLError.extensions)
        if (directDetails) {
            return directDetails
        }

        const responseBody = parseActionResponseBody(
            graphQLError.extensions?.internal?.response?.body
        ) as {extensions?: unknown} | undefined
        const responseDetails = getViolationDetails(responseBody?.extensions)
        if (responseDetails) {
            return responseDetails
        }
    }

    return {}
}

export const isPasswordPolicyNotConfiguredError = (error: unknown): boolean =>
    hasGraphQLActionErrorCode(error, PASSWORD_POLICY_NOT_CONFIGURED_ERROR_CODE, [
        "Password Policy is not configured.",
    ])

/**
 * The message a rejected password should read as, or undefined when the error
 * is not a policy violation. Shared so that the same backend rejection reads
 * the same way wherever a password is submitted.
 */
export const getPasswordPolicyMessage = (
    error: unknown,
    t: (key: string, options?: Record<string, unknown>) => string
): string | undefined => {
    const violation = getPasswordPolicyViolation(error)
    if (!violation) {
        return undefined
    }

    return violation.rule && violation.requiredCount !== undefined
        ? t(`usersAndRolesScreen.editPassword.passwordPolicyRules.${violation.rule}`, {
              count: violation.requiredCount,
          })
        : t("usersAndRolesScreen.editPassword.passwordPolicyViolation")
}

export const getVoterInformationLetterPasswordPolicyError = (
    error: unknown
): VoterInformationLetterPasswordPolicyError | undefined => {
    if (isPasswordPolicyNotConfiguredError(error)) {
        return "notConfigured"
    }
    if (hasGraphQLActionErrorCode(error, PASSWORD_POLICY_MINIMUM_LENGTH_MISSING_ERROR_CODE)) {
        return "minimumLengthMissing"
    }
    if (hasGraphQLActionErrorCode(error, PASSWORD_POLICY_CHARACTER_CLASS_MISSING_ERROR_CODE)) {
        return "characterClassMissing"
    }
    return undefined
}
