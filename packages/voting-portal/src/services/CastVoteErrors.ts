// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {CombinedGraphQLErrors} from "@apollo/client/errors"
import {
    EGraphQLErrorCode,
    EGraphQLInternalErrorMessage,
    IExtensionError,
    IGraphQLActionError,
} from "@sequentech/ui-core"
import {TFunction} from "i18next"
import {isApolloTransportError} from "./ApolloErrors"
import {CastBallotsErrorType} from "./VotingPortalError"

// Hasura can wrap a non-2xx action response in an `unexpected` error.
// Recognize this public code only; never display arbitrary backend messages.
const wrappedQuotaCode = (extensions: IExtensionError): string | undefined => {
    if (extensions.code !== EGraphQLErrorCode.UNEXPECTED) return undefined
    const body: unknown = extensions.internal?.response?.body
    try {
        const parsed: unknown = typeof body === "string" ? JSON.parse(body) : body
        if (!parsed || typeof parsed !== "object" || !("extensions" in parsed)) return undefined
        const inner = parsed.extensions
        if (!inner || typeof inner !== "object" || !("code" in inner)) return undefined
        return inner.code === "InsertFailedExceedsAllowedRevotes" ? inner.code : undefined
    } catch {
        return undefined
    }
}

// The review screen's text for a ballot the ballot box did not accept, whether
// it was being received or cast.
export const castVoteErrorMessage = (error: unknown, t: TFunction): string => {
    let castError = error as IGraphQLActionError
    let errorExtensions = (
        CombinedGraphQLErrors.is(error)
            ? error.errors[0]?.extensions
            : castError?.graphQLErrors?.[0]?.extensions
    ) as IExtensionError | undefined
    const publicCode =
        errorExtensions && (wrappedQuotaCode(errorExtensions) ?? errorExtensions.code)
    // Hasura may label its wrapper "internal error" while retaining this
    // explicit public refusal. Never replace it with the generic panic text.
    if (publicCode === "InsertFailedExceedsAllowedRevotes") {
        return t(`reviewScreen.error.${CastBallotsErrorType.CAST_VOTE}_${publicCode}`)
    }
    if (castError?.message?.includes("internal error")) {
        return t(`reviewScreen.error.${CastBallotsErrorType.INTERNAL_ERROR}`) // can happen if the backend panics
    } else if (errorExtensions?.code) {
        let errorCode = wrappedQuotaCode(errorExtensions) ?? errorExtensions.code
        let internalErrMessage = errorExtensions?.internal?.error?.message
        if (
            errorCode === EGraphQLErrorCode.UNEXPECTED &&
            internalErrMessage === EGraphQLInternalErrorMessage.TIMEOUT_ERROR
        ) {
            return t(`reviewScreen.error.${CastBallotsErrorType.CAST_VOTE_TIMEOUT}`)
        }
        return t(`reviewScreen.error.${CastBallotsErrorType.CAST_VOTE}_${errorCode}`)
    } else if (
        isApolloTransportError(error instanceof Error ? error : undefined) ||
        (error && typeof error === "object" && "networkError" in error && error.networkError)
    ) {
        return t(`reviewScreen.error.${CastBallotsErrorType.NETWORK_ERROR}`)
    }
    return t(`reviewScreen.error.${CastBallotsErrorType.CAST_VOTE}`) // Generic error
}
