// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const getLanguageFromURL = () => {
    const params = new URLSearchParams(window.location.search)
    return params.get("lang") || undefined
}
