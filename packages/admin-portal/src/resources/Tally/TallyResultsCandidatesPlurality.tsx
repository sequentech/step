// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo} from "react"
import {useTranslation} from "react-i18next"
import {NoItem} from "@/components/NoItem"
import {Typography, Box} from "@mui/material"
import {DataGrid, GridColDef} from "@mui/x-data-grid"
import {Sequent_Backend_Candidate_Extended} from "./types"
import {CandidatesResultsCharts} from "./TallyResultsCharts"
import {formatPercentOne, isNumber} from "@sequentech/ui-core"
import {GridRenderCellParams} from "@mui/x-data-grid"
import {winningPositionComparator} from "./utils"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {useDefaultElectionLang} from "@/hooks/useDefaultElectionLang"

interface TallyResultsCandidatesProps {
    resultsData: Sequent_Backend_Candidate_Extended[]
    orderedResultsData: Sequent_Backend_Candidate_Extended[]
    chartName: string
    electionId: string
    electionEventId: string
}

export const TallyResultsCandidatesPlurality: React.FC<TallyResultsCandidatesProps> = ({
    resultsData,
    orderedResultsData,
    chartName,
    electionId,
    electionEventId,
}) => {
    const {t} = useTranslation()
    const aliasRenderer = useAliasRenderer()
    const defaultElectionLang = useDefaultElectionLang(electionId, electionEventId)
    const normalizedOrderedResultsData = useMemo(
        () =>
            orderedResultsData.map((candidate) => ({
                ...candidate,
                name: aliasRenderer(candidate.presentation, defaultElectionLang),
            })),
        [orderedResultsData, aliasRenderer, defaultElectionLang]
    )

    const columns: GridColDef[] = [
        {
            field: "name",
            headerName: t("tally.table.options"),
            flex: 1,
            editable: false,
            align: "left",
            renderCell: (props: GridRenderCellParams<any, string>) => props.value ?? "-",
        },
        {
            field: "cast_votes",
            headerName: t("tally.table.cast_votes"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, string>) => props["value"] ?? "-",
            align: "right",
            headerAlign: "right",
        },
        {
            field: "cast_votes_percent",
            headerName: t("tally.table.cast_votes_percent"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, string>) =>
                isNumber(props["value"]) ? formatPercentOne(props["value"]) : "-",
            align: "right",
            headerAlign: "right",
        },
        {
            field: "winning_position",
            headerName: t("tally.table.winning_position"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, number>) => props["value"] ?? "-",
            sortComparator: winningPositionComparator,
            align: "right",
            headerAlign: "right",
        },
    ]

    return (
        <Box sx={{borderTop: "1px solid #ccc", mt: 4, p: 0}}>
            <Typography variant="h6" component="div" sx={{mt: 6, ml: 1}}>
                {t("tally.table.candidates")}
            </Typography>

            {resultsData.length ? (
                <Box
                    sx={{
                        display: "flex",
                        flexDirection: {xs: "column", lg: "row"},
                        gap: 4,
                        alignItems: "flex-start",
                    }}
                >
                    <Box sx={{flex: {xs: "1 1 auto", lg: "0 0 auto"}, mt: 2}}>
                        <CandidatesResultsCharts
                            results={normalizedOrderedResultsData}
                            chartName={chartName}
                        />
                    </Box>
                    <Box sx={{flex: "1 1 auto", alignItems: "center", mt: 2, minWidth: 0}}>
                        <DataGrid
                            sx={{mt: 0}}
                            rows={normalizedOrderedResultsData}
                            columns={columns}
                            initialState={{
                                pagination: {
                                    paginationModel: {
                                        pageSize: 10,
                                    },
                                },
                            }}
                            pageSizeOptions={[10, 20, 50, 100]}
                            disableRowSelectionOnClick
                        />
                    </Box>
                </Box>
            ) : (
                <NoItem />
            )}
        </Box>
    )
}
