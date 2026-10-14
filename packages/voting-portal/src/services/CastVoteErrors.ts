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

// The review screen's text for a ballot the ballot box did not accept, whether
// it was being received or cast.
export const castVoteErrorMessage = (error: unknown, t: TFunction): string => {
    let castError = error as IGraphQLActionError
    let errorExtensions = (
        CombinedGraphQLErrors.is(error)
            ? error.errors[0]?.extensions
            : castError?.graphQLErrors?.[0]?.extensions
    ) as IExtensionError | undefined
    if (castError?.message?.includes("internal error")) {
        return t(`reviewScreen.error.${CastBallotsErrorType.INTERNAL_ERROR}`) // can happen if the backend panics
    } else if (errorExtensions?.code) {
        let errorCode = errorExtensions?.code
        console.log(castError.name, castError.message)
        let internalErrMessage = errorExtensions?.internal?.error?.message
        console.log(errorCode, internalErrMessage)
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
