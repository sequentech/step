// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box} from "@mui/material"
import {CandidateResults} from "./CandidateResults"
import {ParticipationSummary} from "./ParticipationSummary"
import {PreferentialCandidateResults} from "./PreferentialCandidateResults"
import type {ResultsAndParticipationProps} from "./types"

export const ResultsAndParticipation: React.FC<ResultsAndParticipationProps> = ({
    chartName,
    summary,
    candidates,
    labels,
    showWeight,
    processResults,
}) => (
    <Box className="seq-tally-results">
        <ParticipationSummary
            result={summary}
            chartName={chartName}
            labels={labels}
            showWeight={showWeight}
        />
        {processResults ? (
            <PreferentialCandidateResults
                processResults={processResults}
                candidates={candidates}
                labels={labels}
            />
        ) : (
            <CandidateResults candidates={candidates} chartName={chartName} labels={labels} />
        )}
    </Box>
)

export default ResultsAndParticipation
