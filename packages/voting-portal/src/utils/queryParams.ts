// SPDX-FileCopyrightText: 2024 Sequent Tech <leaal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const getLanguageFromURL = () => {
    const params = new URLSearchParams(window.location.search)
    return params.get("lang") || undefined
}
