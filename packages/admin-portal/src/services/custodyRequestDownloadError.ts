// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {hasGraphQLActionErrorCode} from "@/services/graphqlActionError"

export const CUSTODY_REQUEST_DOWNLOAD_UNAVAILABLE_ERROR_CODE = "CustodyRequestDownloadUnavailable"

export const isCustodyRequestDownloadUnavailableError = (error: unknown): boolean =>
    hasGraphQLActionErrorCode(error, CUSTODY_REQUEST_DOWNLOAD_UNAVAILABLE_ERROR_CODE)
