// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {useGetOne, useGetMany} from "react-admin"

import {
    Sequent_Backend_Trustee,
    Sequent_Backend_Tally_Session,
} from "../../gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {
    DataGrid,
    GridColDef,
    GridRenderCellParams,
} from "@mui/x-data-grid"
import Checkbox from "@mui/material/Checkbox"

interface TallyTrusteesListProps {
    update: (elections: Array<string>) => void
}

export const TallyTrusteesList: React.FC<TallyTrusteesListProps> = (props) => {
    const {update} = props

    const [tallyId] = useElectionEventTallyStore()

    const [trusteesData, setTrusteesData] = useState<
        Array<Sequent_Backend_Trustee & {rowId: number; id: string; active: boolean}>
    >([])


    const {data} = useGetOne<Sequent_Backend_Tally_Session>(
        "sequent_backend_tally_session",
        {
            id: tallyId,
        }
    )

    const {data: trustees} = useGetMany("sequent_backend_trustee", {
        ids: data?.trustee_ids || [],
    })

    useEffect(() => {
        if (trustees) {
            const temp = (trustees || []).map((trustee, index) => ({
                ...trustee,
                rowId: index,
                id: trustee.id || "",
                name: trustee.name,
                active: false,
            }))
            setTrusteesData(temp)
        }
    }, [trustees])

    useEffect(() => {
        if (trusteesData) {
            const temp = trusteesData.filter((election) => election.active).map((election) => election.id)
            update(temp)
        }
    }, [trusteesData])

    const columns: GridColDef[] = [
        {
            field: "name",
            headerName: "Trustees",
            flex: 1,
            editable: false,
        },
        {
            field: "active",
            headerName: "Fragment",
            flex: 1,
            editable: true,
            renderCell: (props: GridRenderCellParams<any, boolean>) => (
                <Checkbox checked={props.value} onChange={() => handleConfirmChange(props.row)} />
            ),
        },
    ]

    function handleConfirmChange(clickedRow: any) {
        const updatedData: Array<
            Sequent_Backend_Trustee & {rowId: number; id: string; active: boolean}
        > = trusteesData?.map((x) => {
            if (x.rowId === clickedRow.rowId) {
                return {
                    ...x,
                    active: !clickedRow.active,
                }
            }
            return x
        })
        setTrusteesData(updatedData)
    }

    return (
        <DataGrid
            rows={trusteesData}
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
