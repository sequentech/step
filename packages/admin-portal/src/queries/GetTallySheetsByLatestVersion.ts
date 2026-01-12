// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const LIST_TALLY_SHEETS_BY_LATEST_VERSION = gql`
    query GetLatestTallySheets(
        $limit: Int
        $offset: Int
        $where: sequent_backend_tally_sheet_bool_exp
    ) {
        items: sequent_backend_tally_sheet(
            limit: $limit
            offset: $offset
            where: $where
            distinct_on: [tenant_id, election_event_id, election_id, contest_id, area_id, channel]
            order_by: [
                {tenant_id: asc}
                {election_event_id: asc}
                {election_id: asc}
                {contest_id: asc}
                {area_id: asc}
                {channel: asc}
                {version: desc}
                {last_updated_at: desc}
                {id: desc}
            ]
        ) {
            annotations
            area_id
            channel
            content
            contest_id
            created_at
            created_by_user_id
            deleted_at
            election_event_id
            election_id
            id
            labels
            last_updated_at
            reviewed_at
            reviewed_by_user_id
            status
            tenant_id
            version
            __typename
        }

        total: sequent_backend_tally_sheet_aggregate(where: $where) {
            aggregate {
                count
            }
        }
    }
`

/** helpers for sequent_backend_tally_sheet query */

export function removeFromWhere(where: any, key: string) {
    if (!where) return

    // remove top-level version if exists
    if (where[key]) delete where[key]

    // remove clauses inside _and array
    if (Array.isArray(where._and)) {
        where._and = where._and.filter((clause: any) => clause?.[key] === undefined)
    }
}

export function addOneDay(yyyyMmDd: string) {
    const d = new Date(`${yyyyMmDd}T00:00:00Z`)
    d.setUTCDate(d.getUTCDate() + 1)
    return d.toISOString().slice(0, 10) // "YYYY-MM-DD"
}
