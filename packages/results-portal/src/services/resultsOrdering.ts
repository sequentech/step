// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    IContestPresentation,
    IElectionEventPresentation,
    IElectionPresentation,
    parseEntityPresentation,
    sortByPresentationOrder,
} from "@sequentech/ui-core"
import {ResultsManifest, ResultsRow, ResultsSqliteDataset} from "@/types/results"
import {translatedLabel} from "./resultLabels"

const sameId = (left: unknown, right: unknown): boolean =>
    left !== null &&
    left !== undefined &&
    right !== null &&
    right !== undefined &&
    String(left) === String(right)

const unique = (values: unknown[]): string[] => {
    const seen = new Set<string>()
    return values.reduce<string[]>((output, value) => {
        if (value === null || value === undefined) return output

        const id = String(value)
        if (!seen.has(id)) {
            seen.add(id)
            output.push(id)
        }
        return output
    }, [])
}

const findRow = (rows: ResultsRow[], id: unknown) => rows.find((row) => sameId(row.id, id))

const electionEventPresentation = (
    manifest: ResultsManifest,
    dataset: ResultsSqliteDataset
): IElectionEventPresentation | undefined =>
    parseEntityPresentation<IElectionEventPresentation>(
        dataset.election_event.find((row) => sameId(row.id, manifest.election_event_id))
            ?.presentation ?? dataset.election_event[0]?.presentation
    )

export const orderResultElectionIds = (
    manifest: ResultsManifest,
    dataset: ResultsSqliteDataset,
    locale: string
): string[] => {
    const ids = unique([
        ...manifest.election_ids,
        ...manifest.contests.map((contest) => contest.election_id),
    ])
    const order = electionEventPresentation(manifest, dataset)?.elections_order

    return sortByPresentationOrder(ids, order, {
        getLabel: (id) => translatedLabel(findRow(dataset.election, id), locale, id),
        getPresentation: (id) => findRow(dataset.election, id)?.presentation,
    })
}

export const orderResultContestIds = (
    manifest: ResultsManifest,
    dataset: ResultsSqliteDataset,
    electionId: unknown,
    locale: string
): string[] => {
    const ids = unique(
        manifest.contests
            .filter((contest) => sameId(contest.election_id, electionId))
            .map((contest) => contest.contest_id)
    )
    const election = findRow(dataset.election, electionId)
    const order = parseEntityPresentation<IElectionPresentation>(
        election?.presentation
    )?.contests_order

    return sortByPresentationOrder(ids, order, {
        getLabel: (id) => translatedLabel(findRow(dataset.contest, id), locale, id),
        getPresentation: (id) => findRow(dataset.contest, id)?.presentation,
    })
}

export const orderResultCandidates = (
    dataset: ResultsSqliteDataset,
    contestId: unknown,
    locale: string
): ResultsRow[] => {
    const contest = findRow(dataset.contest, contestId)
    const order = parseEntityPresentation<IContestPresentation>(
        contest?.presentation
    )?.candidates_order
    const candidates = dataset.candidate.filter((candidate) =>
        sameId(candidate.contest_id, contestId)
    )

    return sortByPresentationOrder(candidates, order, {
        getLabel: (candidate) => translatedLabel(candidate, locale),
        getPresentation: (candidate) => candidate.presentation,
    })
}

export const orderElectionResultRows = <T extends ResultsRow>(
    rows: readonly T[],
    manifest: ResultsManifest,
    dataset: ResultsSqliteDataset,
    locale: string
): T[] => {
    const order = electionEventPresentation(manifest, dataset)?.elections_order
    const snapshotOrder = new Map(
        unique([
            ...manifest.election_ids,
            ...manifest.contests.map((contest) => contest.election_id),
        ]).map((id, index) => [id, index])
    )
    const snapshotPosition = (electionId: unknown): number =>
        electionId === null || electionId === undefined
            ? Number.MAX_SAFE_INTEGER
            : (snapshotOrder.get(String(electionId)) ?? Number.MAX_SAFE_INTEGER)
    const snapshotRows = [...rows].sort((left, right) => {
        return snapshotPosition(left.election_id) - snapshotPosition(right.election_id)
    })

    return sortByPresentationOrder(snapshotRows, order, {
        getLabel: (row) =>
            translatedLabel(findRow(dataset.election, row.election_id), locale, String(row.id)),
        getPresentation: (row) => findRow(dataset.election, row.election_id)?.presentation,
    })
}
