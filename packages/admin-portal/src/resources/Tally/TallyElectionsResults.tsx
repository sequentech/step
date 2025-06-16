// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useMemo, useState} from "react"
import {useGetMany, useGetList} from "react-admin"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"

import {Sequent_Backend_Election, Sequent_Backend_Results_Election} from "../../gql/graphql"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import {useTranslation} from "react-i18next"
import {NoItem} from "@/components/NoItem"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {formatPercentOne, isNumber} from "@sequentech/ui-core"
import {useAtomValue} from "jotai"
import {tallyQueryData} from "@/atoms/tally-candidates"
import {useSQLQuery} from "@/hooks/useSQLiteDatabase"

interface TallyElectionsResultsProps {
    tenantId: string | null
    electionEventId: string | null
    resultsEventId: string | null
    electionIds?: string[] | null
    databaseBuffer: Uint8Array | null
}

type Sequent_Backend_Election_Extended = Sequent_Backend_Election & {
    rowId: number
    id: string
    status: string
    elegible_census: number | "-"
    total_voters: number | "-"
    total_voters_percent: number | "-"
}

export const TallyElectionsResults: React.FC<TallyElectionsResultsProps> = (props) => {
    const {tenantId, electionEventId, resultsEventId, electionIds, databaseBuffer} = props
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const [resultsData, setResultsData] = useState<Array<Sequent_Backend_Election_Extended>>([])
    const tallyData = useAtomValue(tallyQueryData)
    const aliasRenderer = useAliasRenderer()

    const ids = electionIds || []
    const {data: elections} = useSQLQuery(
        `SELECT * FROM election WHERE id IN (${ids.map(() => "?").join(",")})`,
        ids,
        {
            databaseBuffer: databaseBuffer,
            enabled: !!databaseBuffer,
        }
    )

    const {data: results} = useSQLQuery("SELECT * FROM results_election", [], {
        databaseBuffer: databaseBuffer,
        enabled: !!databaseBuffer,
    })

    useEffect(() => {
        if (elections && results) {
            const temp: Array<Sequent_Backend_Election_Extended> | undefined = elections?.map(
                (item, index): Sequent_Backend_Election_Extended => {
                    const result = results?.find((r) => r.election_id === item.id)

                    return {
                        ...(item as Sequent_Backend_Election),
                        rowId: index,
                        id: item.id || "",
                        name: item.name,
                        status: item.status || "",
                        elegible_census: result?.elegible_census ?? "-",
                        total_voters: result?.total_voters ?? "-",
                        total_voters_percent: result?.total_voters_percent ?? "-",
                    }
                }
            )

            setResultsData(temp)
        }
    }, [results, elections])

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
            field: "elegible_census",
            headerName: t("tally.table.elegible_census"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, string>) => props["value"] ?? "-",
        },
        {
            field: "total_voters",
            headerName: t("tally.table.total_votes"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, number>) => props["value"] ?? "-",
        },
        {
            field: "total_voters_percent",
            headerName: t("tally.table.total_votes_percent"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, number>) =>
                isNumber(props["value"]) ? formatPercentOne(props["value"]) : "-",
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
            ) : (
                <NoItem />
            )}
        </>
    )
}
