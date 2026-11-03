// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/** Prefix trustee downloads with an identity safe for filenames and command examples. */
export const trusteeFilename = (trustee: string, artifact: string): string => {
    const name = trustee
        .replace(new RegExp("[^\\p{L}\\p{N}._-]+", "gu"), "-")
        .replace(/^[._-]+|[._-]+$/g, "")
    return `${name || "trustee"}-${artifact}`
}
