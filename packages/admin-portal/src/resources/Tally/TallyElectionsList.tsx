// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useState} from "react"
import {Sequent_Backend_Election, Sequent_Backend_Tally_Session} from "../../gql/graphql"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import Checkbox from "@mui/material/Checkbox"
import {useTranslation} from "react-i18next"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {
    IElectionEventPresentation,
    parseEntityPresentation,
    sortByPresentationOrder,
} from "@sequentech/ui-core"

type Sequent_Backend_Election_Extended = Sequent_Backend_Election & {
    rowId: number
    id: string
    active: boolean
    name: string
}
interface TallyElectionsListProps {
    elections: Sequent_Backend_Election[] | undefined
    electionEventId: string
    disabled?: boolean
    update: (elections: Array<string>) => void
    keysCeremonyId: string | null
    tallySession?: Sequent_Backend_Tally_Session
    electionEventPresentation?: unknown
}

export const TallyElectionsList: React.FC<TallyElectionsListProps> = (props) => {
    const {
        disabled,
        elections,
        update,
        keysCeremonyId,
        tallySession: tallyData,
        electionEventPresentation,
    } = props

    const {t, i18n} = useTranslation()
    const aliasRenderer = useAliasRenderer()

    const [electionsData, setElectionsData] = useState<Array<Sequent_Backend_Election_Extended>>([])

    const filteredElections = useMemo(() => {
        if (!keysCeremonyId || tallyData) {
            return elections
        }
        return elections?.filter((election) => election.keys_ceremony_id === keysCeremonyId)
    }, [elections, keysCeremonyId, tallyData])

    const electionsOrder =
        parseEntityPresentation<IElectionEventPresentation>(
            electionEventPresentation
        )?.elections_order

    useEffect(() => {
        if (filteredElections) {
            const mappedElections: Array<Sequent_Backend_Election_Extended> = filteredElections
                .map((election, index) => {
                    const electionName = aliasRenderer(election.presentation)
                    return {
                        ...election,
                        rowId: index,
                        id: election.id || "",
                        name: electionName,
                        active: true,
                    }
                })
                .filter((election) =>
                    tallyData ? (tallyData.election_ids || []).includes(election.id) : true
                )
            const orderedElections = sortByPresentationOrder(mappedElections, electionsOrder, {
                getLabel: (election) => election.name,
                getPresentation: (election) => election.presentation,
            }).map((election, index) => ({...election, rowId: index}))
            setElectionsData(orderedElections)
        }
    }, [aliasRenderer, electionsOrder, filteredElections, tallyData])

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
            field: `presentation.i18n[${i18n.language}].alias`,
            headerName: t("tally.table.elections"),
            flex: 1,
            editable: false,
            valueGetter(value, row) {
                return value ? value : aliasRenderer(row)
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
