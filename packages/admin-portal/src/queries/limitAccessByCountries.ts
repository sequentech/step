// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const LIMIT_ACCESS_BY_COUNTRIES = gql`
    mutation limitAccessByCountries($votingCountries: [String!]!, $enrollCountries: [String!]!) {
        limit_access_by_countries(
            voting_countries: $votingCountries
            enroll_countries: $enrollCountries
        ) {
            success
        }
    }
`
