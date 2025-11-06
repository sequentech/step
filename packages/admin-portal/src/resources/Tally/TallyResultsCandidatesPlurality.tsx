// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useTranslation} from "react-i18next"
import {NoItem} from "@/components/NoItem"
import {Typography, Box} from "@mui/material"
import {DataGrid, GridColDef} from "@mui/x-data-grid"
import {Sequent_Backend_Candidate_Extended} from "./types"
import {CandidatesResultsCharts} from "./TallyResultsCharts"

interface TallyResultsCandidatesProps {
    resultsData: Sequent_Backend_Candidate_Extended[]
    orderedResultsData: Sequent_Backend_Candidate_Extended[]
    columns: GridColDef[]
    chartName: string
}

export const TallyResultsCandidatesPlurality: React.FC<TallyResultsCandidatesProps> = ({
    resultsData,
    orderedResultsData,
    columns,
    chartName,
}) => {
    const {t} = useTranslation()

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
                            results={orderedResultsData}
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
                <NoItem />
            )}
        </Box>
    )
}
