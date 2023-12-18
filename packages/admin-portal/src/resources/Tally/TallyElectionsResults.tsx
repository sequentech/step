// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {useGetMany, useGetList} from "react-admin"

import {Sequent_Backend_Election, Sequent_Backend_Results_Election} from "../../gql/graphql"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import {useTranslation} from "react-i18next"
import globalSettings from "@/global-settings"
import { NoItem } from '@/components/NoItem'

interface TallyElectionsResultsProps {
    tenantId: string | null
    electionEventId: string | null
    resultsEventId: string | null
    electionIds: any
}

export const TallyElectionsResults: React.FC<TallyElectionsResultsProps> = (props) => {
    const {tenantId, electionEventId, resultsEventId, electionIds} = props
    const {t} = useTranslation()
    const [resultsData, setResultsData] = useState<
        Array<
            Sequent_Backend_Election & {
                rowId: number
                id: string
                status: string
                elegible_census: number
                total_valid_votes: number
                explicit_invalid_votes: number
                implicit_invalid_votes: number
                blank_votes: number
            }
        >
    >([])

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
        }
    )

    useEffect(() => {
        console.log("results :>> ", results)
        console.log("elections :>> ", elections)
        if (elections) {

            const temp:
                | Array<
                      Sequent_Backend_Election & {
                          rowId: number
                          id: string
                          status: string
                          elegible_census: number
                          total_valid_votes: number
                          explicit_invalid_votes: number
                          implicit_invalid_votes: number
                          blank_votes: number
                      }
                  >
                | undefined = elections?.map((item, index) => {
                console.log("item :>> ", item)

                return {
                    ...item,
                    rowId: index,
                    id: item.id || "",
                    name: item.name,
                    status: item.status || "",
                    elegible_census: item.method,
                    total_valid_votes: item.voters,
                    explicit_invalid_votes: item.number,
                    implicit_invalid_votes: item.turnout,
                    blank_votes: item.turnout,
                }
            })

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
            renderCell: (props: GridRenderCellParams<any, string>) => props["value"] || "-",
        },
        {
            field: "total_valid_votes",
            headerName: t("tally.table.total_valid_votes"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, number>) => props["value"] || 0,
        },
        {
            field: "total_valid_votes",
            headerName: t("tally.table.total_valid_votes"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, number>) => props["value"] || 0,
        },
        {
            field: "explicit_invalid_votes",
            headerName: t("tally.table.explicit_invalid_votes"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, number>) => `${props["value"] || 0}%`,
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
