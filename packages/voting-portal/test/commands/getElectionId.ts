// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const getElectionId = async function (browser) {
    const currentUrl = await browser.url(function (result) {
        return result.value
    })
    const electionIdMatch = currentUrl.match(/election\/([^\/]+)/)
    if (electionIdMatch) {
        const electionId = electionIdMatch[1]
        return electionId
    } else {
        return null
    }
}
