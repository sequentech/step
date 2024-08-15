// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"
import {useGetOne, useGetMany, useGetList, useReferenceOneFieldController} from "react-admin"

import {
    Sequent_Backend_Election,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Execution,
} from "../../gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import {ElectionStatusItem} from "@/components/ElectionStatusItem"
import styled from "@emotion/styled"
import {LinearProgress, Typography, linearProgressClasses} from "@mui/material"
import {useTranslation} from "react-i18next"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {ITallyCeremonyStatus, ITallyElectionStatus} from "@/types/ceremonies"
import {formatPercentOne} from "@sequentech/ui-core"

type Sequent_Backend_Election_Extended = Sequent_Backend_Election & {
    rowId: number
    id: string
    status: string
    progress: number
}

export const TallyElectionsProgress: React.FC = () => {
    const {tallyId} = useElectionEventTallyStore()
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)

    const [electionsData, setElectionsData] = useState<Array<Sequent_Backend_Election_Extended>>([])

    const {data: tally} = useGetOne<Sequent_Backend_Tally_Session>(
        "sequent_backend_tally_session",
        {
            id: tallyId,
        },
        {
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const {data: execution} = useGetList<Sequent_Backend_Tally_Session_Execution>(
        "sequent_backend_tally_session_execution",
        {
            filter: {tally_session_id: tallyId},
            sort: {field: "created_at", order: "DESC"},
            pagination: {page: 1, perPage: 1},
        },
        {
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const {data: elections} = useGetMany<Sequent_Backend_Election>("sequent_backend_election", {
        ids: tally?.election_ids || [],
    })

    useEffect(() => {
        if (elections) {
            const temp: Array<Sequent_Backend_Election_Extended> = (elections || []).map(
                (election, index) => ({
                    ...election,
                    rowId: index,
                    id: election.id || "",
                    name: election.name,
                    status: election.status || "",
                    progress: 0,
                })
            )
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
                let status: ITallyCeremonyStatus | undefined = execution?.[0].status
                const election_data = status?.elections_status?.find(
                    (item) => item.election_id === props["id"]
                )
                return (
                    <ElectionStatusItem
                        name={election_data?.status ?? ITallyElectionStatus.WAITING}
                    />
                )
            },
        },
        {
            field: "progress",
            headerName: t("tally.table.progress"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, number>) => {
                let status: ITallyCeremonyStatus | undefined = execution?.[0].status
                const election_data = status?.elections_status?.find(
                    (item) => item.election_id === props["id"]
                )

                return (
                    <ProgressBarDiv>
                        <BorderLinearProgress
                            variant="determinate"
                            value={election_data?.progress ?? 0}
                        />
                        <Typography
                            variant="body2"
                            color="text.secondary"
                            sx={{marginLeft: "1rem", display: "flex", justifyContent: "end"}}
                        >
                            {formatPercentOne((election_data?.progress ?? 0) / 100)}
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
