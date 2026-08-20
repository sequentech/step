// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useMemo} from "react"
import {Box, Chip, Stack, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {ResultsManifest, ResultsSqliteDataset} from "@/types/results"
import {manifestTitle} from "@/services/resultLabels"
import {buildAreaElectionSummaries} from "@/services/areaSummaries"
import {ResultsSummary} from "./ResultsSummary"
import {ResultsSelectorTabs} from "./ResultsSelectorTabs"
import {entityClassName} from "@/services/cssClassNames"
import {orderElectionResultRows} from "@/services/resultsOrdering"

interface ResultsPageContentProps {
    manifest: ResultsManifest
    dataset: ResultsSqliteDataset
    initialElectionId?: string
    onElectionChange?: (electionId?: string) => void
}

export const ResultsPageContent: React.FC<ResultsPageContentProps> = ({
    manifest,
    dataset,
    initialElectionId,
    onElectionChange,
}) => {
    const {i18n, t} = useTranslation()
    const locale = i18n.resolvedLanguage ?? i18n.language ?? manifest.default_locale ?? "en"
    const title = manifestTitle(manifest.title, locale, t("resultsPortal.pageTitle"))
    const areaElectionSummaries = useMemo(() => buildAreaElectionSummaries(dataset), [dataset])
    const summaryResults = useMemo(
        () =>
            orderElectionResultRows(
                manifest.visibility_scope === "area_based"
                    ? areaElectionSummaries
                    : dataset.results_election,
                manifest,
                dataset,
                locale
            ),
        [areaElectionSummaries, dataset, locale, manifest]
    )
    const pageClassName = [
        "seq-results-page",
        entityClassName("event", manifest.election_event_id),
        entityClassName("publication", manifest.publication_id),
        `seq-results-route-scope--${manifest.route_scope}`,
        `seq-results-access--${manifest.access}`,
        `seq-results-visibility--${manifest.visibility_scope}`,
        manifest.route_election_id
            ? entityClassName("route-election", manifest.route_election_id)
            : null,
    ]
        .filter(Boolean)
        .join(" ")

    return (
        <Box
            className={pageClassName}
            sx={{width: "100%", maxWidth: 1180, mx: "auto", px: {xs: 2, sm: 3}, py: {xs: 3, md: 5}}}
        >
            <Stack
                className="seq-results-page__header"
                direction={{xs: "column", md: "row"}}
                spacing={2}
                justifyContent="space-between"
                alignItems={{xs: "flex-start", md: "center"}}
            >
                <Box className="seq-results-page__intro">
                    <Typography
                        className="seq-results-page__title"
                        component="h1"
                        variant="h3"
                        sx={{fontSize: {xs: 32, md: 44}}}
                    >
                        {title}
                    </Typography>
                    <Typography
                        className="seq-results-page__description"
                        color="text.secondary"
                        sx={{mt: 1}}
                    >
                        {t("resultsPortal.publishedResultsDescription")}
                    </Typography>
                </Box>
                <Stack
                    className="seq-results-page__meta"
                    direction="row"
                    spacing={1}
                    flexWrap="wrap"
                >
                    <Chip
                        className="seq-results-page__version-chip"
                        label={t("resultsPortal.version", {version: manifest.version})}
                        variant="outlined"
                    />
                    <Chip
                        className="seq-results-page__access-chip"
                        label={
                            manifest.access === "public"
                                ? t("resultsPortal.publicAccess")
                                : t("resultsPortal.signedInAccess")
                        }
                        color={manifest.access === "public" ? "success" : "primary"}
                        variant="outlined"
                    />
                </Stack>
            </Stack>

            {summaryResults.length > 0 && (
                <ResultsSummary
                    elections={dataset.election}
                    resultsElections={summaryResults}
                    locale={locale}
                />
            )}

            <Box className="seq-results-page__contests" sx={{mt: {xs: 3, md: 5}}}>
                <Typography
                    className="seq-results-page__contests-title"
                    component="h2"
                    variant="h5"
                    sx={{mb: 2}}
                >
                    {t("resultsPortal.resultsAndParticipationTitle")}
                </Typography>
                <ResultsSelectorTabs
                    manifest={manifest}
                    dataset={dataset}
                    locale={locale}
                    initialElectionId={initialElectionId}
                    onElectionChange={onElectionChange}
                />
            </Box>
        </Box>
    )
}
