// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useMemo, useState} from "react"
import {Box, Tab, Tabs, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {
    ResultsManifest,
    ResultsManifestContest,
    ResultsRow,
    ResultsSqliteDataset,
} from "@/types/results"
import {translatedLabel} from "@/services/resultLabels"
import {ContestResultsBlock} from "./ContestResultsBlock"

interface ResultsSelectorTabsProps {
    manifest: ResultsManifest
    dataset: ResultsSqliteDataset
    locale: string
}

interface TabOption {
    id: string
    label: string
}

interface AreaTabOption {
    id: string | null
    label: string
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

const optionKey = (value: string | null) => value ?? "__global__"

const findRow = (rows: ResultsRow[], id: string | null | undefined) =>
    id ? rows.find((row) => sameId(row.id, id)) : undefined

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
    const electionOptions = useMemo<TabOption[]>(
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
    const [selectedElectionId, setSelectedElectionId] = useState<string | null>(
        electionOptions[0]?.id ?? null
    )
    const selectedElectionIndex = Math.max(
        0,
        electionOptions.findIndex((option) => sameId(option.id, selectedElectionId))
    )

    useEffect(() => {
        if (!electionOptions.length) {
            setSelectedElectionId(null)
            return
        }

        if (!electionOptions.some((option) => sameId(option.id, selectedElectionId))) {
            setSelectedElectionId(electionOptions[0].id)
        }
    }, [electionOptions, selectedElectionId])

    const contestManifests = useMemo(
        () =>
            manifest.contests.filter((contest) => sameId(contest.election_id, selectedElectionId)),
        [manifest.contests, selectedElectionId]
    )
    const contestIds = useMemo(
        () => unique(contestManifests.map((contest) => contest.contest_id)),
        [contestManifests]
    )
    const contestOptions = useMemo<TabOption[]>(
        () =>
            contestIds.map((contestId) => ({
                id: contestId,
                label: translatedLabel(
                    findRow(dataset.contest, contestId),
                    locale,
                    t("resultsPortal.fallbackContestName", {contestId})
                ),
            })),
        [contestIds, dataset.contest, locale, t]
    )
    const [selectedContestId, setSelectedContestId] = useState<string | null>(
        contestOptions[0]?.id ?? null
    )
    const selectedContestIndex = Math.max(
        0,
        contestOptions.findIndex((option) => sameId(option.id, selectedContestId))
    )

    useEffect(() => {
        if (!contestOptions.length) {
            setSelectedContestId(null)
            return
        }

        if (!contestOptions.some((option) => sameId(option.id, selectedContestId))) {
            setSelectedContestId(contestOptions[0].id)
        }
    }, [contestOptions, selectedContestId])

    const selectedGlobalManifestContest = useMemo(
        () =>
            contestManifests.find(
                (contest) => sameId(contest.contest_id, selectedContestId) && !contest.area_id
            ) ?? contestManifests.find((contest) => sameId(contest.contest_id, selectedContestId)),
        [contestManifests, selectedContestId]
    )
    const areaIds = useMemo(
        () =>
            unique([
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
            ]),
        [contestManifests, dataset.results_area_contest, selectedContestId, selectedElectionId]
    )
    const areaOptions = useMemo<AreaTabOption[]>(
        () => [
            {
                id: null,
                label: t("resultsPortal.globalArea"),
            },
            ...areaIds.map((areaId) => ({
                id: areaId,
                label: translatedLabel(findRow(dataset.area, areaId), locale, areaId),
            })),
        ],
        [areaIds, dataset.area, locale, t]
    )
    const [selectedAreaId, setSelectedAreaId] = useState<string | null>(null)
    const selectedAreaIndex = Math.max(
        0,
        areaOptions.findIndex((option) => optionKey(option.id) === optionKey(selectedAreaId))
    )

    useEffect(() => {
        if (!areaOptions.length) {
            setSelectedAreaId(null)
            return
        }

        if (!areaOptions.some((option) => optionKey(option.id) === optionKey(selectedAreaId))) {
            setSelectedAreaId(areaOptions[0].id)
        }
    }, [areaOptions, selectedAreaId])

    const selectedManifestContest = useMemo<ResultsManifestContest | null>(() => {
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
                selectedGlobalManifestContest.publication_state === "published" && hasAreaResults
                    ? "published"
                    : "not_published",
        }
    }, [
        contestManifests,
        dataset.results_area_contest,
        selectedAreaId,
        selectedContestId,
        selectedElectionId,
        selectedGlobalManifestContest,
    ])

    if (!electionOptions.length) {
        return (
            <Typography className="seq-results-selector__empty" color="text.secondary">
                {t("resultsPortal.noResultsForSelection")}
            </Typography>
        )
    }

    return (
        <Box className="seq-results-selector">
            <Box className="seq-results-selector__tabs-row">
                <Typography
                    className="seq-results-selector__tabs-label"
                    variant="body2"
                    component="div"
                    sx={{width: 88, flexShrink: 0}}
                >
                    {t("resultsPortal.electionsTitle")}.
                </Typography>
                <Tabs
                    className="seq-results-selector__election-tabs"
                    value={selectedElectionIndex}
                    variant="scrollable"
                    scrollButtons="auto"
                    onChange={(_, index) =>
                        setSelectedElectionId(electionOptions[index]?.id ?? null)
                    }
                    sx={{flex: 1, minWidth: 0}}
                >
                    {electionOptions.map((option) => (
                        <Tab
                            className="seq-results-selector__election-tab"
                            key={option.id}
                            label={option.label}
                        />
                    ))}
                </Tabs>
            </Box>

            <Box className="seq-results-selector__tabs-row">
                <Typography
                    className="seq-results-selector__tabs-label"
                    variant="body2"
                    component="div"
                    sx={{width: 88, flexShrink: 0}}
                >
                    {t("resultsPortal.contestsTitle")}.
                </Typography>
                <Tabs
                    className="seq-results-selector__contest-tabs"
                    value={contestOptions.length ? selectedContestIndex : false}
                    variant="scrollable"
                    scrollButtons="auto"
                    onChange={(_, index) => setSelectedContestId(contestOptions[index]?.id ?? null)}
                    sx={{flex: 1, minWidth: 0}}
                >
                    {contestOptions.map((option) => (
                        <Tab
                            className="seq-results-selector__contest-tab"
                            key={option.id}
                            label={option.label}
                        />
                    ))}
                </Tabs>
            </Box>

            <Box className="seq-results-selector__tabs-row">
                <Typography
                    className="seq-results-selector__tabs-label"
                    variant="body2"
                    component="div"
                    sx={{width: 88, flexShrink: 0}}
                >
                    {t("resultsPortal.areasTitle")}.
                </Typography>
                <Tabs
                    className="seq-results-selector__area-tabs"
                    value={selectedAreaIndex}
                    variant="scrollable"
                    scrollButtons="auto"
                    onChange={(_, index) => setSelectedAreaId(areaOptions[index]?.id ?? null)}
                    sx={{flex: 1, minWidth: 0}}
                >
                    {areaOptions.map((option) => (
                        <Tab
                            className="seq-results-selector__area-tab"
                            key={optionKey(option.id)}
                            label={option.label}
                        />
                    ))}
                </Tabs>
            </Box>

            <Box className="seq-results-selector__selected-result">
                {selectedManifestContest ? (
                    <ContestResultsBlock
                        manifestContest={selectedManifestContest}
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
                )}
            </Box>
        </Box>
    )
}
