// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const electionEventLink = "sequent_backend_election_event"
export const electionLink = "sequent_backend_election"
export const contestLink = "sequent_backend_contest"
export const candidateLink = "sequent_backend_candidate"
export const testUrl = process.env.TEST_URL || "http://localhost:3002"
export const admin_portal_username = process.env.ADMIN_PORTAL_TEST_USERNAME || "admin"
export const admin_portal_password = process.env.ADMIN_PORTAL_TEST_PASSWORD || "admin"
export const voterDetails = {
    firstName: "this is an voter firstname",
    lastName: "this is an voter lastname",
    email: "thisisavoter@voter.com",
    username: "voterusername",
}

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
