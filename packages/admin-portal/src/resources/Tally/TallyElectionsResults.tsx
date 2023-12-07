// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {useGetOne, useGetMany, useGetList} from "react-admin"

import {
    Sequent_Backend_Election,
    Sequent_Backend_Results_Election,
    Sequent_Backend_Tally_Session,
} from "../../gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import Checkbox from "@mui/material/Checkbox"
import {ElectionStatusItem} from "@/components/ElectionStatusItem"
import styled from "@emotion/styled"
import {Box, LinearProgress, Typography, linearProgressClasses} from "@mui/material"
import {useTranslation} from "react-i18next"
import {TenantContextProvider} from "@/providers/TenantContextProvider"

interface TallyElectionsResultsProps {
    tenantId: string | null
    electionEventId: string | null
    electionIds: Array<string>
}

export const TallyElectionsResults: React.FC<TallyElectionsResultsProps> = (props) => {
    const {tenantId, electionEventId, electionIds} = props
    const {t} = useTranslation()
    const [resultsData, setResultsData] = useState<
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
    >([])

    const {data: results, isLoading} = useGetList<Sequent_Backend_Results_Election>(
        "sequent_backend_results_election",
        {
            pagination: {page: 1, perPage: 1},
            filter: {tenant_id: tenantId, election_event_id: electionEventId},
        },
        // {
        //     refetchInterval: 5000,
        // }
    )

    const {data: elections} = useGetMany("sequent_backend_election", {
        ids: electionIds || [],
    })

    useEffect(() => {
        console.log("TallyElectionsResults :: results", results)
        console.log("TallyElectionsResults :: elections", elections)

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
                | undefined = elections?.map((election, index) => {
                return {
                    ...election,
                    rowId: index,
                    id: election.id || "",
                    name: election.name,
                    status: election.status || "",
                    method: election.method,
                    voters: election.voters,
                    number: election.number,
                    turnout: election.turnout,
                }
            })

            setResultsData(temp)
        }
    }, [results, elections])

    // useEffect(() => {
    //     if (elections) {
    //         const temp = (elections || []).map((election, index) => ({
    //             ...election,
    //             rowId: index,
    //             id: election.id || "",
    //             name: election.name,
    //             status: election.status || "",
    //             method: election.method,
    //             voters: election.voters,
    //             number: election.number,
    //             turnout: election.turnout,
    //         }))
    //         setElectionsData(temp)
    //     }
    // }, [elections])

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
        </>
    )
}
