// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export interface BallotFilesUrlsOutput {
    election_event_url: string | null
    elections_url: string | null
    ballot_style_urls: Array<string> | null
}

export const fetchJson = async (url: string) => {
    try {
        const response = await fetch(url)
        if (!response.ok) {
            console.log(response)
            throw new Error(`HTTP error! status: ${response.status}`)
        }
        const jsonData = await response.json()
        return jsonData
    } catch (error) {
        console.error("Error fetching JSON:", error)
        throw error
    }
}
