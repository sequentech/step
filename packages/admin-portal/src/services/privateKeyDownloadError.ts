// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IGraphQLActionError} from "@sequentech/ui-core"
import {parseActionResponseBody} from "./graphqlActionError"

export const PRIVATE_KEY_DOWNLOAD_UNAVAILABLE_ERROR_CODE = "PrivateKeyDownloadUnavailable"

export const isPrivateKeyDownloadUnavailableError = (error: unknown): boolean => {
    const actionError = error as IGraphQLActionError | undefined

    return (
        actionError?.graphQLErrors?.some((graphQLError) => {
            if (graphQLError.extensions?.code === PRIVATE_KEY_DOWNLOAD_UNAVAILABLE_ERROR_CODE) {
                return true
            }

            const responseBody = parseActionResponseBody(
                graphQLError.extensions?.internal?.response?.body
            ) as {extensions?: {code?: unknown}} | undefined

            return responseBody?.extensions?.code === PRIVATE_KEY_DOWNLOAD_UNAVAILABLE_ERROR_CODE
        }) === true
    )
}
