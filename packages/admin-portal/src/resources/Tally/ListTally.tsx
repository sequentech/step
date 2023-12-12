// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useContext, useEffect} from "react"
import {styled as MUIStiled} from "@mui/material/styles"
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
    useGetList,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {Alert, Box, Button, Drawer, Typography} from "@mui/material"
import {CreateTally} from "./CreateTally"
import {
    Sequent_Backend_Election_Event,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Execution,
} from "../../gql/graphql"
import {Action, ActionsColumn} from "../../components/ActionButons"
import DescriptionIcon from "@mui/icons-material/Description"
import {Trans, useTranslation} from "react-i18next"
import {useTenantStore} from "../../providers/TenantContextProvider"
import {useNavigate, useParams} from "react-router"
import ElectionHeader from "@/components/ElectionHeader"
import {TrusteeItems} from "@/components/TrusteeItems"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {StatusChip} from "@/components/StatusChip"
import KeyIcon from "@mui/icons-material/Key"
import {theme, IconButton} from "@sequentech/ui-essentials"
import {AuthContext, AuthContextValues} from "@/providers/AuthContextProvider"
import {useActionPermissions} from "../ElectionEvent/EditElectionEventKeys"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import styled from "@emotion/styled"
import {Maybe} from "graphql/jsutils/Maybe"
import {IExecutionStatus, ITallyCeremonyStatus} from "@/types/ceremonies"

const OMIT_FIELDS = ["id", "ballot_eml"]

const Filters: Array<ReactElement> = [
    <TextInput label="Name" source="name" key={0} />,
    <TextInput label="Description" source="description" key={1} />,
    <TextInput label="ID" source="id" key={2} />,
    <TextInput label="Type" source="type" key={3} />,
    <TextInput source="election_event_id" key={3} />,
]

const NotificationLink = styled.span`
    text-decoration: underline;
    cursor: pointer;
    padding: 2px;

    &:hover {
        text-decoration: none;
    }
`

const TrusteeKeyIcon = MUIStiled(KeyIcon)`
    color: ${theme.palette.brandSuccess};
`

export interface ListAreaProps {
    recordTally: Sequent_Backend_Tally_Session
}

export const ListTally: React.FC<ListAreaProps> = (props) => {
    const {recordTally} = props
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const {canAdminCeremony, canTrusteeCeremony} = useActionPermissions()
    
    const record = useRecordContext<Sequent_Backend_Election_Event>()

    const [tenantId] = useTenantStore()
    const {tallyId, setTallyId, setCreatingFlag, isTrustee} = useElectionEventTallyStore()
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

    // const {data: tallySessionExecutions} = useGetList<Sequent_Backend_Tally_Session_Execution>(
    //     "sequent_backend_tally_session_execution",
    //     {
    //         pagination: {page: 1, perPage: 1},
    //         sort: {field: "created_at", order: "DESC"},
    //         filter: {
    //             tally_session_id: keysCeremonies?.id,
    //             tenant_id: tenantId,
    //         },
    //     },
    //     {
    //         refetchInterval: 5000,
    //     }
    // )

    const isTrusteeParticipating = (
        ceremony: Sequent_Backend_Tally_Session_Execution,
        authContext: AuthContextValues
    ) => {
        const status: ITallyCeremonyStatus = ceremony.status
        return (
            (ceremony.status === IExecutionStatus.NOT_STARTED ||
                ceremony.status === IExecutionStatus.IN_PROCESS) &&
            !!status.trustees.find((trustee) => trustee.name === authContext.username)
        )
    }

    const CreateButton = () => (
        <Button
            onClick={() => setCreatingFlag(true)}
            disabled={!keysCeremonies || keysCeremonies?.length > 0}
        >
            <IconButton icon={faPlus} fontSize="24px" />
            {t("electionEventScreen.tally.create.createButton")}
        </Button>
    )

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t("electionEventScreen.tally.emptyHeader")}
            </Typography>
            {canAdminCeremony ? (
                <>
                    <Typography variant="body1" paragraph>
                        {t("common.resources.noResult.askCreate")}
                    </Typography>
                    <CreateButton />
                </>
            ) : null}
        </ResourceListStyles.EmptyBox>
    )

    const handleCloseCreateDrawer = () => {
        setRecordId(undefined)
        setOpen(false)
        setOpenDrawer(false)
    }

    const viewAdminTally = (id: Identifier) => {
        setTallyId(id as string, false)
    }

    const viewTrusteeTally = (id: Identifier) => {
        setTallyId(id as string, true)
    }

    const actions: Action[] = [
        {
            icon: <DescriptionIcon />,
            action: viewAdminTally,
            showAction: (id: Identifier) => canAdminCeremony,
        },
        {
            icon: <TrusteeKeyIcon />,
            action: viewTrusteeTally,
            showAction: (id: Identifier) => canTrusteeCeremony,
        },
    ]

    // Returns a keys ceremony if there's any in which we have been required to
    // participate and is active
    // const getActiveCeremony = (
    //     tallyCeremonies: Sequent_Backend_Tally_Session[] | undefined,
    //     authContext: AuthContextValues
    // ) => {
    //     if (!tallyCeremonies) {
    //         return
    //     } else {
    //         return tallyCeremonies.find((ceremony) =>
    //             isTrusteeParticipating(tallySessionExecutions?.[0], authContext)
    //         )
    //     }
    // }
    // let activeCeremony = getActiveCeremony(keysCeremonies, authContext)

    return (
        <>
            {isTrustee ? (
                <Alert severity="info">
                    <Trans i18nKey="electionEventScreen.keys.notify.participateNow">
                        You have been invited to participate in a Keys ceremony. Please
                        <NotificationLink
                            onClick={(e: any) => {
                                e.preventDefault()
                                viewTrusteeTally(keysCeremonies?.[0].id)
                            }}
                        >
                            click on the tally Key Action
                        </NotificationLink>
                        to participate.
                    </Trans>
                </Alert>
            ) : null}

            <List
                resource="sequent_backend_tally_session"
                actions={
                    <ListActions
                        withColumns={false}
                        withImport={false}
                        withExport={false}
                        withAction={true}
                        doAction={() => setCreatingFlag(true)}
                        actionLabel="common.label.add"
                    />
                }
                empty={<Empty />}
                sx={{flexGrow: 2}}
                filter={{
                    tenant_id: tenantId || undefined,
                    election_event_id: record?.id || undefined,
                }}
                filters={Filters}
            >
                <ElectionHeader title={"electionEventScreen.tally.title"} subtitle="" />

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

            <Drawer
                anchor="right"
                open={open}
                onClose={handleCloseCreateDrawer}
                PaperProps={{
                    sx: {width: "40%"},
                }}
            >
                <CreateTally record={record} close={handleCloseCreateDrawer} />
            </Drawer>
        </>
    )
}
