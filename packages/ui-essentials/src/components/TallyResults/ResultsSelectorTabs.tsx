// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useMemo, useState} from "react"
import {Box, Tab, Tabs, Typography} from "@mui/material"

export interface ResultsSelectorOption<TData = unknown> {
    id: string
    label: React.ReactNode
    data?: TData
    disabled?: boolean
}

export interface ResultsSelectorAreaOption<TData = unknown> {
    id: string | null
    label: React.ReactNode
    data?: TData
    disabled?: boolean
}

export interface ResultsSelectorLabels {
    elections: React.ReactNode
    contests: React.ReactNode
    areas: React.ReactNode
    empty: React.ReactNode
}

export interface ResultsSelectorSelection<
    TElection = unknown,
    TContest = unknown,
    TArea = unknown,
> {
    election: ResultsSelectorOption<TElection> | null
    contest: ResultsSelectorOption<TContest> | null
    area: ResultsSelectorAreaOption<TArea> | null
    electionId: string | null
    contestId: string | null
    areaId: string | null
}

export interface ResultsSelectorTabsProps<
    TElection = unknown,
    TContest = unknown,
    TArea = unknown,
> {
    className?: string
    initialElectionId?: string
    labels: ResultsSelectorLabels
    elections: ResultsSelectorOption<TElection>[]
    getContests: (
        selection: Pick<ResultsSelectorSelection<TElection, TContest, TArea>, "election">
    ) => ResultsSelectorOption<TContest>[]
    getAreas: (
        selection: Pick<
            ResultsSelectorSelection<TElection, TContest, TArea>,
            "election" | "contest"
        >
    ) => ResultsSelectorAreaOption<TArea>[]
    renderResult: (
        selection: ResultsSelectorSelection<TElection, TContest, TArea>
    ) => React.ReactNode
    renderElectionActions?: (
        selection: ResultsSelectorSelection<TElection, TContest, TArea>
    ) => React.ReactNode
    renderContestActions?: (
        selection: ResultsSelectorSelection<TElection, TContest, TArea>
    ) => React.ReactNode
    renderAreaActions?: (
        selection: ResultsSelectorSelection<TElection, TContest, TArea>
    ) => React.ReactNode
    onSelectionChange?: (selection: ResultsSelectorSelection<TElection, TContest, TArea>) => void
}

const GLOBAL_AREA_KEY = "__global__"

const areaOptionKey = (id: string | null | undefined) => id ?? GLOBAL_AREA_KEY
const classToken = (id: string | null | undefined) =>
    (id ?? "global").replace(/[^a-zA-Z0-9_-]/g, "-")

const findOption = <TData,>(
    options: ResultsSelectorOption<TData>[],
    id: string | null
): ResultsSelectorOption<TData> | null =>
    id ? (options.find((option) => option.id === id) ?? null) : null

const findAreaOption = <TData,>(
    options: ResultsSelectorAreaOption<TData>[],
    id: string | null
): ResultsSelectorAreaOption<TData> | null =>
    options.find((option) => areaOptionKey(option.id) === areaOptionKey(id)) ?? null

const selectedTabIndex = <TData,>(
    options: Array<ResultsSelectorOption<TData> | ResultsSelectorAreaOption<TData>>,
    selectedKey: string
) => {
    const index = options.findIndex((option) => areaOptionKey(option.id) === selectedKey)
    return index >= 0 ? index : false
}

const TabRow: React.FC<{
    className: string
    label: React.ReactNode
    options: Array<ResultsSelectorOption | ResultsSelectorAreaOption>
    selectedKey: string
    onSelect: (index: number) => void
    actions?: React.ReactNode
    entityName: "election" | "contest" | "area"
}> = ({className, label, options, selectedKey, onSelect, actions, entityName}) => (
    <Box
        className={className}
        sx={{
            borderBottom: 1,
            borderColor: "divider",
            display: "flex",
            flexDirection: "row",
            justifyContent: "flex-start",
            alignItems: "center",
            minWidth: 0,
        }}
    >
        <Typography
            className={`${className}__label`}
            variant="body2"
            component="div"
            sx={{width: 88, flexShrink: 0}}
        >
            {label}.
        </Typography>
        <Tabs
            className={`${className}__tabs`}
            value={selectedTabIndex(options, selectedKey)}
            variant="scrollable"
            scrollButtons="auto"
            onChange={(_, index) => onSelect(index)}
            sx={{flex: 1, minWidth: 0}}
        >
            {options.map((option) => (
                <Tab
                    className={[
                        `${className}__tab`,
                        "seq-results-selector__tab",
                        `seq-results-selector__${entityName}-tab`,
                        `seq-results-selector__${entityName}-tab--${classToken(option.id)}`,
                        areaOptionKey(option.id) === selectedKey
                            ? "seq-results-selector__tab--selected"
                            : null,
                    ]
                        .filter(Boolean)
                        .join(" ")}
                    key={areaOptionKey(option.id)}
                    label={option.label}
                    disabled={option.disabled}
                />
            ))}
        </Tabs>
        {actions ? (
            <Box className={`${className}__actions`} sx={{ml: 1, flexShrink: 0}}>
                {actions}
            </Box>
        ) : null}
    </Box>
)

export const ResultsSelectorTabs = <TElection, TContest, TArea>({
    className,
    initialElectionId,
    labels,
    elections,
    getContests,
    getAreas,
    renderResult,
    renderElectionActions,
    renderContestActions,
    renderAreaActions,
    onSelectionChange,
}: ResultsSelectorTabsProps<TElection, TContest, TArea>): React.JSX.Element => {
    const rootClassName = ["seq-results-selector", className].filter(Boolean).join(" ")
    const [selectedElectionId, setSelectedElectionId] = useState<string | null>(
        findOption(elections, initialElectionId ?? null)?.id ?? elections[0]?.id ?? null
    )
    const selectedElection = useMemo(
        () => findOption(elections, selectedElectionId),
        [elections, selectedElectionId]
    )
    const contestOptions = useMemo(
        () => getContests({election: selectedElection}),
        [getContests, selectedElection]
    )
    const [selectedContestId, setSelectedContestId] = useState<string | null>(
        contestOptions[0]?.id ?? null
    )
    const selectedContest = useMemo(
        () => findOption(contestOptions, selectedContestId),
        [contestOptions, selectedContestId]
    )
    const areaOptions = useMemo(
        () => getAreas({election: selectedElection, contest: selectedContest}),
        [getAreas, selectedContest, selectedElection]
    )
    const [selectedAreaId, setSelectedAreaId] = useState<string | null>(areaOptions[0]?.id ?? null)
    const selectedArea = useMemo(
        () => findAreaOption(areaOptions, selectedAreaId),
        [areaOptions, selectedAreaId]
    )
    const selectElection = (electionId: string | null) => {
        setSelectedElectionId(electionId)
        setSelectedContestId(null)
        setSelectedAreaId(null)
    }
    const selectContest = (contestId: string | null) => {
        setSelectedContestId(contestId)
        setSelectedAreaId(null)
    }
    const selection = useMemo<ResultsSelectorSelection<TElection, TContest, TArea>>(
        () => ({
            election: selectedElection,
            contest: selectedContest,
            area: selectedArea,
            electionId: selectedElection?.id ?? null,
            contestId: selectedContest?.id ?? null,
            areaId: selectedArea?.id ?? null,
        }),
        [selectedArea, selectedContest, selectedElection]
    )

    useEffect(() => {
        onSelectionChange?.(selection)
    }, [onSelectionChange, selection])

    useEffect(() => {
        if (!elections.length) {
            setSelectedElectionId(null)
            return
        }

        if (!findOption(elections, selectedElectionId)) {
            setSelectedElectionId(
                findOption(elections, initialElectionId ?? null)?.id ?? elections[0].id
            )
        }
    }, [elections, initialElectionId, selectedElectionId])

    useEffect(() => {
        if (!contestOptions.length) {
            setSelectedContestId(null)
            return
        }

        if (!findOption(contestOptions, selectedContestId)) {
            setSelectedContestId(contestOptions[0].id)
        }
    }, [contestOptions, selectedContestId])

    useEffect(() => {
        if (!areaOptions.length) {
            setSelectedAreaId(null)
            return
        }

        if (!findAreaOption(areaOptions, selectedAreaId)) {
            setSelectedAreaId(areaOptions[0].id)
        }
    }, [areaOptions, selectedAreaId])

    if (!elections.length) {
        return (
            <Typography className="seq-results-selector__empty" color="text.secondary">
                {labels.empty}
            </Typography>
        )
    }

    return (
        <Box
            className={[
                rootClassName,
                `seq-results-selector--election-${classToken(selection.electionId)}`,
                `seq-results-selector--contest-${classToken(selection.contestId)}`,
                `seq-results-selector--area-${classToken(selection.areaId)}`,
            ].join(" ")}
        >
            <TabRow
                className="seq-results-selector__election-row"
                entityName="election"
                label={labels.elections}
                options={elections}
                selectedKey={areaOptionKey(selectedElection?.id)}
                onSelect={(index) => selectElection(elections[index]?.id ?? null)}
                actions={renderElectionActions?.(selection)}
            />
            <TabRow
                className="seq-results-selector__contest-row"
                entityName="contest"
                label={labels.contests}
                options={contestOptions}
                selectedKey={areaOptionKey(selectedContest?.id)}
                onSelect={(index) => selectContest(contestOptions[index]?.id ?? null)}
                actions={renderContestActions?.(selection)}
            />
            <TabRow
                className="seq-results-selector__area-row"
                entityName="area"
                label={labels.areas}
                options={areaOptions}
                selectedKey={areaOptionKey(selectedArea?.id)}
                onSelect={(index) => setSelectedAreaId(areaOptions[index]?.id ?? null)}
                actions={renderAreaActions?.(selection)}
            />
            <Box
                className={[
                    "seq-results-selector__selected-result",
                    `seq-results-selector__selected-result--election-${classToken(
                        selection.electionId
                    )}`,
                    `seq-results-selector__selected-result--contest-${classToken(
                        selection.contestId
                    )}`,
                    `seq-results-selector__selected-result--area-${classToken(selection.areaId)}`,
                ].join(" ")}
            >
                {renderResult(selection)}
            </Box>
        </Box>
    )
}

export default ResultsSelectorTabs
