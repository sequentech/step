// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//Auth
// should be updated per event
export const loginUrl =
    "http://localhost:3000/tenant/90505c8a-23a9-4cdf-a26b-4e19f6a097d5/event/082f17f4-d873-467a-a8a5-596e10a23a38/login"
export const username = "edulix@nvotes.com"
export const password = "12345"

export const electionEventLink = "sequent_backend_election_event"
export const electionLink = "sequent_backend_election"
export const contestLink = "sequent_backend_contest"
export const candidateLink = "sequent_backend_candidate"

export const pause = {
    short: 1000,
    medium: 2000,
    long: 5000,
    xLong: 10000,
}

export type NightWatchLogin = {
    password: string
    username: string
}

export type NightWatchHoverAndClick =
    | string
    | {
          hoverElement: string
          clickElement: string
      }
