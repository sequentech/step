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
import {Button} from "react-admin"
import {Alert, Box, Tooltip, Typography} from "@mui/material"
import {
    ListKeysCeremonyQuery,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Execution,
    TrusteeNamesQuery,
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
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import styled from "@emotion/styled"
import {EAllowTally} from "@sequentech/ui-core"
import {
    ETallyType,
    IExecutionStatus,
    ITallyCeremonyStatus,
    ITallyExecutionStatus,
} from "@/types/ceremonies"
import {useMutation, useQuery} from "@apollo/client"
import {UPDATE_TALLY_CEREMONY} from "@/queries/UpdateTallyCeremony"
import {IPermissions} from "@/types/keycloak"
import {ResetFilters} from "@/components/ResetFilters"
import {LIST_KEYS_CEREMONY} from "@/queries/ListKeysCeremonies"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {IKeysCeremonyExecutionStatus} from "@/services/KeyCeremony"
import {Add} from "@mui/icons-material"
import {useKeysPermissions} from "../ElectionEvent/useKeysPermissions"
import {GET_TRUSTEES_NAMES} from "@/queries/GetTrusteesNames"
import {StyledChip} from "@/components/StyledChip"

const OMIT_FIELDS = ["ballot_eml", "trustees"]

const Filters: Array<ReactElement> = [
    <TextInput label="Name" source="name" key={0} />,
    <TextInput label="Tally Type" source="tally_type" key={1} />,
    <TextInput label="Description" source="description" key={2} />,
    <TextInput label="ID" source="id" key={3} />,
    <TextInput label="Type" source="type" key={4} />,
    <TextInput source="election_event_id" key={5} />,
]

const NotificationLink = styled.span`
    text-decoration: underline;
    cursor: pointer;
    padding: 2px;

    &:hover {
        text-decoration: none;
    }
`

const StyledNull = styled.div`
    display: block;
    padding-left: 18px;
`

const TrusteeKeyIcon = MUIStiled(KeyIcon)`
    color: ${theme.palette.brandSuccess};
`

export interface ListAreaProps {}

export const ListTally: React.FC<ListAreaProps> = () => {
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const {
        canAdminCeremony,
        canTrusteeCeremony,
        canExportCeremony,
        canCreateCeremony,
        showTallyColumns,
    } = useKeysPermissions()
    const notify = useNotify()

    const electionEventRecord = useRecordContext<Sequent_Backend_Election_Event>()
    const refresh = useRefresh()

    const [tenantId] = useTenantStore()
    const {globalSettings} = useContext(SettingsContext)

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
    const [isCreatingTally, setIsCreatingTally] = React.useState<boolean>(false)

    const isPublished = electionEventRecord?.status && electionEventRecord.status.is_published

    const [UpdateTallyCeremonyMutation] =
        useMutation<UpdateTallyCeremonyMutation>(UPDATE_TALLY_CEREMONY)

    const {data: keysCeremonies, error: errorCeremonies} = useQuery<ListKeysCeremonyQuery>(
        LIST_KEYS_CEREMONY,
        {
            variables: {
                tenantId: tenantId,
                electionEventId: electionEventRecord?.id,
            },
            pollInterval: globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
            context: {
                headers: {
                    "x-hasura-role": isTrustee
                        ? IPermissions.TRUSTEE_CEREMONY
                        : IPermissions.ADMIN_CEREMONY,
                },
            },
        }
    )

    const {data: tallySessions} = useGetList<Sequent_Backend_Tally_Session>(
        "sequent_backend_tally_session",
        {
            sort: {field: "created_at", order: "DESC"},
            filter: {
                tenant_id: tenantId,
                election_event_id: electionEventRecord?.id,
            },
        },
        {
            refetchOnWindowFocus: true,
            refetchOnReconnect: true,
            refetchOnMount: true,
            meta: {
                context: {
                    headers: {
                        "x-hasura-role": isTrustee
                            ? IPermissions.TRUSTEE_CEREMONY
                            : IPermissions.ADMIN_CEREMONY,
                    },
                },
            },
        }
    )

    const {data: tallySessionExecutions} = useGetList<Sequent_Backend_Tally_Session_Execution>(
        "sequent_backend_tally_session_execution",
        {
            pagination: {page: 1, perPage: 1},
            sort: {field: "created_at", order: "DESC"},
            filter: {
                tally_session_id: tallySessions?.[0]?.id,
                tenant_id: tenantId,
            },
        },
        {
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const {data: trusteeNames} = useQuery<TrusteeNamesQuery>(GET_TRUSTEES_NAMES, {
        variables: {
            tenantId: tenantId,
        },
    })
    const isKeyCeremonyFinished = useMemo(
        () =>
            !!keysCeremonies?.list_keys_ceremony?.items?.find(
                (keysCeremony) =>
                    (keysCeremony.execution_status as IKeysCeremonyExecutionStatus | undefined) ===
                    IKeysCeremonyExecutionStatus.SUCCESS
            ),
        [keysCeremonies?.list_keys_ceremony?.items]
    )

    const keysCeremonyIds = useMemo(
        () => keysCeremonies?.list_keys_ceremony?.items?.map((ceremony) => ceremony?.id) ?? [],
        [keysCeremonies?.list_keys_ceremony?.items]
    )

    const CreateTallyButton = () => (
        <Button
            label={String(t("electionEventScreen.tally.create.createTallyButton"))}
            onClick={() => {
                setIsCreatingTally(true)
                setCreatingFlag(ETallyType.ELECTORAL_RESULTS)
            }}
            disabled={!isKeyCeremonyFinished || !isPublished || isCreatingTally}
            style={{height: "10px"}}
            sx={{marginBottom: "10px"}}
        >
            <IconButton icon={faPlus as any} fontSize="24px" />
        </Button>
    )

    const CreateInitializationReportButton: React.FC<{isListActions: boolean}> = ({
        isListActions,
    }) => (
        <Button
            label={String(t("electionEventScreen.tally.create.createInitializationReportButton"))}
            onClick={() => setCreatingFlag(ETallyType.INITIALIZATION_REPORT)}
            disabled={!isKeyCeremonyFinished || !isPublished}
        >
            {isListActions ? <Add /> : <IconButton icon={faPlus as any} fontSize="24px" />}
        </Button>
    )

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            {canCreateCeremony && !isKeyCeremonyFinished && (
                <Alert severity="warning">
                    {t("electionEventScreen.tally.notify.noKeysTally")}
                </Alert>
            )}
            {canCreateCeremony && isKeyCeremonyFinished && !isPublished && (
                <Alert severity="warning">
                    {t("electionEventScreen.tally.notify.noPublication")}
                </Alert>
            )}
            <Typography variant="h4" paragraph>
                {t("electionEventScreen.tally.emptyHeader")}
            </Typography>
            {canCreateCeremony ? (
                <>
                    <Typography variant="body1" paragraph>
                        {t("common.resources.noResult.askCreate")}
                    </Typography>
                    <CreateTallyButton />
                    <CreateInitializationReportButton isListActions={false} />
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
                <Tooltip title={String(t("tallysheet.common.tallyCeremony.manage"))}>
                    <CellTowerIcon />
                </Tooltip>
            ) : (
                <Tooltip title={String(t("tallysheet.common.tallyCeremony.view"))}>
                    <DescriptionIcon />
                </Tooltip>
            ),
            action: viewAdminTally,
            showAction: (id: Identifier) => canAdminCeremony || canDoMiruAction,
        },
        {
            icon: (
                <Tooltip title={String(t("tallysheet.common.tallyCeremony.cancel"))}>
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
                    <Tooltip title={String(t("tallysheet.common.tallyCeremony.addKey"))}>
                        <TrusteeKeyIcon />
                    </Tooltip>
                ) : (
                    <Tooltip title={String(t("tallysheet.common.tallyCeremony.view"))}>
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
                setCreatingFlag(null)
                refresh()
            }
        } catch (error) {
            console.log("TallyCeremony :: confirmCeremonyAction :: error", error)
            notify(t("tally.cancelTallyCeremonyError"), {type: "error"})
        }
    }

    const isTrusteeParticipating = (
        tally_session: Sequent_Backend_Tally_Session,
        ceremony: Sequent_Backend_Tally_Session_Execution | undefined,
        authContext: AuthContextValues
    ) => {
        if (ceremony) {
            let ret =
                tally_session.execution_status === ITallyExecutionStatus.STARTED &&
                !!ceremony.status.trustees.find(
                    (trustee: any) => trustee.name === authContext.trustee
                )
            return ret
        }
        return false
    }

    // Returns a keys ceremony if there's any in which we have been required to
    // participate and is active
    const getActiveCeremony = (
        tallySessions: Sequent_Backend_Tally_Session[] | undefined,
        authContext: AuthContextValues
    ) => {
        if (!tallySessions) {
            return
        } else {
            return tallySessions.find((tallySession) =>
                isTrusteeParticipating(tallySession, tallySessionExecutions?.[0], authContext)
            )
        }
    }
    let activeCeremony = getActiveCeremony(tallySessions, authContext)

    if (errorCeremonies) {
        return (
            <ResourceListStyles.EmptyBox>
                <Typography variant="h4" paragraph>
                    {errorCeremonies.graphQLErrors[0].message}
                </Typography>
            </ResourceListStyles.EmptyBox>
        )
    }

    return (
        <>
            {canTrusteeCeremony &&
            activeCeremony &&
            tallySessions?.[0]?.execution_status === "STARTED" ? (
                <Alert severity="info">
                    <Trans i18nKey="electionEventScreen.tally.notify.participateNow">
                        {t("tally.invited")}
                        <NotificationLink
                            onClick={(e: any) => {
                                e.preventDefault()
                                viewTrusteeTally(tallySessions?.[0]?.id)
                            }}
                        >
                            click on the tally Key Action
                        </NotificationLink>
                        to participate.
                    </Trans>
                </Alert>
            ) : null}

            {
                <List
                    resource="sequent_backend_tally_session"
                    actions={
                        <ListActions
                            withColumns={showTallyColumns}
                            withImport={false}
                            withExport={false}
                            withFilter={false}
                            withAction={canCreateCeremony}
                            doAction={() => {
                                setIsCreatingTally(true)
                                setCreatingFlag(ETallyType.ELECTORAL_RESULTS)
                            }}
                            actionLabel="electionEventScreen.tally.create.createTallyButton"
                            extraActions={
                                canAdminCeremony
                                    ? [
                                          <CreateInitializationReportButton
                                              key={"initialization"}
                                              isListActions={true}
                                          />,
                                      ]
                                    : []
                            }
                        />
                    }
                    empty={<Empty />}
                    sx={{flexGrow: 2}}
                    filter={{
                        tenant_id: tenantId || undefined,
                        election_event_id: electionEventRecord?.id || undefined,
                        keys_ceremony_id: {
                            format: "hasura-raw-query",
                            value: {_in: keysCeremonyIds},
                        },
                    }}
                    storeKey={false}
                    filters={Filters}
                >
                    <ResetFilters />
                    <ElectionHeader title={"electionEventScreen.tally.title"} subtitle="" />
                    <DatagridConfigurable omit={OMIT_FIELDS} bulkActionButtons={false}>
                        <TextField source="id" />
                        <FunctionField
                            label={String(t("electionEventScreen.tally.tallyType.label"))}
                            render={(record: RaRecord<Identifier>) =>
                                t(`electionEventScreen.tally.tallyType.${record.tally_type}`)
                            }
                        />
                        <DateField source="created_at" showTime={true} />

                        <FunctionField
                            key="permission_label"
                            label={String(t("electionEventScreen.tally.permissionLabels"))}
                            render={(record: RaRecord<Identifier>) => {
                                return (
                                    <>
                                        {record?.permission_label &&
                                        record?.permission_label.length > 0 ? (
                                            record?.permission_label.map(
                                                (item: any, index: number) => (
                                                    <StyledChip key={index} label={item} />
                                                )
                                            )
                                        ) : (
                                            <StyledNull>-</StyledNull>
                                        )}
                                    </>
                                )
                            }}
                        />

                        <FunctionField
                            source="trustees"
                            label={String(t("electionEventScreen.tally.trustees"))}
                            render={(record: RaRecord<Identifier>) => (
                                <Box sx={{height: 36, overflowY: "scroll"}}>
                                    <TrusteeItems
                                        record={record}
                                        trusteeNames={trusteeNames?.sequent_backend_trustee}
                                    />
                                </Box>
                            )}
                        />

                        <FunctionField
                            label={String(t("electionEventScreen.tally.electionNumber"))}
                            render={(record: RaRecord<Identifier>) =>
                                record?.election_ids?.length || 0
                            }
                        />

                        <FunctionField
                            label={String(t("electionEventScreen.tally.status"))}
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
            }

            <Dialog
                variant="warning"
                open={openCancelTally}
                ok={String(t("tally.common.dialog.okCancel"))}
                cancel={String(t("tally.common.dialog.cancel"))}
                title={String(t("tally.common.dialog.cancelTitle"))}
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
