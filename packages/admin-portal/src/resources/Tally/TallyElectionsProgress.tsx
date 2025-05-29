// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useState} from "react"

import {
    Sequent_Backend_Election,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Execution,
} from "../../gql/graphql"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import {ElectionStatusItem} from "@/components/ElectionStatusItem"
import styled from "@emotion/styled"
import {LinearProgress, Typography, linearProgressClasses} from "@mui/material"
import {useTranslation} from "react-i18next"
import {ITallyCeremonyStatus, ITallyElectionStatus} from "@/types/ceremonies"
import {formatPercentOne} from "@sequentech/ui-core"

type Sequent_Backend_Election_Extended = Sequent_Backend_Election & {
    rowId: number
    id: string
    status: string
    progress: number
}

interface TallyElectionsProgressProps {
    tally?: Sequent_Backend_Tally_Session
    tallySessionExecutions?: Array<Sequent_Backend_Tally_Session_Execution>
    allElections?: Array<Sequent_Backend_Election>
}

export const TallyElectionsProgress: React.FC<TallyElectionsProgressProps> = ({
    tally,
    tallySessionExecutions: execution,
    allElections,
}) => {
    const {t} = useTranslation()

    console.log("bb ALL ELEC", allElections);
    

    const elections = useMemo(() => {
        return (
            allElections?.filter((election) =>
                (tally?.election_ids ?? []).includes(election?.id)
            ) ?? []
        )
    }, [tally?.election_ids, allElections])

    const [electionsData, setElectionsData] = useState<Array<Sequent_Backend_Election_Extended>>([])

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
