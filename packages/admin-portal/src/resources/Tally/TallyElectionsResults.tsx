// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {useGetOne, useGetMany} from "react-admin"

import {Sequent_Backend_Election, Sequent_Backend_Tally_Session} from "../../gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import Checkbox from "@mui/material/Checkbox"
import {ElectionStatusItem} from "@/components/ElectionStatusItem"
import styled from "@emotion/styled"
import {Box, LinearProgress, Typography, linearProgressClasses} from "@mui/material"
import {useTranslation} from "react-i18next"

// interface TallyElectionsResultsProps {
//     update: (elections: Array<string>) => void
// }

export const TallyElectionsResults: React.FC = () => {
    const [tallyId] = useElectionEventTallyStore()
    const {t} = useTranslation()

    const [electionsData, setElectionsData] = useState<
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

    const {data} = useGetOne<Sequent_Backend_Tally_Session>(
        "sequent_backend_tally_session",
        {
            id: tallyId,
        },
        {
            refetchInterval: 5000,
        }
    )

    const {data: elections} = useGetMany("sequent_backend_election", {
        ids: data?.election_ids || [],
    })

    useEffect(() => {
        if (elections) {
            const temp = (elections || []).map((election, index) => ({
                ...election,
                rowId: index,
                id: election.id || "",
                name: election.name,
                status: election.status || "",
                method: election.method,
                voters: election.voters,
                number: election.number,
                turnout: election.turnout,
            }))
            setElectionsData(temp)
        }
    }, [elections])

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
        <DataGrid
            rows={electionsData}
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
    )
}
