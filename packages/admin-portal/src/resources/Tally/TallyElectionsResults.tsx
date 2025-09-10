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
}

const GeneralInformationCharts: React.FC<GeneralInformationChartsProps> = ({results}) => {
    const {t} = useTranslation()

    // Filter out results with valid participation data
    const validResults = results.filter(
        (result) => isNumber(result.elegible_census) && isNumber(result.total_voters)
    )

    if (validResults.length === 0) {
        return null
    }

    return (
        <Box sx={{mt: 4, display: "flex", flexDirection: "row", alignItems: "left", gap: 4}}>
            {validResults.map((result) => {
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
                            position: "bottom",
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
                        key={result.id}
                        sx={{display: "flex", flexDirection: "column", alignItems: "center", mb: 2}}
                    >
                        <CardChart title={result.name}>
                            <Chart
                                options={chartOptions.options}
                                series={chartOptions.series}
                                type="pie"
                                width={400}
                                height={300}
                            />
                        </CardChart>
                    </Box>
                )
            })}
        </Box>
    )
}

export const TallyElectionsResults: React.FC<TallyElectionsResultsProps> = (props) => {
    const {tenantId, electionEventId, resultsEventId, electionIds} = props
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const [resultsData, setResultsData] = useState<Array<Sequent_Backend_Election_Extended>>([])
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
        }
    }, [results, elections])

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
                <Box sx={{display: "flex", flexDirection: "row", gap: 4, alignItems: "flex-start"}}>
                    <Box sx={{flex: "0 0 auto"}}>
                        <GeneralInformationCharts results={resultsData} />
                    </Box>
                    <Box sx={{flex: 1}}>
                        <DataGrid
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
                        />
                    </Box>
                </Box>
            ) : (
                <NoItem />
            )}
        </>
    )
}
