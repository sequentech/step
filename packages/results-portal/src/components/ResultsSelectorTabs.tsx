// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useMemo} from "react"
import {Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {ResultsSelectorTabs as SharedResultsSelectorTabs} from "@sequentech/ui-essentials"
import type {
    ResultsSelectorAreaOption,
    ResultsSelectorOption,
    ResultsSelectorSelection,
} from "@sequentech/ui-essentials"
import {ResultsManifest, ResultsRow, ResultsSqliteDataset} from "@/types/results"
import {translatedLabel} from "@/services/resultLabels"
import {ContestResultsBlock} from "./ContestResultsBlock"

interface ResultsSelectorTabsProps {
    manifest: ResultsManifest
    dataset: ResultsSqliteDataset
    locale: string
}

const sameId = (left: unknown, right: unknown): boolean =>
    left !== null &&
    left !== undefined &&
    right !== null &&
    right !== undefined &&
    String(left) === String(right)

const unique = (values: Array<string | null | undefined>): string[] => {
    const seen = new Set<string>()
    const output: string[] = []

    values.forEach((value) => {
        if (!value) return
        const id = String(value)
        if (seen.has(id)) return

        seen.add(id)
        output.push(id)
    })

    return output
}

const findRow = (rows: ResultsRow[], id: string | null | undefined) =>
    id ? rows.find((row) => sameId(row.id, id)) : undefined

type PortalSelection = ResultsSelectorSelection<unknown, unknown, unknown>

export const ResultsSelectorTabs: React.FC<ResultsSelectorTabsProps> = ({
    manifest,
    dataset,
    locale,
}) => {
    const {t} = useTranslation()
    const electionIds = useMemo(
        () =>
            unique([
                ...manifest.election_ids,
                ...manifest.contests.map((contest) => contest.election_id),
            ]),
        [manifest.election_ids, manifest.contests]
    )
    const electionOptions = useMemo<ResultsSelectorOption[]>(
        () =>
            electionIds.map((electionId) => ({
                id: electionId,
                label: translatedLabel(
                    findRow(dataset.election, electionId),
                    locale,
                    t("resultsPortal.fallbackElectionName")
                ),
            })),
        [dataset.election, electionIds, locale, t]
    )

    const getContestOptions = useCallback(
        (selection: Pick<PortalSelection, "election">): ResultsSelectorOption[] => {
            const selectedElectionId = selection.election?.id ?? null
            const contestManifests = manifest.contests.filter((contest) =>
                sameId(contest.election_id, selectedElectionId)
            )
            const contestIds = unique(contestManifests.map((contest) => contest.contest_id))

            return contestIds.map((contestId) => ({
                id: contestId,
                label: translatedLabel(
                    findRow(dataset.contest, contestId),
                    locale,
                    t("resultsPortal.fallbackContestName", {contestId})
                ),
            }))
        },
        [dataset.contest, locale, manifest.contests, t]
    )

    const getAreaOptions = useCallback(
        (selection: Pick<PortalSelection, "election" | "contest">): ResultsSelectorAreaOption[] => {
            const selectedElectionId = selection.election?.id ?? null
            const selectedContestId = selection.contest?.id ?? null
            const contestManifests = manifest.contests.filter((contest) =>
                sameId(contest.election_id, selectedElectionId)
            )
            const areaIds = unique([
                ...contestManifests
                    .filter((contest) => sameId(contest.contest_id, selectedContestId))
                    .map((contest) => contest.area_id),
                ...dataset.results_area_contest
                    .filter(
                        (result) =>
                            sameId(result.election_id, selectedElectionId) &&
                            sameId(result.contest_id, selectedContestId)
                    )
                    .map((result) => result.area_id),
            ])

            return [
                {
                    id: null,
                    label: t("resultsPortal.globalArea"),
                },
                ...areaIds.map((areaId) => ({
                    id: areaId,
                    label: translatedLabel(findRow(dataset.area, areaId), locale, areaId),
                })),
            ]
        },
        [dataset.area, dataset.results_area_contest, locale, manifest.contests, t]
    )

    const selectedManifestContest = useCallback(
        (selection: PortalSelection) => {
            const selectedElectionId = selection.election?.id ?? null
            const selectedContestId = selection.contest?.id ?? null
            const selectedAreaId = selection.area?.id ?? null
            const contestManifests = manifest.contests.filter((contest) =>
                sameId(contest.election_id, selectedElectionId)
            )
            const selectedGlobalManifestContest =
                contestManifests.find(
                    (contest) => sameId(contest.contest_id, selectedContestId) && !contest.area_id
                ) ??
                contestManifests.find((contest) => sameId(contest.contest_id, selectedContestId))

            if (!selectedGlobalManifestContest) {
                return null
            }

            if (!selectedAreaId) {
                return selectedGlobalManifestContest
            }

            const manifestContest = contestManifests.find(
                (contest) =>
                    sameId(contest.contest_id, selectedContestId) &&
                    sameId(contest.area_id, selectedAreaId)
            )
            if (manifestContest) {
                return manifestContest
            }

            const hasAreaResults = dataset.results_area_contest.some(
                (result) =>
                    sameId(result.election_id, selectedElectionId) &&
                    sameId(result.contest_id, selectedContestId) &&
                    sameId(result.area_id, selectedAreaId)
            )

            return {
                ...selectedGlobalManifestContest,
                area_id: selectedAreaId,
                publication_state:
                    selectedGlobalManifestContest.publication_state === "published" &&
                    hasAreaResults
                        ? "published"
                        : "not_published",
            }
        },
        [dataset.results_area_contest, manifest.contests]
    )

    return (
        <SharedResultsSelectorTabs
            className="seq-results-portal-results-selector"
            labels={{
                elections: t("resultsPortal.electionsTitle"),
                contests: t("resultsPortal.contestsTitle"),
                areas: t("resultsPortal.areasTitle"),
                empty: t("resultsPortal.noResultsForSelection"),
            }}
            elections={electionOptions}
            getContests={getContestOptions}
            getAreas={getAreaOptions}
            renderResult={(selection) => {
                const manifestContest = selectedManifestContest(selection)

                return manifestContest ? (
                    <ContestResultsBlock
                        manifestContest={manifestContest}
                        dataset={dataset}
                        locale={locale}
                    />
                ) : (
                    <Typography
                        className="seq-results-selector__empty"
                        color="text.secondary"
                        sx={{mt: 3}}
                    >
                        {t("resultsPortal.noResultsForSelection")}
                    </Typography>
                )
            }}
        />
    )
}
