// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {useGetList, useGetOne} from "react-admin"

import {
    Sequent_Backend_Candidate,
    Sequent_Backend_Results_Contest,
    Sequent_Backend_Results_Contest_Candidate,
} from "../../gql/graphql"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import {useTranslation} from "react-i18next"
import { NoItem } from '@/components/NoItem'

interface TallyResultsGlobalCandidatesProps {
    contestId: string
    electionId: string
    electionEventId: string
    tenantId: string
}

export const TallyResultsGlobalCandidates: React.FC<TallyResultsGlobalCandidatesProps> = (
    props
) => {
    const {contestId, electionId, electionEventId, tenantId} = props
    const {t} = useTranslation()

    const [resultsData, setResultsData] = useState<
        Array<
            Sequent_Backend_Candidate & {
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

    const {data: election} = useGetOne("sequent_backend_election", {
        id: electionId,
        meta: {tenant_id: tenantId},
    })

    const {data: candidates} = useGetList("sequent_backend_candidate", {
        pagination: {page: 1, perPage: 9999},
        filter: {
            contest_id: contestId,
            tenant_id: tenantId,
            election_event_id: election?.election_event_id,
        },
    })

    const {data: general} = useGetList<Sequent_Backend_Results_Contest>(
        "sequent_backend_results_contest",
        {
            pagination: {page: 1, perPage: 1},
            filter: {
                contest_id: contestId,
                tenant_id: tenantId,
                election_event_id: electionEventId,
                election_id: electionId,
            },
        },
        {
            refetchInterval: 5000,
        }
    )

    const {data: results} = useGetList<Sequent_Backend_Results_Contest_Candidate>(
        "sequent_backend_results_contest_candidate",
        {
            pagination: {page: 1, perPage: 1},
            filter: {
                contest_id: contestId,
                tenant_id: tenantId,
                election_event_id: electionEventId,
                election_id: electionId,
            },
        },
        {
            refetchInterval: 5000,
        }
    )

    useEffect(() => {
        if (results && candidates) {
            const temp:
                | Array<
                      Sequent_Backend_Candidate & {
                          rowId: number
                          id: string
                          status: string
                          method: string
                          voters: number
                          number: number
                          turnout: number
                      }
                  >
                | undefined = candidates?.map((item, index) => {
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

            console.log("TallyResultsGlobalCandidates :: temp", temp);
            

            setResultsData(temp)
        }
    }, [results, candidates])

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
            ) : <NoItem />}
        </>
    )
}
