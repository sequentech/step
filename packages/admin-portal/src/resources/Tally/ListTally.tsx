// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useContext, useMemo} from "react"

import {styled as MUIStiled} from "@mui/material/styles"
import {
    DatagridConfigurable,
    List,
    TextField,
    TextInput,
    Identifier,
    RaRecord,
    useRecordContext,
    FunctionField,
    DateField,
    useGetList,
    useNotify,
    useRefresh,
} from "react-admin"
import CellTowerIcon from "@mui/icons-material/CellTower"
import {ListActions} from "../../components/ListActions"
import {Alert, Button, Drawer, Tooltip, Typography} from "@mui/material"
import {
    Sequent_Backend_Election_Event,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Execution,
    UpdateTallyCeremonyMutation,
} from "../../gql/graphql"
import {ActionsColumn} from "../../components/ActionButons"
import DescriptionIcon from "@mui/icons-material/Description"
import {Trans, useTranslation} from "react-i18next"
import {useTenantStore} from "../../providers/TenantContextProvider"
import ElectionHeader from "@/components/ElectionHeader"
import {TrusteeItems} from "@/components/TrusteeItems"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {StatusChip} from "@/components/StatusChip"
import KeyIcon from "@mui/icons-material/Key"
import DoNotDisturbOnIcon from "@mui/icons-material/DoNotDisturbOn"
import {theme, IconButton, Dialog} from "@sequentech/ui-essentials"
import {AuthContext, AuthContextValues} from "@/providers/AuthContextProvider"
import {useActionPermissions} from "../ElectionEvent/EditElectionEventKeys"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import styled from "@emotion/styled"
import {IExecutionStatus, ITallyCeremonyStatus, ITallyExecutionStatus} from "@/types/ceremonies"
import {useMutation} from "@apollo/client"
import {UPDATE_TALLY_CEREMONY} from "@/queries/UpdateTallyCeremony"
import {IPermissions} from "@/types/keycloak"

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
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const {canAdminCeremony, canTrusteeCeremony} = useActionPermissions()
    const notify = useNotify()

    const electionEventRecord = useRecordContext<Sequent_Backend_Election_Event>()
    const refresh = useRefresh()

    const [tenantId] = useTenantStore()
    const {setTallyId, setCreatingFlag} = useElectionEventTallyStore()
    const isTrustee = authContext.isAuthorized(true, tenantId, IPermissions.TRUSTEE_CEREMONY)
    const canDoMiruAction = authContext.isAuthorized(true, tenantId, [
        IPermissions.MIRU_SIGN,
        IPermissions.MIRU_CREATE,
        IPermissions.MIRU_DOWNLOAD,
        IPermissions.MIRU_SEND,
    ])

    const [openCancelTally, openCancelTallySet] = React.useState(false)
    const [deleteId, setDeleteId] = React.useState<Identifier | undefined>()

    const isKeyCeremonyFinished =
        electionEventRecord?.status && electionEventRecord.status.keys_ceremony_finished
    const isPublished = electionEventRecord?.status && electionEventRecord.status.is_published

    const [UpdateTallyCeremonyMutation] =
        useMutation<UpdateTallyCeremonyMutation>(UPDATE_TALLY_CEREMONY)

    const {data: keysCeremonies} = useGetList<Sequent_Backend_Tally_Session>(
        "sequent_backend_tally_session",
        {
            sort: {field: "created_at", order: "DESC"},
            filter: {
                tenant_id: tenantId,
                election_event_id: electionEventRecord?.id,
            },
        },
        {
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const {data: tallySessionExecutions} = useGetList<Sequent_Backend_Tally_Session_Execution>(
        "sequent_backend_tally_session_execution",
        {
            pagination: {page: 1, perPage: 1},
            sort: {field: "created_at", order: "DESC"},
            filter: {
                tally_session_id: keysCeremonies?.[0]?.id,
                tenant_id: tenantId,
            },
        },
        {
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const CreateButton = () => (
        <Button
            onClick={() => setCreatingFlag(true)}
            disabled={!isKeyCeremonyFinished || !isPublished}
        >
            <IconButton icon={faPlus} fontSize="24px" />
            {t("electionEventScreen.tally.create.createButton")}
        </Button>
    )

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            {canAdminCeremony && !isKeyCeremonyFinished && (
                <Alert severity="warning">
                    {t("electionEventScreen.tally.notify.noKeysTally")}
                </Alert>
            )}
            {canAdminCeremony && isKeyCeremonyFinished && !isPublished && (
                <Alert severity="warning">
                    {t("electionEventScreen.tally.notify.noPublication")}
                </Alert>
            )}
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

    const viewAdminTally = (id: Identifier) => {
        setTallyId(id as string, false)
    }

    const viewTrusteeTally = (id: Identifier) => {
        setTallyId(id as string, true)
    }

    const cancelAdminTally = (id: Identifier) => {
        setDeleteId(id)
        openCancelTallySet(true)
    }

    const actions = (record: RaRecord) => [
        {
            icon: isTrustee ? (
                <Tooltip title={t("tallysheet.common.tallyCeremony.manage")}>
                    <CellTowerIcon />
                </Tooltip>
            ) : (
                <Tooltip title={t("tallysheet.common.tallyCeremony.view")}>
                    <DescriptionIcon />
                </Tooltip>
            ),
            action: viewAdminTally,
            showAction: (id: Identifier) => canAdminCeremony || canDoMiruAction,
        },
        {
            icon: (
                <Tooltip title={t("tallysheet.common.tallyCeremony.cancel")}>
                    <DoNotDisturbOnIcon />
                </Tooltip>
            ),
            action: cancelAdminTally,
            showAction: (id: Identifier) =>
                canAdminCeremony &&
                (record.execution_status === ITallyExecutionStatus.NOT_STARTED ||
                    record.execution_status === ITallyExecutionStatus.STARTED ||
                    record.execution_status === ITallyExecutionStatus.CONNECTED),
        },
        {
            icon:
                record.execution_status === ITallyExecutionStatus.NOT_STARTED ||
                record.execution_status === ITallyExecutionStatus.STARTED ||
                record.execution_status === ITallyExecutionStatus.CONNECTED ? (
                    <Tooltip title={t("tallysheet.common.tallyCeremony.addKey")}>
                        <TrusteeKeyIcon />
                    </Tooltip>
                ) : (
                    <Tooltip title={t("tallysheet.common.tallyCeremony.view")}>
                        <DescriptionIcon />
                    </Tooltip>
                ),
            action: viewTrusteeTally,
            showAction: (id: Identifier) => canTrusteeCeremony,
        },
    ]

    const confirmCancelAction = async () => {
        try {
            const {data: nextStatus, errors} = await UpdateTallyCeremonyMutation({
                variables: {
                    election_event_id: electionEventRecord?.id,
                    tally_session_id: deleteId,
                    status: ITallyExecutionStatus.CANCELLED,
                },
            })

            if (errors) {
                notify(t("tally.cancelTallyCeremonyError"), {type: "error"})
            }

            if (nextStatus) {
                notify(t("tally.cancelTallyCeremonySuccess"), {type: "success"})
                setCreatingFlag(false)
                refresh()
            }
        } catch (error) {
            console.log("TallyCeremony :: confirmCeremonyAction :: error", error)
            notify(t("tally.cancelTallyCeremonyError"), {type: "error"})
        }
    }

    const isTrusteeParticipating = (
        ceremony: Sequent_Backend_Tally_Session_Execution | undefined,
        authContext: AuthContextValues
    ) => {
        if (ceremony) {
            const status: ITallyCeremonyStatus = ceremony.status
            return (
                (ceremony.status === IExecutionStatus.NOT_STARTED ||
                    ceremony.status === IExecutionStatus.IN_PROCESS) &&
                !!status.trustees.find((trustee) => trustee.name === authContext.trustee)
            )
        }
        return false
    }

    // Returns a keys ceremony if there's any in which we have been required to
    // participate and is active
    const getActiveCeremony = (
        keysCeremonies: Sequent_Backend_Tally_Session[] | undefined,
        authContext: AuthContextValues
    ) => {
        if (!keysCeremonies) {
            return
        } else {
            return keysCeremonies.find((ceremony) =>
                isTrusteeParticipating(tallySessionExecutions?.[0], authContext)
            )
        }
    }
    let activeCeremony = getActiveCeremony(keysCeremonies, authContext)

    return (
        <>
            {canTrusteeCeremony && keysCeremonies?.[0]?.execution_status === "STARTED" ? (
                <Alert severity="info">
                    <Trans i18nKey="electionEventScreen.tally.notify.participateNow">
                        {t("tally.invited")}
                        <NotificationLink
                            onClick={(e: any) => {
                                e.preventDefault()
                                viewTrusteeTally(keysCeremonies?.[0]?.id)
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
                        withColumns={canAdminCeremony}
                        withImport={false}
                        withExport={false}
                        withFilter={false}
                        withAction={canAdminCeremony}
                        doAction={() => setCreatingFlag(true)}
                        actionLabel="electionEventScreen.tally.create.createButton"
                    />
                }
                empty={<Empty />}
                sx={{flexGrow: 2}}
                filter={{
                    tenant_id: tenantId || undefined,
                    election_event_id: electionEventRecord?.id || undefined,
                }}
                storeKey={false}
                filters={Filters}
            >
                <ElectionHeader title={"electionEventScreen.tally.title"} subtitle="" />

                <DatagridConfigurable omit={OMIT_FIELDS} bulkActionButtons={false}>
                    <TextField source="tenant_id" />
                    <DateField source="created_at" showTime={true} />

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

                    <FunctionField
                        source="actions"
                        label="Actions"
                        render={(record: RaRecord<Identifier>) => (
                            <ActionsColumn actions={actions(record)} />
                        )}
                    >
                        {/* <ActionsColumn actions={actions} /> */}
                    </FunctionField>
                </DatagridConfigurable>
            </List>

            <Dialog
                variant="warning"
                open={openCancelTally}
                ok={t("tally.common.dialog.okCancel")}
                cancel={t("tally.common.dialog.cancel")}
                title={t("tally.common.dialog.cancelTitle")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmCancelAction()
                    }
                    openCancelTallySet(false)
                }}
            >
                {t("tally.common.dialog.cancelMessage")}
            </Dialog>
        </>
    )
}
