// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//
// Workbench-flavoured composition of the lifted tally components.
//
// This file is NOT a 1-1 lift from admin-portal; it is a thin assembly layer
// that admin-portal does not need (admin-portal composes these via its own
// TallyResultsSectionGlobal / TallyResultsSectionArea, which carry too much
// admin-only context to lift). The substitution rationale lives in
// packages/workbench/LIFTING-TALLY.md adaptation V1.

import React from "react"
import {Box, Typography} from "@mui/material"

import {TallyResultsViewModel} from "./types"
import {ParticipationSummaryChart} from "./TallyResultsCharts"
import {TallyResultsCandidatesPlurality} from "./TallyResultsCandidatesPlurality"
import {TallyResultsCandidatesIRV} from "./TallyResultsCandidatesIRV"

interface TallyResultsViewProps {
    model: TallyResultsViewModel
}

export const TallyResultsView: React.FC<TallyResultsViewProps> = ({model}) => {
    const chartName = model.contestName ?? model.summary.id
    const participation = `${chartName} — participation`
    const candidatesName = `${chartName} — candidates`

    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 2}}>
            <Box sx={{display: "flex", flexDirection: {xs: "column", md: "row"}, gap: 2}}>
                <ParticipationSummaryChart result={model.summary} chartName={participation} />
                <Box sx={{flex: 1}}>
                    <Typography variant="body2" sx={{mb: 1}}>
                        Census: <strong>{model.summary.elegible_census}</strong> · valid:{" "}
                        <strong>{model.summary.total_valid_votes}</strong> · blank:{" "}
                        <strong>{model.summary.blank_votes}</strong> · invalid:{" "}
                        <strong>{model.summary.total_invalid_votes}</strong>
                    </Typography>
                    <Typography variant="body2" color="text.secondary">
                        Counting algorithm: {model.countingAlgorithm ?? "unknown"} · winners:{" "}
                        {model.winnersCount}
                    </Typography>
                </Box>
            </Box>
            <TallyResultsCandidatesPlurality
                resultsData={model.candidates}
                orderedResultsData={model.candidates}
                chartName={candidatesName}
            />
            {model.runoff ? <TallyResultsCandidatesIRV processResults={model.runoff} /> : null}
        </Box>
    )
}
