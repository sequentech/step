// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Typography} from "@mui/material"
import {CandidateResults} from "./CandidateResults"
import {ParticipationSummary} from "./ParticipationSummary"
import {PreferentialCandidateResults} from "./PreferentialCandidateResults"
import type {ResultsAndParticipationProps} from "./types"
import {mergeLabels} from "./utils"

export const ResultsAndParticipation: React.FC<ResultsAndParticipationProps> = ({
    chartName,
    summary,
    candidates,
    labels,
    showWeight,
    processResults,
    preferential = false,
}) => {
    const mergedLabels = mergeLabels(labels)

    return (
        <Box
            className={`seq-tally-results seq-tally-results--${
                preferential ? "preferential" : "candidate"
            }`}
        >
            <ParticipationSummary
                result={summary}
                chartName={chartName}
                labels={labels}
                showWeight={showWeight}
            />
            {preferential ? (
                processResults ? (
                    <PreferentialCandidateResults
                        processResults={processResults}
                        candidates={candidates}
                        labels={labels}
                    />
                ) : (
                    <Typography
                        className="seq-tally-results-preferential-results__empty"
                        color="text.secondary"
                        sx={{mt: 4}}
                    >
                        {mergedLabels.empty}
                    </Typography>
                )
            ) : (
                <CandidateResults candidates={candidates} chartName={chartName} labels={labels} />
            )}
        </Box>
    )
}

export default ResultsAndParticipation
