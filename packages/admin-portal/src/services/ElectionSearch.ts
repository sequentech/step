// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** A Hasura boolean expression that ra-data-hasura passes through unchanged. */
interface HasuraRawQuery {
    format: "hasura-raw-query"
    value: Record<string, unknown>[]
}

/**
 * The react-admin filter of an election autocomplete. Elections keep their
 * translated names in `presentation` and their former alias in `external_id`
 * (migration 1772358027729), so the search matches either.
 */
export function electionSearchFilter(searchText: string): {_or?: HasuraRawQuery} {
    const text = searchText.trim()
    if (!text) return {}
    const pattern = `%${text}%`
    return {
        _or: {
            format: "hasura-raw-query",
            value: [
                {external_id: {_ilike: pattern}},
                {presentation: {_cast: {String: {_ilike: pattern}}}},
            ],
        },
    }
}
