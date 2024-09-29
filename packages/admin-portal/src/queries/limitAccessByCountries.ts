// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const LIMIT_ACCESS_BY_COUNTRIES = gql`
    mutation limitAccessByCountries($countries: [String!]!) {
        limit_access_by_countries(countries: $countries) {
            success
        }
    }
`
