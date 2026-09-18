// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IGraphQLActionError} from "@sequentech/ui-core"

export const PRIVATE_KEY_DOWNLOAD_UNAVAILABLE_ERROR_CODE = "PrivateKeyDownloadUnavailable"

export const isPrivateKeyDownloadUnavailableError = (error: unknown): boolean => {
    const actionError = error as IGraphQLActionError | undefined

    return (
        actionError?.graphQLErrors?.some(
            (graphQLError) =>
                graphQLError.extensions?.code === PRIVATE_KEY_DOWNLOAD_UNAVAILABLE_ERROR_CODE
        ) === true
    )
}
