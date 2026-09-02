// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {isString} from "@sequentech/ui-core"
import {useTranslation} from "react-i18next"

export const sortFunction = (a: {name?: string | null}, b: {name?: string | null}) => {
    if (isString(a?.name) && isString(b?.name)) {
        return a.name.localeCompare(b.name)
    }
    return 0
}

export interface ISharedValidationError {
    code: string
    message: string
    field?: string | null
    params?: Record<string, string> | null
}

// Error codes emitted by sequent_core::services::tally_sheet_validation.
// The values must match the Rust side exactly.
export const VALIDATION_ERROR_CODES = {
    INVALID_TOTAL_VALID_VOTES: "invalid_total_valid_votes",
    TOTAL_VOTES_EXCEEDS_CENSUS: "total_votes_exceeds_census",
    INVALID_TOTAL_INVALID: "invalid_total_invalid",
    INVALID_TOTAL_VOTES: "invalid_total_votes",
    UNKNOWN_COUNTING_ALGORITHM: "unknown_counting_algorithm",
} as const

// Maps a shared tally sheet validation error's `code` (from
// sequent_core::services::tally_sheet_validation, used by both manual entry
// and tally sheet import) to the i18n key used to render it. Codes without
// an entry here fall back to `tallysheet.inputError.<code>`.
const VALIDATION_ERROR_TRANSLATION_KEYS: Record<string, string> = {
    [VALIDATION_ERROR_CODES.INVALID_TOTAL_VALID_VOTES]:
        "tallysheet.inputError.totalValidDoesNotMatch",
    [VALIDATION_ERROR_CODES.TOTAL_VOTES_EXCEEDS_CENSUS]: "tallysheet.inputError.censusTooSmall",
    [VALIDATION_ERROR_CODES.INVALID_TOTAL_INVALID]:
        "tallysheet.inputError.totalInvalidDoesNotMatch",
    [VALIDATION_ERROR_CODES.INVALID_TOTAL_VOTES]: "tallysheet.inputError.totalVotesDoesNotMatch",
    [VALIDATION_ERROR_CODES.UNKNOWN_COUNTING_ALGORITHM]:
        "tallysheet.inputError.unknownCountingAlgorithm",
}

// `t` is typed loosely (matching react-i18next's `useTranslation().t`)
// rather than importing i18next's TFunction, to avoid a new type
// dependency for a single helper.
export const translateSharedValidationError = (
    t: ReturnType<typeof useTranslation>["t"],
    error: ISharedValidationError
): string => {
    const translationKey =
        VALIDATION_ERROR_TRANSLATION_KEYS[error.code] ?? `tallysheet.inputError.${error.code}`
    return String(
        t(translationKey, {
            ...(error.params ?? {}),
            defaultValue: error.message,
        })
    )
}
