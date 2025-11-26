// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//Auth
export const loginUrl =
    "http://127.0.0.1:8090/realms/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-082f17f4-d873-467a-a8a5-596e10a23a38/protocol/openid-connect/auth?client_id=voting-portal&redirect_uri=http%3A%2F%2F127.0.0.1%3A3001%2Ftenant%2F90505c8a-23a9-4cdf-a26b-4e19f6a097d5%2Fevent%2F082f17f4-d873-467a-a8a5-596e10a23a38%2Fstart&state=10b44891-8480-4d03-83a2-f95be66b418a&response_mode=fragment&response_type=code&scope=openid&nonce=80e33658-b83a-4690-86ef-472158457d44"
export const username = "edulix@nvotes.com"
export const password = "12345"

export const fileDownloadDir = "/Users/kong/Downloads/"
export const ballot_id = "dc2c427b3607dea9561a8358b9ac9d6aef387817cd11d7341629a2a0fe96d673"

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
