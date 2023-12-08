// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {useGetMany, useGetList} from "react-admin"

import {
    Sequent_Backend_Election,
    Sequent_Backend_Results_Election,
} from "../../gql/graphql"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import {useTranslation} from "react-i18next"

interface TallyElectionsResultsProps {
    tenantId: string | null
    electionEventId: string | null
    electionIds: Array<string>
}

export const TallyElectionsResults: React.FC<TallyElectionsResultsProps> = (props) => {
    const {tenantId, electionEventId, electionIds} = props
    const {t} = useTranslation()
    const [resultsData, setResultsData] = useState<
        Array<
            Sequent_Backend_Election & {
                rowId: number
                id: string
                status: string
                method: string
                voters: number
                number: number
                turnout: number
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
            filter: {tenant_id: tenantId, election_event_id: electionEventId},
        },
        {
            refetchInterval: 5000,
        }
    )

    useEffect(() => {
        if (results && elections) {
            const temp:
                | Array<
                      Sequent_Backend_Election & {
                          rowId: number
                          id: string
                          status: string
                          method: string
                          voters: number
                          number: number
                          turnout: number
                      }
                  >
                | undefined = elections?.map((item, index) => {
                return {
                    ...item,
                    rowId: index,
                    id: item.id || "",
                    name: item.name,
                    status: item.status || "",
                    method: item.method,
                    voters: item.voters,
                    number: item.number,
                    turnout: item.turnout,
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
            field: "method",
            headerName: t("tally.table.method"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, string>) => props["value"] || "-",
        },
        {
            field: "elegible",
            headerName: t("tally.table.elegible"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, number>) => props["value"] || 0,
        },
        {
            field: "number",
            headerName: t("tally.table.number"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, number>) => props["value"] || 0,
        },
        {
            field: "turnout",
            headerName: t("tally.table.turnout"),
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
            ) : null}
        </>
    )
}
