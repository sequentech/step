// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {hasGraphQLActionErrorCode} from "@/services/graphqlActionError"

export const PRIVATE_KEY_DOWNLOAD_UNAVAILABLE_ERROR_CODE = "PrivateKeyDownloadUnavailable"

export const isPrivateKeyDownloadUnavailableError = (error: unknown): boolean =>
    hasGraphQLActionErrorCode(error, PRIVATE_KEY_DOWNLOAD_UNAVAILABLE_ERROR_CODE)
