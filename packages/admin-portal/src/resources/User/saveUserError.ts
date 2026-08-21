// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IGraphQLActionError} from "@sequentech/ui-core"
import {getGraphQLActionErrorReason, parseActionResponseBody} from "@/services/graphqlActionError"
import {getPasswordPolicyMessage} from "./editPasswordError"

export type TranslateMessage = (key: string, options?: Record<string, unknown>) => string

export const USER_PROFILE_VALIDATION_ERROR_CODE = "UserProfileValidation"

export interface UserProfileValidation {
    field?: string
    error?: string
    params: unknown[]
}

// Keycloak's own constraint keys, mapped to what the admin is told. Anything
// Keycloak may add later falls back to the generic message rather than
// surfacing a raw key.
const CONSTRAINT_MESSAGES: Record<string, string> = {
    "error-invalid-length": "invalidLength",
    "error-invalid-length-too-short": "tooShort",
    "error-invalid-length-too-long": "tooLong",
    "error-user-attribute-required": "required",
    "error-invalid-email": "invalidEmail",
    "error-pattern-no-match": "invalidFormat",
}

const readValidation = (value: unknown): UserProfileValidation | undefined => {
    const extensions = value as
        | {
              code?: unknown
              user_profile_field?: unknown
              user_profile_error?: unknown
              user_profile_params?: unknown
          }
        | undefined
    if (extensions?.code !== USER_PROFILE_VALIDATION_ERROR_CODE) {
        return undefined
    }

    return {
        field:
            typeof extensions.user_profile_field === "string"
                ? extensions.user_profile_field
                : undefined,
        error:
            typeof extensions.user_profile_error === "string"
                ? extensions.user_profile_error
                : undefined,
        params: Array.isArray(extensions.user_profile_params) ? extensions.user_profile_params : [],
    }
}

/**
 * The user profile constraint Keycloak refused the write against, when it named
 * one. Harvest forwards the attribute and the constraint's arguments, so the
 * admin can be told which field to correct rather than that something failed.
 */
export const getUserProfileValidation = (error: unknown): UserProfileValidation | undefined => {
    const actionError = error as IGraphQLActionError | undefined

    for (const graphQLError of actionError?.graphQLErrors ?? []) {
        const direct = readValidation(graphQLError.extensions)
        if (direct) {
            return direct
        }

        const responseBody = parseActionResponseBody(
            graphQLError.extensions?.internal?.response?.body
        ) as {extensions?: unknown} | undefined
        const fromBody = readValidation(responseBody?.extensions)
        if (fromBody) {
            return fromBody
        }
    }

    return undefined
}

// Keycloak repeats the attribute name as the constraint's first argument, and
// the message already names the field.
const constraintArguments = (validation: UserProfileValidation): unknown[] =>
    validation.params[0] === validation.field ? validation.params.slice(1) : validation.params

const interpolation = (messageKey: string, args: unknown[]): Record<string, unknown> => {
    switch (messageKey) {
        case "invalidLength":
            return {min: args[0], max: args[1]}
        case "tooShort":
            return {min: args[0]}
        case "tooLong":
            return {max: args[0]}
        default:
            return {}
    }
}

const getUserProfileValidationMessage = (
    error: unknown,
    t: TranslateMessage,
    resolveFieldLabel?: (field: string) => string
): string | undefined => {
    const validation = getUserProfileValidation(error)
    if (!validation) {
        return undefined
    }

    const field = validation.field ?? ""
    const messageKey = CONSTRAINT_MESSAGES[validation.error ?? ""] ?? "invalid"

    return t(`usersAndRolesScreen.voters.errors.attribute.${messageKey}`, {
        field: (resolveFieldLabel && field ? resolveFieldLabel(field) : field) || field,
        ...interpolation(messageKey, constraintArguments(validation)),
    })
}

/**
 * Message shown when saving a voter fails: the reason the backend gave when
 * there is one, and the bare failure message when there is not.
 */
export const getSaveUserErrorMessage = (
    error: unknown,
    messageKey: string,
    reasonKey: string,
    t: TranslateMessage,
    resolveFieldLabel?: (field: string) => string
): string => {
    // The voter editor submits passwords through the same `edit_user` action as
    // EditPassword, so a rejected password reads the same way in both places
    // rather than falling back to Harvest's untranslated text. A refused
    // attribute is likewise stated in the admin's language, naming the field.
    const reason =
        getPasswordPolicyMessage(error, t) ??
        getUserProfileValidationMessage(error, t, resolveFieldLabel) ??
        getGraphQLActionErrorReason(error)

    return reason ? t(reasonKey, {reason}) : t(messageKey)
}
