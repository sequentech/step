// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const getElectionEventId = async function (browser) {
    const currentUrl = await browser.url(function (result) {
        return result.value
    })

    const electionEventIdMatch = currentUrl.match(/sequent_backend_election_event\/([^\/]+)/)
    if (electionEventIdMatch) {
        const electionEventId = electionEventIdMatch[1]
        return electionEventId
    } else {
        console.log("Election event ID not found in the URL")
        return null
    }
}
