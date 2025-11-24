// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    ListKeysCeremonyQuery,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Keys_Ceremony,
    TrusteeNamesQuery,
} from "@/gql/graphql"
import {styled} from "@mui/material/styles"
import React, {ReactNode, useEffect, useMemo, useState} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    useRecordContext,
    DateField,
    Identifier,
    ReferenceArrayField,
    SingleFieldList,
    ChipField,
    FunctionField,
    RaRecord,
} from "react-admin"
import {Button, Typography, Chip, Alert, Box} from "@mui/material"
import {theme, IconButton} from "@sequentech/ui-essentials"
import {AdminWizard} from "@/components/keys-ceremony/AdminWizard"
import {TrusteeWizard, isTrusteeParticipating} from "@/components/keys-ceremony/TrusteeWizard"
import {statusColor} from "@/components/keys-ceremony/CeremonyStep"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {Action, ActionsColumn} from "@/components/ActionButons"
import {useTranslation, Trans} from "react-i18next"
import {useContext} from "react"
import {AuthContext, AuthContextValues} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import FileOpenIcon from "@mui/icons-material/FileOpen"
import KeyIcon from "@mui/icons-material/Key"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {ListActions} from "../../components/ListActions"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {ResetFilters} from "@/components/ResetFilters"
import {useQuery} from "@apollo/client"
import {LIST_KEYS_CEREMONY} from "@/queries/ListKeysCeremonies"
import {GET_TRUSTEES_NAMES} from "@/queries/GetTrusteesNames"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {useKeysPermissions} from "./useKeysPermissions"
import {TrusteeItems} from "@/components/TrusteeItems"
import {StyledChip} from "@/components/StyledChip"

const NotificationLink = styled("span")`
    text-decoration: underline;
    cursor: pointer;
    padding: 2px;

    &:hover {
        text-decoration: none;
    }
`

const TrusteeKeyIcon = styled(KeyIcon)`
    color: ${theme.palette.brandSuccess};
`

const StyledNull = styled("div")`
    display: block;
    padding-left: 18px;
`

interface StatusLabelProps {
    record: any
}

const StatusChip: React.FC<StatusLabelProps> = (props) => {
    const {record} = props
    return (
        <>
            <Chip
                sx={{
                    backgroundColor: statusColor(record["execution_status"]),
                    color: theme.palette.background.default,
                }}
                label={record["execution_status"]}
            />
        </>
    )
}

const OMIT_FIELDS: Array<string> = ["trustees"]

// Returns a keys ceremony if there's any in which we have been required to
// participate and is active
const getActiveCeremony = (
    keyCeremonies: Sequent_Backend_Keys_Ceremony[] | undefined,
    authContext: AuthContextValues
) => {
    if (!keyCeremonies) {
        return
    } else {
        return keyCeremonies.find((ceremony) => isTrusteeParticipating(ceremony, authContext))
    }
}

interface EditElectionEventKeysProps {
    isShowCeremony?: string | null
    isShowTrusteeCeremony?: string | null
}

export const EditElectionEventKeys: React.FC<EditElectionEventKeysProps> = (props) => {
    const {isShowCeremony, isShowTrusteeCeremony} = props

    useEffect(() => {
        if (isShowCeremony) {
            setShowCeremony(false)
        }
    }, [isShowCeremony])

    useEffect(() => {
        if (isShowTrusteeCeremony) {
            setShowTrusteeCeremony(false)
        }
    }, [isShowTrusteeCeremony])

    const {t} = useTranslation()
    const electionEvent = useRecordContext<Sequent_Backend_Election_Event>()
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const isTrustee = authContext.hasRole(IPermissions.TRUSTEE_CEREMONY)
    const {globalSettings} = useContext(SettingsContext)

    const {data: keysCeremonies} = useQuery<ListKeysCeremonyQuery>(LIST_KEYS_CEREMONY, {
        variables: {
            tenantId: tenantId,
            electionEventId: electionEvent?.id ?? "",
        },
        pollInterval: globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
        context: {
            headers: {
                "x-hasura-role": isTrustee
                    ? IPermissions.TRUSTEE_CEREMONY
                    : IPermissions.ADMIN_CEREMONY,
            },
        },
    })
    const keysCeremonyIds = useMemo(() => {
        return keysCeremonies?.list_keys_ceremony?.items.map((key) => key?.id) ?? []
    }, [keysCeremonies?.list_keys_ceremony?.items])

    let activeCeremony = getActiveCeremony(
        keysCeremonies?.list_keys_ceremony?.items as any,
        authContext
    )

    const {data: trusteeNames} = useQuery<TrusteeNamesQuery>(GET_TRUSTEES_NAMES, {
        variables: {
            tenantId: tenantId,
        },
    })

    // This is the ceremony currently being shown
    const [currentCeremony, setCurrentCeremony] = useState<Sequent_Backend_Keys_Ceremony | null>(
        null
    )

    const [showCeremony, setShowCeremony] = useState(false)
    const [showTrusteeCeremony, setShowTrusteeCeremony] = useState(false)
    const {
        canAdminCeremony,
        canTrusteeCeremony,
        canExportCeremony,
        canCreateCeremony,
        showKeysColumns,
    } = useKeysPermissions()

    const CreateButton = () => (
        <Button
            onClick={() => setShowCeremony(true)}
            disabled={!keysCeremonies}
            className="keys-add-button"
        >
            <ResourceListStyles.CreateIcon icon={faPlus as any} />
            {t("electionEventScreen.keys.createNew")}
        </Button>
    )

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t("electionEventScreen.keys.emptyHeader")}
            </Typography>
            {canCreateCeremony ? (
                <>
                    <Typography variant="body1" paragraph>
                        {t("common.resources.noResult.askCreate")}
                    </Typography>
                    <CreateButton />
                </>
            ) : null}
        </ResourceListStyles.EmptyBox>
    )

    const goBack = () => {
        setShowCeremony(false)
        setShowTrusteeCeremony(false)
        setCurrentCeremony(null)
    }

    const getCeremony = (id: Identifier): Sequent_Backend_Keys_Ceremony | undefined => {
        if (keysCeremonies) {
            return keysCeremonies?.list_keys_ceremony?.items.find(
                (element) => element?.id === id
            ) as any
        }
    }

    const viewAdminCeremony = (id: Identifier) => {
        const ceremony = getCeremony(id)
        if (!ceremony || !canAdminCeremony) {
            return
        } else {
            setCurrentCeremony(ceremony)
            setShowCeremony(true)
            setShowTrusteeCeremony(false)
        }
    }
    const viewTrusteeCeremony = (id: Identifier) => {
        const ceremony = getCeremony(id)
        if (!ceremony || !canTrusteeCeremony) {
            return
        } else {
            setCurrentCeremony(ceremony)
            setShowCeremony(false)
            setShowTrusteeCeremony(true)
        }
    }

    const actions: Action[] = [
        {
            icon: <FileOpenIcon className="keys-view-admin-icon" />,
            action: viewAdminCeremony,
            showAction: (id: Identifier) => canAdminCeremony && !!getCeremony(id),
        },
        {
            icon: <TrusteeKeyIcon className="keys-view-trustee-icon" />,
            action: viewTrusteeCeremony,
            showAction: (id: Identifier) => canTrusteeCeremony && !!getCeremony(id),
        },
    ]

    return (
        <>
            {/* Show the notification if the conditions are met */}
            {canTrusteeCeremony && activeCeremony && !showCeremony && !showTrusteeCeremony && (
                <Alert severity="info">
                    <Trans i18nKey="electionEventScreen.keys.notify.participateNow">
                        You have been invited to participate in a Keys ceremony. Please
                        <NotificationLink
                            onClick={(e: any) => {
                                // TODO: this onClick is not being called!
                                console.log("clicked")

                                e.preventDefault()
                                viewTrusteeCeremony(activeCeremony?.id)
                            }}
                        >
                            click on the ceremony Key Action
                        </NotificationLink>
                        to participate.
                    </Trans>
                </Alert>
            )}
            {/* Show the admin keys ceremony steps if the conditions are met */}
            {canAdminCeremony && showCeremony && (
                <AdminWizard
                    electionEvent={electionEvent}
                    currentCeremony={currentCeremony}
                    setCurrentCeremony={setCurrentCeremony}
                    goBack={goBack}
                />
            )}
            {/* Show the trustees keys ceremony steps if the conditions are met */}
            {canTrusteeCeremony && showTrusteeCeremony && currentCeremony && (
                <TrusteeWizard
                    electionEvent={electionEvent}
                    currentCeremony={currentCeremony}
                    setCurrentCeremony={setCurrentCeremony}
                    goBack={goBack}
                />
            )}
            {/* Show the keys ceremony table list */}
            {!showCeremony && !showTrusteeCeremony && (
                <List
                    resource="sequent_backend_keys_ceremony"
                    filter={{
                        tenant_id: tenantId || undefined,
                        election_event_id: electionEvent?.id || undefined,
                        id: {
                            format: "hasura-raw-query",
                            value: {_in: keysCeremonyIds},
                        },
                    }}
                    storeKey={false}
                    empty={<Empty />}
                    actions={
                        <ListActions
                            withColumns={showKeysColumns}
                            withFilter={false}
                            withImport={false}
                            withExport={false}
                            actionLabel="common.label.add"
                            doAction={() => setShowCeremony(true)}
                            withAction={canCreateCeremony}
                        />
                    }
                >
                    <ResetFilters />
                    <DatagridConfigurable omit={OMIT_FIELDS} bulkActionButtons={<></>}>
                        <TextField source="id" />
                        <TextField source="name" />
                        <DateField
                            source="created_at"
                            showTime={true}
                            label={String(t("electionEventScreen.keys.started"))}
                        />

                        <FunctionField
                            label={String(t("electionEventScreen.keys.statusLabel"))}
                            render={(record: any) => <StatusChip record={record} />}
                        />

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
                        <ActionsColumn
                            actions={actions}
                            label={String(t("common.label.actions"))}
                        />
                    </DatagridConfigurable>
                </List>
            )}
        </>
    )
}
