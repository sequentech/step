// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//
// Lifted from admin-portal/src/resources/Tally/TallyResultsCandidatesPlurality.tsx.
// Adaptations:
//   P1: replace `useTranslation()` with the `t()` shim from ./strings (see L1).
//   P2: drop `Sequent_Backend_Candidate_Extended` graphql type; use TallyCandidate.
//   P3: replace admin-portal's `<NoItem />` empty-state component with an inline
//       MUI Typography paragraph ("No data available.").

import React from "react"
import {Typography, Box} from "@mui/material"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import {formatPercentOne, isNumber} from "@sequentech/ui-core"

import {TallyCandidate} from "./types"
import {CandidatesResultsCharts} from "./TallyResultsCharts"
import {winningPositionComparator} from "./utils"
import {t} from "./strings"

interface TallyResultsCandidatesProps {
    resultsData: TallyCandidate[]
    orderedResultsData: TallyCandidate[]
    chartName: string
}

export const TallyResultsCandidatesPlurality: React.FC<TallyResultsCandidatesProps> = ({
    resultsData,
    orderedResultsData,
    chartName,
}) => {
    const columns: GridColDef[] = [
        {
            field: "name",
            headerName: t("tally.table.options"),
            flex: 1,
            editable: false,
            align: "left",
        },
        {
            field: "cast_votes",
            headerName: t("tally.table.cast_votes"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<TallyCandidate, string>) =>
                props["value"] ?? "-",
            align: "right",
            headerAlign: "right",
        },
        {
            field: "cast_votes_percent",
            headerName: t("tally.table.cast_votes_percent"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<TallyCandidate, string>) =>
                isNumber(props["value"]) ? formatPercentOne(props["value"]) : "-",
            align: "right",
            headerAlign: "right",
        },
        {
            field: "winning_position",
            headerName: t("tally.table.winning_position"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<TallyCandidate, number>) =>
                props["value"] ?? "-",
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
                            candidates={orderedResultsData}
                            chartName={chartName}
                        />
                    </Box>
                    <Box sx={{flex: "1 1 auto", alignItems: "center", mt: 2, minWidth: 0}}>
                        <DataGrid
                            sx={{mt: 0}}
                            rows={orderedResultsData}
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
                <Typography variant="body2" sx={{mt: 2, ml: 1, color: "text.secondary"}}>
                    No data available.
                </Typography>
            )}
        </Box>
    )
}
