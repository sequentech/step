// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IGraphQLActionError} from "@sequentech/ui-core"
import {getGraphQLActionErrorReason, parseActionResponseBody} from "@/services/graphqlActionError"
import {getPasswordPolicyMessage} from "./editPasswordError"

export type TranslateMessage = (key: string, options?: Record<string, unknown>) => string

export const USER_PROFILE_VALIDATION_ERROR_CODE = "UserProfileValidation"

interface UserProfileValidation {
    field?: string
    error?: string
    params: unknown[]
}

interface UserProfileValidations {
    /** The refused attributes Harvest reported, capped on its side. */
    reported: UserProfileValidation[]
    /** Everything Keycloak refused, which can exceed what was reported. */
    total: number
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
    "error-invalid-value": "invalid",
}

const readValidation = (value: unknown): UserProfileValidation | undefined => {
    const entry = value as {field?: unknown; error?: unknown; params?: unknown} | undefined
    // Only an entry naming the attribute it refused is one of these, the same
    // way Harvest decides it.
    if (typeof entry !== "object" || entry === null || typeof entry.field !== "string") {
        return undefined
    }

    return {
        field: entry.field,
        error: typeof entry.error === "string" ? entry.error : undefined,
        params: Array.isArray(entry.params) ? entry.params : [],
    }
}

const readValidations = (value: unknown): UserProfileValidations | undefined => {
    const extensions = value as
        | {
              code?: unknown
              user_profile_errors?: unknown
              user_profile_errors_total?: unknown
          }
        | undefined
    if (extensions?.code !== USER_PROFILE_VALIDATION_ERROR_CODE) {
        return undefined
    }

    const reported = (
        Array.isArray(extensions.user_profile_errors) ? extensions.user_profile_errors : []
    )
        .map(readValidation)
        .filter((entry): entry is UserProfileValidation => entry !== undefined)
    if (reported.length === 0) {
        return undefined
    }

    const total = extensions.user_profile_errors_total
    return {
        reported,
        total: typeof total === "number" && total > reported.length ? total : reported.length,
    }
}

/**
 * The user profile constraint Keycloak refused the write against, when it named
 * one. Harvest forwards the attribute and the constraint's arguments, so the
 * admin can be told which field to correct rather than that something failed.
 */
const getUserProfileValidations = (error: unknown): UserProfileValidations | undefined => {
    const actionError = error as IGraphQLActionError | undefined
    const graphQLErrors = Array.isArray(actionError?.graphQLErrors) ? actionError.graphQLErrors : []

    for (const graphQLError of graphQLErrors) {
        const direct = readValidations(graphQLError.extensions)
        if (direct) {
            return direct
        }

        const responseBody = parseActionResponseBody(
            graphQLError.extensions?.internal?.response?.body
        ) as {extensions?: unknown} | undefined
        const fromBody = readValidations(responseBody?.extensions)
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

const describeValidation = (
    validation: UserProfileValidation,
    t: TranslateMessage,
    resolveFieldLabel?: (field: string) => string
): string => {
    const field = validation.field ?? ""
    const messageKey = CONSTRAINT_MESSAGES[validation.error ?? ""] ?? "invalid"

    return t(`usersAndRolesScreen.voters.errors.attribute.${messageKey}`, {
        field: resolveFieldLabel && field ? resolveFieldLabel(field) : field,
        ...interpolation(messageKey, constraintArguments(validation)),
    })
}

const getUserProfileValidationMessage = (
    error: unknown,
    t: TranslateMessage,
    resolveFieldLabel?: (field: string) => string
): string | undefined => {
    const validations = getUserProfileValidations(error)
    if (!validations) {
        return undefined
    }

    // Harvest's own message names every field and the constraint it broke, so
    // it beats saying only that something was invalid.
    if (validations.reported.some((validation) => !CONSTRAINT_MESSAGES[validation.error ?? ""])) {
        return undefined
    }

    const described = validations.reported
        .map((validation) => describeValidation(validation, t, resolveFieldLabel))
        .join("; ")
    const unreported = validations.total - validations.reported.length

    // Keycloak refuses every attribute at once, so a mis-mapped import can
    // produce more than a message can carry. Say so rather than let the rest
    // pass unmentioned.
    return unreported > 0
        ? `${described}; ${t("usersAndRolesScreen.voters.errors.attribute.andMore", {
              count: unreported,
          })}`
        : described
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
