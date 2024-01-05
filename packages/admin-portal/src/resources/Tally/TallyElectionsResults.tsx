// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"
import {useGetMany, useGetList} from "react-admin"

import {Sequent_Backend_Election, Sequent_Backend_Results_Election} from "../../gql/graphql"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import {useTranslation} from "react-i18next"
import {NoItem} from "@/components/NoItem"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {formatPercentOne, isNumber} from "@sequentech/ui-essentials"

interface TallyElectionsResultsProps {
    tenantId: string | null
    electionEventId: string | null
    resultsEventId: string | null
    electionIds: any
}

type Sequent_Backend_Election_Extended = Sequent_Backend_Election & {
    rowId: number
    id: string
    status: string
    elegible_census: number | "-"
    total_valid_votes: number | "-"
    total_valid_votes_percent: number | "-"
}

export const TallyElectionsResults: React.FC<TallyElectionsResultsProps> = (props) => {
    const {tenantId, electionEventId, resultsEventId, electionIds} = props
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const [resultsData, setResultsData] = useState<Array<Sequent_Backend_Election_Extended>>([])

    const {data: elections} = useGetMany("sequent_backend_election", {
        ids: electionIds || [],
    })

    const {data: results} = useGetList<Sequent_Backend_Results_Election>(
        "sequent_backend_results_election",
        {
            pagination: {page: 1, perPage: 1},
            filter: {
                tenant_id: tenantId,
                election_event_id: electionEventId,
                results_event_id: resultsEventId,
            },
        },
        {
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    useEffect(() => {
        console.log("results :>> ", results)
        console.log("elections :>> ", elections)
        if (elections && results) {
            const temp: Array<Sequent_Backend_Election_Extended> | undefined = elections?.map(
                (item, index) => {
                    const result = results?.find((r) => r.election_id === item.id)
                    console.log("result :>> ", result)

                    return {
                        ...item,
                        rowId: index,
                        id: item.id || "",
                        name: item.name,
                        status: item.status || "",
                        elegible_census: result?.elegible_census ?? "-",
                        total_valid_votes: result?.total_valid_votes ?? "-",
                        total_valid_votes_percent: result?.total_valid_votes_percent ?? "-",
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
        },
        {
            field: "elegible_census",
            headerName: t("tally.table.elegible_census"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, string>) => props["value"] ?? "-",
        },
        {
            field: "total_valid_votes",
            headerName: t("tally.table.total_valid_votes"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, number>) => props["value"] ?? "-",
        },
        {
            field: "total_valid_votes_percent",
            headerName: t("tally.table.total_valid_votes_percent"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, number>) =>
                isNumber(props["value"]) ? formatPercentOne(props["value"]) : "-",
        },
    ]

    return (
        <>
            {resultsData.length ? (
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
            ) : (
                <NoItem />
            )}
        </>
    )
}
