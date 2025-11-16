// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"
import {useGetList} from "react-admin"

import {
    Sequent_Backend_Trustee,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Execution,
    Sequent_Backend_Keys_Ceremony,
} from "../../gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import CachedIcon from "@mui/icons-material/Cached"
import CheckCircleIcon from "@mui/icons-material/CheckCircle"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ITallyCeremonyStatus, ITallyExecutionStatus, ITallyTrusteeStatus} from "@/types/ceremonies"
import {NoItem} from "@/components/NoItem"
import {useTranslation} from "react-i18next"
import {Box, Icon, Typography} from "@mui/material"
import ElectionHeader from "@/components/ElectionHeader"
import KeyIcon from "@mui/icons-material/Key"
import {SettingsContext} from "@/providers/SettingsContextProvider"

interface TallyTrusteesListProps {
    tally?: Sequent_Backend_Tally_Session
    tallySessionExecutions?: Array<Sequent_Backend_Tally_Session_Execution>
    update: (selectedTrustees: boolean) => void
}

export const TallyTrusteesList: React.FC<TallyTrusteesListProps> = (props) => {
    const {tally, update, tallySessionExecutions} = props
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)

    const {tallyId} = useElectionEventTallyStore()
    const [tenantId] = useTenantStore()
    const [eventTrustees, setEventTrustees] = useState<Array<string>>([])

    const [trusteesData, setTrusteesData] = useState<
        Array<Sequent_Backend_Trustee & {rowId: number; id: string; active: boolean}>
    >([])
    const [keysImported, setKeysImported] = useState<number>(0)

    const {data: keyCeremony} = useGetList<Sequent_Backend_Keys_Ceremony>(
        "sequent_backend_keys_ceremony",
        {
            pagination: {page: 1, perPage: 9999},
            filter: {election_event_id: tally?.election_event_id, tenant_id: tenantId},
        },
        {
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const {data: trustees} = useGetList<Sequent_Backend_Trustee>("sequent_backend_trustee", {
        pagination: {page: 1, perPage: 1000},
        filter: {
            tenant_id: tenantId,
            id: eventTrustees,
        },
    })

    useEffect(() => {
        let newTrustees = keyCeremony?.[0]?.trustee_ids ?? []
        if (eventTrustees !== newTrustees) {
            setEventTrustees(newTrustees)
        }
    }, [keyCeremony])

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
            active:
                status.trustees.find((x) => x.name === trustee.name)?.status !==
                ITallyTrusteeStatus.WAITING,
        }))
        setTrusteesData(temp)
    }, [trustees, tallySessionExecutions])

    useEffect(() => {
        if (trusteesData) {
            const importadas = trusteesData.filter((election) => election.active).length
            setKeysImported(importadas)

            update(importadas === tally?.threshold ? true : false)
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
            width: 100,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, boolean>) =>
                props.value ? <CheckCircleIcon sx={{color: "#0F054C"}} /> : <CachedIcon />,
        },
    ]

    return (
        <>
            <Box
                sx={{
                    width: "100%",
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "center",
                }}
            >
                <ElectionHeader
                    title={"tally.trusteeTallyTitle"}
                    subtitle={"tally.trusteeTallySubTitle"}
                />

                <div
                    style={{
                        display: "flex",
                        justifyContent: "flex-end",
                        alignItems: "start",
                    }}
                >
                    <div
                        style={{
                            display: "flex",
                            flexDirection: "column",
                            justifyContent: "center",
                            alignItems: "end",
                            marginRight: "16px",
                        }}
                    >
                        <Typography variant="body2" sx={{margin: 0}}>
                            {keysImported}/{trusteesData.length}
                            {t("tally.common.imported")}
                        </Typography>
                        <Typography variant="body2" sx={{margin: 0}}>
                            {tally?.threshold ?? "-"}
                            {t("tally.common.needed")}
                        </Typography>
                    </div>

                    <Icon
                        sx={{
                            height: "100%",
                            color:
                                tally?.execution_status === ITallyExecutionStatus.CONNECTED ||
                                tally?.execution_status === ITallyExecutionStatus.IN_PROGRESS ||
                                tally?.execution_status === ITallyExecutionStatus.SUCCESS
                                    ? "#43E3A1"
                                    : "#d32f2f",
                        }}
                    >
                        <KeyIcon />
                    </Icon>
                </div>
            </Box>

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
