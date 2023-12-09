// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useContext, useEffect} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    TextInput,
    Identifier,
    RaRecord,
    useRecordContext,
    useDelete,
    WrapperField,
    FunctionField,
    DateField,
    AuthContext,
    useGetList,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {Drawer} from "@mui/material"
import {CreateTally} from "./CreateTally"
import {Sequent_Backend_Election_Event, Sequent_Backend_Tally_Session} from "../../gql/graphql"
import {Dialog} from "@sequentech/ui-essentials"
import {Action, ActionsColumn} from "../../components/ActionButons"
import DescriptionIcon from "@mui/icons-material/Description"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "../../providers/TenantContextProvider"
import {useParams} from "react-router"
import ElectionHeader from "@/components/ElectionHeader"
import {EditTally} from "./EditTally"
import {TrusteeItems} from "@/components/TrusteeItems"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {StatusChip} from "@/components/StatusChip"
import KeyIcon from "@mui/icons-material/Key"
import {styled as MUIStiled} from "@mui/material/styles"
import {theme} from "@sequentech/ui-essentials"
import {IPermissions} from "@/types/keycloak"
import {useActionPermissions} from "../ElectionEvent/EditElectionEventKeys"
import { AuthContextValues } from '@/providers/AuthContextProvider'
import { isTrusteeParticipating } from '@/components/keys-ceremony/TrusteeWizard'

const OMIT_FIELDS = ["id", "ballot_eml"]

const Filters: Array<ReactElement> = [
    <TextInput label="Name" source="name" key={0} />,
    <TextInput label="Description" source="description" key={1} />,
    <TextInput label="ID" source="id" key={2} />,
    <TextInput label="Type" source="type" key={3} />,
    <TextInput source="election_event_id" key={3} />,
]

const TrusteeKeyIcon = MUIStiled(KeyIcon)`
    color: ${theme.palette.brandSuccess};
`

export interface ListAreaProps {
    record: Sequent_Backend_Tally_Session
}

export const ListTally: React.FC<ListAreaProps> = (props) => {
    const {t} = useTranslation()
    const {id} = useParams()

    const record = useRecordContext<Sequent_Backend_Election_Event>()

    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const [_, setTallyId] = useElectionEventTallyStore()
    const [deleteOne] = useDelete()

    const [open, setOpen] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [deleteId, setDeleteId] = React.useState<Identifier | undefined>()
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)
    const [recordId, setRecordId] = React.useState<Identifier | undefined>(undefined)
    const electionEvent = useRecordContext<Sequent_Backend_Election_Event>()

    const {data: keysCeremonies} = useGetList<Sequent_Backend_Tally_Session>(
        "sequent_backend_tally_session",
        {
            sort: {field: "created_at", order: "DESC"},
            filter: {
                tenant_id: tenantId,
                election_event_id: electionEvent.id,
            },
        }
    )

    // Returns a keys ceremony if there's any in which we have been required to
    // participate and is active
    // const getActiveCeremony = (
    //     keyCeremonies: Sequent_Backend_Tally_Session[] | undefined,
    //     authContext: AuthContextValues
    // ) => {
    //     if (!keyCeremonies) {
    //         return
    //     } else {
    //         return keyCeremonies.find((ceremony) => isTrusteeParticipating(ceremony, authContext))
    //     }
    // }

    // let activeCeremony = getActiveCeremony(keysCeremonies, authContext)

    // const {canAdminCeremony, canTrusteeCeremony: canWriteTrustee} = useActionPermissions()

    useEffect(() => {
        if (recordId) {
            setOpen(true)
        }
    }, [recordId])

    const handleCloseCreateDrawer = () => {
        setRecordId(undefined)
        setOpenDrawer(false)
    }

    const viewAdminTally = (id: Identifier) => {
        setTallyId(id as string, false)
    }

    const viewTrusteeTally = (id: Identifier) => {
        setTallyId(id as string, true)
    }

    const actions: Action[] = [
        {icon: <DescriptionIcon />, action: viewAdminTally},
        {
            icon: <TrusteeKeyIcon />,
            action: viewTrusteeTally,
            // showAction: (id: Identifier) => canWriteTrustee && !!getCeremony(id),
        },
    ]

    return (
        <>
            <List
                resource="sequent_backend_tally_session"
                actions={
                    <ListActions
                        withColumns={false}
                        withImport={false}
                        withExport={false}
                        // withFilter={false}
                        open={openDrawer}
                        setOpen={setOpenDrawer}
                        Component={<CreateTally record={record} close={handleCloseCreateDrawer} />}
                    />
                }
                empty={false}
                sx={{flexGrow: 2}}
                filter={{
                    tenant_id: tenantId || undefined,
                    election_event_id: record?.id || undefined,
                }}
                filters={Filters}
            >
                <ElectionHeader title={t("electionEventScreen.tally.title")} subtitle="" />

                <DatagridConfigurable omit={OMIT_FIELDS}>
                    <TextField source="tenant_id" />
                    <DateField source="created_at" />

                    <FunctionField
                        label={t("electionEventScreen.tally.trustees")}
                        render={(record: RaRecord<Identifier>) => <TrusteeItems record={record} />}
                    />

                    <FunctionField
                        label={t("electionEventScreen.tally.electionNumber")}
                        render={(record: RaRecord<Identifier>) => record?.election_ids?.length || 0}
                    />

                    <FunctionField
                        label={t("electionEventScreen.tally.status")}
                        render={(record: RaRecord<Identifier>) => (
                            <StatusChip status={record.execution_status} />
                        )}
                    />

                    <WrapperField source="actions" label="Actions">
                        <ActionsColumn actions={actions} />
                    </WrapperField>
                </DatagridConfigurable>
            </List>
        </>
    )
}
