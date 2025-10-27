// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useMemo, useState} from "react"
import {useGetMany, useGetList} from "react-admin"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import Chart, {Props} from "react-apexcharts"
import CardChart from "@/components/dashboard/charts/Charts"
import {Box, Typography} from "@mui/material"

import {Sequent_Backend_Election, Sequent_Backend_Results_Election} from "../../gql/graphql"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import {useTranslation} from "react-i18next"
import {NoItem} from "@/components/NoItem"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {formatPercentOne, isNumber} from "@sequentech/ui-core"
import {useAtomValue} from "jotai"
import {tallyQueryData} from "@/atoms/tally-candidates"

interface TallyElectionsResultsProps {
    tenantId: string | null
    electionEventId: string | null
    resultsEventId: string | null
    electionIds?: string[] | null
}

type Sequent_Backend_Election_Extended = Sequent_Backend_Election & {
    rowId: number
    id: string
    status: string
    elegible_census: number | "-"
    total_voters: number | "-"
    total_voters_percent: number | "-"
}

interface GeneralInformationChartsProps {
    results: Sequent_Backend_Election_Extended[]
    selectedElectionId?: string
    aliasRenderer: (item: any) => any
}

const GeneralInformationCharts: React.FC<GeneralInformationChartsProps> = ({
    results,
    selectedElectionId,
    aliasRenderer,
}) => {
    const {t} = useTranslation()

    // Filter out results with valid participation data
    const validResults = results.filter(
        (result) => isNumber(result.elegible_census) && isNumber(result.total_voters)
    )

    if (validResults.length === 0) {
        return null
    }

    // Find the selected result or use the first one as default
    const selectedResult = selectedElectionId
        ? validResults.find((result) => result.id === selectedElectionId)
        : validResults[0]

    if (!selectedResult) {
        return null
    }

    const result = selectedResult
    const eligibleCensus = result.elegible_census as number
    const totalVoters = result.total_voters as number
    const nonVoters = eligibleCensus - totalVoters

    const chartData = [
        {
            label: t("tally.chart.totalVoters"),
            value: totalVoters,
        },
        {
            label: t("tally.chart.nonVoters"),
            value: nonVoters,
        },
    ].filter((item) => item.value > 0)

    const chartOptions: Props = {
        options: {
            labels: chartData.map((item) => item.label),
            legend: {
                position: "right",
            },
            responsive: [
                {
                    breakpoint: 480,
                    options: {
                        chart: {
                            width: 200,
                        },
                        legend: {
                            position: "bottom",
                        },
                    },
                },
            ],
        },
        series: chartData.map((item) => item.value),
    }

    return (
        <Box
            sx={{
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                mb: 2,
                border: "1px solid #cccccc99",
                maxWidth: {xs: "100%", lg: 450},
            }}
        >
            <CardChart title={aliasRenderer(result)} collapsible={true}>
                <Chart
                    options={chartOptions.options}
                    series={chartOptions.series}
                    type="pie"
                    width="100%"
                    height={300}
                />
            </CardChart>
        </Box>
    )
}

export const TallyElectionsResults: React.FC<TallyElectionsResultsProps> = (props) => {
    const {tenantId, electionEventId, resultsEventId, electionIds} = props
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const [resultsData, setResultsData] = useState<Array<Sequent_Backend_Election_Extended>>([])
    const [selectedElectionId, setSelectedElectionId] = useState<string | null>(null)
    const tallyData = useAtomValue(tallyQueryData)
    const aliasRenderer = useAliasRenderer()

    const elections: Array<Sequent_Backend_Election> | undefined = useMemo(
        () =>
            tallyData?.sequent_backend_election
                ?.filter((election) => electionIds?.includes(election.id))
                ?.map((election): Sequent_Backend_Election => election as any),
        [tallyData?.sequent_backend_election, electionIds]
    )

    const results: Array<Sequent_Backend_Results_Election> | undefined = useMemo(
        () => tallyData?.sequent_backend_results_election,
        [tallyData?.sequent_backend_results_election]
    )

    useEffect(() => {
        if (elections && results) {
            const temp: Array<Sequent_Backend_Election_Extended> | undefined = elections?.map(
                (item, index): Sequent_Backend_Election_Extended => {
                    const result = results?.find((r) => r.election_id === item.id)

                    return {
                        ...item,
                        rowId: index,
                        id: item.id || "",
                        name: item.name,
                        status: item.status || "",
                        elegible_census: result?.elegible_census ?? "-",
                        total_voters: result?.total_voters ?? "-",
                        total_voters_percent: result?.total_voters_percent ?? "-",
                    }
                }
            )

            setResultsData(temp)
            // Set default selected election to the first one if none is selected
            if (!selectedElectionId && temp.length > 0) {
                setSelectedElectionId(temp[0].id)
            }
        }
    }, [results, elections, selectedElectionId])

    const columns: GridColDef[] = [
        {
            field: "name",
            headerName: t("tally.table.elections"),
            flex: 1,
            editable: false,
            valueGetter(params) {
                return aliasRenderer(params.row)
            },
        },
        {
            field: "elegible_census",
            headerName: t("tally.table.elegible_census"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, string>) => props["value"] ?? "-",
        },
        {
            field: "total_voters",
            headerName: t("tally.table.total_votes"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, number>) => props["value"] ?? "-",
        },
        {
            field: "total_voters_percent",
            headerName: t("tally.table.total_votes_percent"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, number>) =>
                isNumber(props["value"]) ? formatPercentOne(props["value"]) : "-",
        },
    ]

    return (
        <>
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
                        <GeneralInformationCharts
                            results={resultsData}
                            selectedElectionId={selectedElectionId || undefined}
                            aliasRenderer={aliasRenderer}
                        />
                    </Box>
                    <Box sx={{flex: "1 1 auto", alignItems: "center", mt: 2, minWidth: 0}}>
                        <DataGrid
                            sx={{
                                "mt": 0,
                                "& .MuiDataGrid-row.selected": {
                                    backgroundColor: "rgba(25, 118, 210, 0.08)",
                                },
                                "& .MuiDataGrid-row.selected:hover": {
                                    backgroundColor: "rgba(25, 118, 210, 0.12)",
                                },
                                "& .MuiDataGrid-cell:focus": {
                                    outline: "none",
                                },
                                "& .MuiDataGrid-cell:focus-within": {
                                    outline: "none",
                                },
                            }}
                            rows={resultsData}
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
                            onRowClick={(params) => {
                                setSelectedElectionId(params.row.id)
                            }}
                            getRowClassName={(params) =>
                                params.row.id === selectedElectionId ? "selected" : ""
                            }
                        />
                    </Box>
                </Box>
            ) : (
                <NoItem />
            )}
        </>
    )
}
