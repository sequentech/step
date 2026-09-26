// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** A Hasura boolean expression that ra-data-hasura passes through unchanged. */
interface HasuraRawQuery {
    format: "hasura-raw-query"
    value: Record<string, unknown>[]
}

/** An election search that also keeps the text typed, for the filter input to show. */
export interface ElectionSearchQuery extends HasuraRawQuery {
    text: string
}

/**
 * Election events, elections, contests and candidates keep their translated
 * names in `presentation` and their former alias in `external_id` (migration
 * 1772358027729), so the search matches either.
 */
export function electionSearchQuery(searchText: string): ElectionSearchQuery | undefined {
    const text = searchText.trim()
    if (!text) return undefined
    const pattern = `%${text}%`
    return {
        format: "hasura-raw-query",
        text: searchText,
        value: [
            {external_id: {_ilike: pattern}},
            {presentation: {_cast: {String: {_ilike: pattern}}}},
        ],
    }
}

/** The text of an election search, for the filter input that set it. */
export const electionSearchText = (query?: ElectionSearchQuery | null): string => query?.text ?? ""

/** The react-admin filter of an election autocomplete. */
export function electionSearchFilter(searchText: string): {_or?: ElectionSearchQuery} {
    const query = electionSearchQuery(searchText)
    return query ? {_or: query} : {}
}
