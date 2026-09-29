// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, createContext, useContext, useMemo} from "react"
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
    useListContext,
    useNotify,
    useRefresh,
} from "react-admin"
import CellTowerIcon from "@mui/icons-material/CellTower"
import {ListActions} from "../../components/ListActions"
import {Button} from "react-admin"
import {Alert, Box, Tooltip, Typography} from "@mui/material"
import {
    ListKeysCeremonyQuery,
    ListTallyKeyRestoreStateQuery,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Execution,
    TrusteeNamesQuery,
    UpdateTallyCeremonyMutation,
} from "../../gql/graphql"
import {Action, ActionsColumn} from "../../components/ActionButons"
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
import {AuthContext} from "@/providers/AuthContextProvider"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import styled from "@emotion/styled"
import {EAllowTally} from "@sequentech/ui-core"
import {
    ETallyKeyRestoreEligibility,
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
import {LIST_TALLY_KEY_RESTORE_STATE} from "@/queries/ListTallyKeyRestoreState"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {IKeysCeremonyExecutionStatus} from "@/services/KeyCeremony"
import {Add} from "@mui/icons-material"
import {useKeysPermissions} from "../ElectionEvent/useKeysPermissions"
import {GET_TRUSTEES_NAMES} from "@/queries/GetTrusteesNames"
import {StyledChip} from "@/components/StyledChip"
import {getTallyTrusteeStatus} from "@/services/tallyCeremonyParticipation"
import {getTallyKeyRestoreEligibility} from "./utils"

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

const getLatestExecutionByTallySessionId = <T extends {tally_session_id: string}>(
    executions: ReadonlyArray<T> | undefined
): Map<string, T> => {
    const latestExecutions = new Map<string, T>()
    for (const execution of executions ?? []) {
        if (!latestExecutions.has(execution.tally_session_id)) {
            latestExecutions.set(execution.tally_session_id, execution)
        }
    }
    return latestExecutions
}

const LatestTallySessionExecutionsContext = createContext<
    Map<string, Sequent_Backend_Tally_Session_Execution>
>(new Map())

interface LatestTallySessionExecutionsProviderProps {
    enabled: boolean
    children: React.ReactNode
}

// Loads the latest execution of each tally session on the current page of the list.
const LatestTallySessionExecutionsProvider: React.FC<LatestTallySessionExecutionsProviderProps> = ({
    enabled,
    children,
}) => {
    const [tenantId] = useTenantStore()
    const {globalSettings} = useContext(SettingsContext)
    const {data: tallySessions} = useListContext<Sequent_Backend_Tally_Session>()

    const tallySessionIds = useMemo(
        () => tallySessions?.map((tallySession) => tallySession.id) ?? [],
        [tallySessions]
    )

    const {data: tallySessionExecutions} = useGetList<Sequent_Backend_Tally_Session_Execution>(
        "sequent_backend_tally_session_execution",
        {
            pagination: {page: 1, perPage: Math.max(tallySessionIds.length, 1)},
            sort: {field: "created_at", order: "DESC"},
            filter: {
                tally_session_id: {
                    format: "hasura-raw-query",
                    value: {_in: tallySessionIds},
                },
                tenant_id: tenantId,
            },
            meta: {latestPerTallySession: true},
        },
        {
            enabled,
            refetchInterval: globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const latestExecutionByTallySessionId = useMemo(
        () => getLatestExecutionByTallySessionId(tallySessionExecutions),
        [tallySessionExecutions]
    )

    return (
        <LatestTallySessionExecutionsContext.Provider value={latestExecutionByTallySessionId}>
            {children}
        </LatestTallySessionExecutionsContext.Provider>
    )
}

interface TallyActionsColumnProps {
    record: RaRecord
    actions: (
        record: RaRecord,
        latestExecution: Sequent_Backend_Tally_Session_Execution | undefined
    ) => Array<Action>
}

const TallyActionsColumn: React.FC<TallyActionsColumnProps> = ({record, actions}) => {
    const latestExecutionByTallySessionId = useContext(LatestTallySessionExecutionsContext)
    return (
        <ActionsColumn
            actions={actions(record, latestExecutionByTallySessionId.get(String(record.id)))}
        />
    )
}

export interface ListAreaProps {
    recordTally: Sequent_Backend_Tally_Session
}

export const ListTally: React.FC<ListAreaProps> = (props) => {
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

    const {data: keyRestoreState} = useQuery<ListTallyKeyRestoreStateQuery>(
        LIST_TALLY_KEY_RESTORE_STATE,
        {
            variables: {
                tenantId: tenantId,
                electionEventId: electionEventRecord?.id,
            },
            skip: !canTrusteeCeremony || !tenantId || !electionEventRecord?.id,
            pollInterval: globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
            context: {
                headers: {
                    "x-hasura-role": IPermissions.TRUSTEE_CEREMONY,
                },
            },
        }
    )

    const latestExecutionByTallySessionId = useMemo(
        () =>
            getLatestExecutionByTallySessionId(
                keyRestoreState?.sequent_backend_tally_session_execution
            ),
        [keyRestoreState?.sequent_backend_tally_session_execution]
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
            label={t("electionEventScreen.tally.create.createTallyButton")}
            onClick={() => {
                setIsCreatingTally(true)
                setCreatingFlag(ETallyType.ELECTORAL_RESULTS)
            }}
            disabled={!isKeyCeremonyFinished || !isPublished || isCreatingTally}
            style={{height: "10px"}}
            sx={{marginBottom: "10px"}}
        >
            <IconButton icon={faPlus} fontSize="24px" />
        </Button>
    )

    const CreateInitializationReportButton: React.FC<{isListActions: boolean}> = ({
        isListActions,
    }) => (
        <Button
            label={t("electionEventScreen.tally.create.createInitializationReportButton")}
            onClick={() => setCreatingFlag(ETallyType.INITIALIZATION_REPORT)}
            disabled={!isKeyCeremonyFinished || !isPublished}
        >
            {isListActions ? <Add /> : <IconButton icon={faPlus} fontSize="24px" />}
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

    const trusteeKeyRestoreEligibility = (
        record: RaRecord,
        latestExecution: Sequent_Backend_Tally_Session_Execution | undefined
    ): ETallyKeyRestoreEligibility =>
        getTallyKeyRestoreEligibility(
            getTallyTrusteeStatus(latestExecution, authContext.trustee),
            record.execution_status
        )

    const actions = (
        record: RaRecord,
        latestExecution: Sequent_Backend_Tally_Session_Execution | undefined
    ) => [
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
                trusteeKeyRestoreEligibility(record, latestExecution) ===
                ETallyKeyRestoreEligibility.ALLOWED ? (
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
                setCreatingFlag(null)
                refresh()
            }
        } catch (error) {
            console.log("TallyCeremony :: confirmCeremonyAction :: error", error)
            notify(t("tally.cancelTallyCeremonyError"), {type: "error"})
        }
    }

    // Returns an active tally ceremony in which the current trustee must restore a key.
    const getActiveCeremony = (
        tallySessions: ListTallyKeyRestoreStateQuery["sequent_backend_tally_session"] | undefined,
        trusteeName: string | null | undefined
    ) => {
        if (!tallySessions) {
            return
        } else {
            return tallySessions.find(
                (tallySession) =>
                    getTallyKeyRestoreEligibility(
                        getTallyTrusteeStatus(
                            latestExecutionByTallySessionId.get(tallySession.id),
                            trusteeName
                        ),
                        tallySession.execution_status
                    ) === ETallyKeyRestoreEligibility.ALLOWED
            )
        }
    }
    const activeCeremony = getActiveCeremony(
        keyRestoreState?.sequent_backend_tally_session,
        authContext.trustee
    )

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
            {canTrusteeCeremony && activeCeremony ? (
                <Alert severity="info">
                    <Trans i18nKey="electionEventScreen.tally.notify.participateNow">
                        {t("tally.invited")}
                        <NotificationLink
                            onClick={(e: any) => {
                                e.preventDefault()
                                viewTrusteeTally(activeCeremony.id)
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
                    <LatestTallySessionExecutionsProvider enabled={canTrusteeCeremony}>
                        <DatagridConfigurable omit={OMIT_FIELDS} bulkActionButtons={false}>
                            <TextField source="id" />
                            <FunctionField
                                label={t("electionEventScreen.tally.tallyType.label")}
                                render={(record: RaRecord<Identifier>) =>
                                    t(`electionEventScreen.tally.tallyType.${record.tally_type}`)
                                }
                            />
                            <DateField source="created_at" showTime={true} />

                            <FunctionField
                                key="permission_label"
                                label={t("electionEventScreen.tally.permissionLabels")}
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
                                label={t("electionEventScreen.tally.trustees")}
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
                                label={t("electionEventScreen.tally.electionNumber")}
                                render={(record: RaRecord<Identifier>) =>
                                    record?.election_ids?.length || 0
                                }
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
                                    <TallyActionsColumn record={record} actions={actions} />
                                )}
                            >
                                {/* <ActionsColumn actions={actions} /> */}
                            </FunctionField>
                        </DatagridConfigurable>
                    </LatestTallySessionExecutionsProvider>
                </List>
            }

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
