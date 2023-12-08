// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {useGetOne, useGetList} from "react-admin"

import {
    Sequent_Backend_Trustee,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Execution,
} from "../../gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import CachedIcon from "@mui/icons-material/Cached"
import CheckCircleIcon from "@mui/icons-material/CheckCircle"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ITallyCeremonyStatus} from "@/types/ceremonies"
import {NoItem} from "@/components/NoItem"
import {useTranslation} from "react-i18next"

interface TallyTrusteesListProps {
    update: (elections: Array<string>) => void
}

export const TallyTrusteesList: React.FC<TallyTrusteesListProps> = (props) => {
    const {update} = props
    const {t} = useTranslation()

    const [tallyId] = useElectionEventTallyStore()
    const [tenantId] = useTenantStore()

    const [trusteesData, setTrusteesData] = useState<
        Array<Sequent_Backend_Trustee & {rowId: number; id: string; active: boolean}>
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

    const {data: tallySessionExecutions} = useGetList<Sequent_Backend_Tally_Session_Execution>(
        "sequent_backend_tally_session",
        {
            pagination: {page: 1, perPage: 1},
            sort: {field: "created_at", order: "DESC"},
            filter: {
                tally_session_id: tallyId,
                tenant_id: tenantId,
            },
        },
        {
            refetchInterval: 5000,
        }
    )

    const {data: trustees} = useGetList("sequent_backend_trustee", {
        pagination: {page: 1, perPage: 1000},
        filter: {tenant_id: tenantId},
    })

    useEffect(() => {
        if (!tallySessionExecutions?.[0].status || !trustees) {
            return
        }
        let status: ITallyCeremonyStatus = tallySessionExecutions[0].status

        const temp = (trustees || []).map((trustee, index) => ({
            ...trustee,
            rowId: index,
            id: trustee.id,
            name: trustee.name,
            active: status.trustees.find((x) => x.name === trustee.name),
        }))
        setTrusteesData(temp)
    }, [trustees, tallySessionExecutions])

    useEffect(() => {
        if (trusteesData) {
            const temp = trusteesData
                .filter((election) => election.active)
                .map((election) => election.id)
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
            renderCell: (props: GridRenderCellParams<any, boolean>) =>
                props.value ? <CheckCircleIcon sx={{color: "#0F054C"}} /> : <CachedIcon />,
        },
    ]

    return (
        <>
            {trusteesData.length ? (
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
            ) : (
                <NoItem item={t("tally.common.noTrustees")} />
            )}
        </>
    )
}

