// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useState} from "react"
import {useGetOne, useGetList} from "react-admin"
import {Sequent_Backend_Election, Sequent_Backend_Tally_Session} from "../../gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import Checkbox from "@mui/material/Checkbox"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"

type Sequent_Backend_Election_Extended = Sequent_Backend_Election & {
    rowId: number
    id: string
    active: boolean
}
interface TallyElectionsListProps {
    elections: Sequent_Backend_Election[] | undefined
    electionEventId: string
    disabled?: boolean
    update: (elections: Array<string>) => void
    keysCeremonyId: string | null
    tallySession?: Sequent_Backend_Tally_Session
}

export const TallyElectionsList: React.FC<TallyElectionsListProps> = (props) => {
    const {disabled, elections, update, keysCeremonyId, tallySession: tallyData} = props

    const {t} = useTranslation()
    const aliasRenderer = useAliasRenderer()

    const [electionsData, setElectionsData] = useState<Array<Sequent_Backend_Election_Extended>>([])

    const filteredElections = useMemo(() => {
        if (!keysCeremonyId || tallyData) {
            return elections
        }
        return elections?.filter((election) => election.keys_ceremony_id === keysCeremonyId)
    }, [elections, keysCeremonyId, tallyData])

    useEffect(() => {
        if (filteredElections) {
            const temp: Array<Sequent_Backend_Election_Extended> = (filteredElections || [])
                .sort((a, b) => {
                    if (a.alias && b.alias) {
                        return a.alias.localeCompare(b.alias)
                    } else {
                        return a.name.localeCompare(b.name)
                    }
                })
                .map((election, index) => ({
                    ...election,
                    rowId: index,
                    id: election.id || "",
                    name: election.name,
                    active: true,
                }))
                .filter((election) =>
                    tallyData ? (tallyData.election_ids || []).includes(election.id) : true
                )
            setElectionsData(temp)
        }
    }, [filteredElections])

    useEffect(() => {
        if (electionsData) {
            const temp: Array<string> = electionsData
                .filter((election) => election.active)
                .map((election) => election.id)
            update(temp)
        }
    }, [electionsData])

    const columns: GridColDef[] = [
        {
            field: "name",
            headerName: t("tally.table.elections"),
            flex: 1,
            editable: false,
            valueGetter(params) {
                return aliasRenderer(params.row)
            },
        },
        {
            field: "active",
            headerName: t("tally.table.selected"),
            editable: false,
            width: 100,
            renderCell: (props: GridRenderCellParams<any, boolean>) => (
                <Checkbox
                    checked={props.value}
                    disabled={disabled}
                    onChange={() => handleConfirmChange(props.row)}
                />
            ),
        },
    ]

    function handleConfirmChange(clickedRow: any) {
        const updatedData: Array<Sequent_Backend_Election_Extended> = electionsData?.map((x) => {
            if (x.rowId === clickedRow.rowId) {
                return {
                    ...x,
                    active: !clickedRow.active,
                }
            }
            return x
        })
        setElectionsData(updatedData)
    }

    return (
        <DataGrid
            rows={electionsData}
            sx={{width: "100%"}}
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
