// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const FOLDER_MAX_CHARS = 200 // Define the maximum number of characters

export const takeLastNChars = (s: string, n: number): string => {
    return s.slice(-n)
}

export const sanitizeFilename = (filename: string, max = FOLDER_MAX_CHARS): string => {
    // Regular expression to match invalid filename characters
    // eslint-disable-next-line no-control-regex
    const invalidCharsRegex = /[<>:"/\\|?*\x00-\x1F]/g

    // Replace invalid characters with an underscore or remove them
    const sanitized = filename
        .replace(invalidCharsRegex, "") // Remove invalid characters
        .replace(/[\s.]+$/, "") // Remove trailing spaces and dots

    return takeLastNChars(sanitized, max)
}
