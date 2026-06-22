// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Chip, Stack, Typography} from "@mui/material"
import {ResultsManifest, ResultsSqliteDataset} from "@/types/results"
import {manifestTitle} from "@/services/resultLabels"
import {ResultsSummary} from "./ResultsSummary"
import {ContestResultsBlock} from "./ContestResultsBlock"

interface ResultsPageContentProps {
    manifest: ResultsManifest
    dataset: ResultsSqliteDataset
}

export const ResultsPageContent: React.FC<ResultsPageContentProps> = ({manifest, dataset}) => {
    const locale = manifest.default_locale ?? "en"
    const title = manifestTitle(manifest.title, locale, "Election Results")

    return (
        <Box sx={{width: "100%", maxWidth: 1180, mx: "auto", px: {xs: 2, sm: 3}, py: {xs: 3, md: 5}}}>
            <Stack
                direction={{xs: "column", md: "row"}}
                spacing={2}
                justifyContent="space-between"
                alignItems={{xs: "flex-start", md: "center"}}
            >
                <Box>
                    <Typography component="h1" variant="h3" sx={{fontSize: {xs: 32, md: 44}}}>
                        {title}
                    </Typography>
                    <Typography color="text.secondary" sx={{mt: 1}}>
                        Published results for this election event.
                    </Typography>
                </Box>
                <Stack direction="row" spacing={1} flexWrap="wrap">
                    <Chip label={`Version ${manifest.version}`} variant="outlined" />
                    <Chip
                        label={manifest.access === "public" ? "Public access" : "Signed-in access"}
                        color={manifest.access === "public" ? "success" : "primary"}
                        variant="outlined"
                    />
                </Stack>
            </Stack>

            {dataset.results_election.length > 0 && (
                <ResultsSummary
                    elections={dataset.election}
                    resultsElections={dataset.results_election}
                    locale={locale}
                />
            )}

            <Box sx={{mt: {xs: 3, md: 5}}}>
                <Typography component="h2" variant="h5" sx={{mb: 2}}>
                    Contests
                </Typography>
                {manifest.contests.map((contest) => (
                    <ContestResultsBlock
                        key={`${contest.election_id}-${contest.contest_id}-${contest.area_id ?? "global"}`}
                        manifestContest={contest}
                        dataset={dataset}
                        locale={locale}
                    />
                ))}
            </Box>
        </Box>
    )
}
