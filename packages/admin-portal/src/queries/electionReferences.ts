// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// A voter's `authorized-election-ids` hold an election's external id or its id,
// depending on how the voter was added.

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i

export interface IElectionRow {
    id: string
    external_id?: string | null
    presentation?: unknown
}

export type IElectionReference<T extends IElectionRow> =
    | (T & {election_id: string})
    | {id: string; election_id: null; external_id: string; presentation: null}

/** The values that can be an election's id. */
export const electionReferenceIds = (references: string[]): string[] =>
    references.filter((reference) => UUID.test(reference))

/**
 * One record per value, under the value itself, so a list of values finds its
 * elections. A value that names no election is kept, to show it as it is.
 */
export function matchElectionReferences<T extends IElectionRow>(
    references: string[],
    rows: T[]
): IElectionReference<T>[] {
    return Array.from(new Set(references)).map((reference) => {
        const row =
            rows.find((election) => election.external_id === reference) ??
            rows.find((election) => election.id.toLowerCase() === reference.toLowerCase())
        return row
            ? {...row, id: reference, election_id: row.id}
            : {id: reference, election_id: null, external_id: reference, presentation: null}
    })
}
