// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {useGetOne, useGetMany, useGetList} from "react-admin"

import {Sequent_Backend_Election, Sequent_Backend_Tally_Session, Sequent_Backend_Tally_Session_Execution} from "../../gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import {ElectionStatusItem} from "@/components/ElectionStatusItem"
import styled from "@emotion/styled"
import {LinearProgress, Typography, linearProgressClasses} from "@mui/material"
import { useTranslation } from 'react-i18next'


export const TallyElectionsProgress: React.FC = () => {
    const [tallyId] = useElectionEventTallyStore()
    const {t} = useTranslation()

    const [electionsData, setElectionsData] = useState<
        Array<
            Sequent_Backend_Election & {rowId: number; id: string; status: string; progress: number}
        >
    >([])

    const {data: tally} = useGetOne<Sequent_Backend_Tally_Session>("sequent_backend_tally_session", {
        id: tallyId,
    })

    const {data: executions} = useGetList<Sequent_Backend_Tally_Session_Execution>(
        "sequent_backend_tally_session_execution",
        {
            filter: {tally_session_id: tallyId},
            sort: {field: "created_at", order: "DESC"},
            pagination: {page: 1, perPage: 1},
        },
        {
            refetchInterval: 5000,
        }
    )

    const {data: elections} = useGetMany("sequent_backend_election", {
        ids: tally?.election_ids || [],
    })

    useEffect(() => {
        if (elections) {
            const temp = (elections || []).map((election, index) => ({
                ...election,
                rowId: index,
                id: election.id || "",
                name: election.name,
                status: election.status || "",
                progress: election.progress,
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
            field: "status",
            headerName: t("tally.table.status"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, string>) => {
                return (
                    <ElectionStatusItem name={props["value"] !== "" ? props["value"] : "Pending"} />
                )
            },
        },
        {
            field: "progress",
            headerName: t("tally.table.progress"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, number>) => {
                let rand: number = Math.floor(Math.random() * (100 + 1) + 0)
                return (
                    <ProgressBarDiv>
                        <BorderLinearProgress variant="determinate" value={rand} />
                        <Typography
                            variant="body2"
                            color="text.secondary"
                            sx={{marginLeft: "1rem", display: "flex", justifyContent: "end"}}
                        >
                            {rand}%
                        </Typography>
                    </ProgressBarDiv>
                )
            },
        },
    ]

    const ProgressBarDiv = styled.div`
        width: 100%;
        max-width: 18rem;
        display: flex;
        flex-direction: row;
        align-items: center;
    `

    const BorderLinearProgress = styled(LinearProgress)(({theme}) => ({
        height: 4,
        width: "100%",
        [`&.${linearProgressClasses.colorPrimary}`]: {
            backgroundColor: theme.palette.grey[theme.palette.mode === "light" ? 200 : 800],
        },
        [`& .${linearProgressClasses.bar}`]: {
            backgroundColor: theme.palette.brandColor,
        },
    }))

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

