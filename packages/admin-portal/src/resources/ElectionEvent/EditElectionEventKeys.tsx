// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Sequent_Backend_Election_Event, Sequent_Backend_Keys_Ceremony} from "@/gql/graphql"
import {styled} from "@mui/material/styles"
import React, {useState} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    TopToolbar,
    useGetList,
    useRecordContext,
    DateField,
    Identifier,
    ReferenceArrayField,
    SingleFieldList,
    ChipField,
    FunctionField,
} from "react-admin"
import {Box, Button, Typography, Chip, Alert} from "@mui/material"
import {theme, IconButton} from "@sequentech/ui-essentials"
import {AdminWizard} from "@/components/keys-ceremony/AdminWizard"
import {
    TrusteeWizard,
    isTrusteeParticipating
} from "@/components/keys-ceremony/TrusteeWizard"
import {statusColor} from "@/components/keys-ceremony/CeremonyStep"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {Action, ActionsColumn} from "@/components/ActionButons"
import {useTranslation, Trans} from "react-i18next"
import {useContext} from "react"
import {AuthContext, AuthContextValues} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import FileOpenIcon from "@mui/icons-material/FileOpen"
import KeyIcon from '@mui/icons-material/Key'

const EmptyBox = styled(Box)`
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    width: 100%;
`

// This could be react-router-dom's Link for example
const Link: (array: any) => any = ({ className, children }) => (
    <a className={className}>
        {children}
    </a>
)

const NotificationLink = styled(Link)`
    //text-decoration: underline;
    //cursor: pointer;
    //
    //&:hover {
    //    text-decoration: none;
    //}
`;

const TrusteeKeyIcon = styled(KeyIcon)`
    color: ${theme.palette.brandSuccess};
`

export function useActionPermissions() {
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)

    const canAdminCeremony = authContext.isAuthorized(true, tenantId, IPermissions.ADMIN_CEREMONY)
    const canTrusteeCeremony = authContext.isAuthorized(true, tenantId, IPermissions.TRUSTEE_CEREMONY)

    return {
        canAdminCeremony,
        canTrusteeCeremony,
    }
}

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

const OMIT_FIELDS: Array<string> = []

// Returns a keys ceremony if there's any in which we have been required to
// participate and is active
const getActiveCeremony = 
    (
        keyCeremonies: Sequent_Backend_Keys_Ceremony[] | undefined,
        authContext: AuthContextValues
    ) => {
        if (!keyCeremonies) {
            return
        } else {
            return keyCeremonies.find((ceremony) =>
                isTrusteeParticipating(ceremony, authContext)
            )
        }
    }


export const EditElectionEventKeys: React.FC = () => {
    const {t} = useTranslation()
    const electionEvent = useRecordContext<Sequent_Backend_Election_Event>()
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)

    const {data: keysCeremonies} = useGetList<Sequent_Backend_Keys_Ceremony>(
        "sequent_backend_keys_ceremony",
        {
            sort: {field: "created_at", order: "DESC"},
            filter: {
                tenant_id: tenantId,
                election_event_id: electionEvent.id,
            },
        }
    )
    let activeCeremony = getActiveCeremony(keysCeremonies, authContext)

    // This is the ceremony currently being shown
    const [currentCeremony, setCurrentCeremony] = useState<Sequent_Backend_Keys_Ceremony | null>(
        null
    )

    const [showCeremony, setShowCeremony] = useState(false)
    const [showTrusteeCeremony, setShowTrusteeCeremony] = useState(false)
    const {canAdminCeremony, canTrusteeCeremony: canWriteTrustee} = useActionPermissions()

    const CreateButton = () => (
        <Button
            onClick={() => setShowCeremony(true)}
            disabled={!keysCeremonies || keysCeremonies?.length > 0}
        >
            <IconButton icon={faPlus} fontSize="24px" />
            {t("electionEventScreen.keys.createNew")}
        </Button>
    )

    const Empty = () => (
        <EmptyBox m={1}>
            <Typography variant="h4" paragraph>
                {t("electionEventScreen.keys.emptyHeader")}
            </Typography>
            {canAdminCeremony ? (
                <>
                    <Typography variant="body1" paragraph>
                        {t("common.resources.noResult.askCreate")}
                    </Typography>
                    <CreateButton />
                </>
            ) : null}
        </EmptyBox>
    )

    const goBack = () => {
        setShowCeremony(false)
        setShowTrusteeCeremony(false)
        setCurrentCeremony(null)
    }

    const getCeremony = (id: Identifier) => {
        if (keysCeremonies) {
            return keysCeremonies?.find(
                (element) => element.id === id
            )
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
        if (!ceremony || !canWriteTrustee) {
            return
        } else {
            setCurrentCeremony(ceremony)
            setShowCeremony(false)
            setShowTrusteeCeremony(true)
        }
    }

    const actions: Action[] = [
        {
            icon: <FileOpenIcon />,
            action: viewAdminCeremony,
            showAction: (id: Identifier) => canAdminCeremony && !!getCeremony(id),
        },
        {
            icon: <TrusteeKeyIcon />,
            action: viewTrusteeCeremony,
            showAction: (id: Identifier) => (
                canWriteTrustee &&
                !!getCeremony(id)
            ),
        }
    ]

    return (
        <>
            {canWriteTrustee && activeCeremony && !showCeremony && !showTrusteeCeremony &&
                <Alert severity="info">
                    <Trans i18nKey="electionEventScreen.keys.notify.participateNow">
                        You have been invited to participate in a Keys ceremony. Please
                        <NotificationLink
                            onClick={(e: any) => {
                                // TODO: this onClick is not being called!
                                e.preventDefault()
                                viewAdminCeremony(activeCeremony?.id)
                            }}
                            >click on the ceremony Key Action</NotificationLink> to participate.
                    </Trans>
                </Alert>}
            {canAdminCeremony && showCeremony && (
                <AdminWizard
                    electionEvent={electionEvent}
                    currentCeremony={currentCeremony}
                    setCurrentCeremony={setCurrentCeremony}
                    goBack={goBack}
                />
            )}
            {canWriteTrustee && showTrusteeCeremony && currentCeremony && (
                <TrusteeWizard
                    electionEvent={electionEvent}
                    currentCeremony={currentCeremony}
                    setCurrentCeremony={setCurrentCeremony}
                    goBack={goBack}
                />
            )}
            {!showCeremony && !showTrusteeCeremony && (
                <List
                    resource="sequent_backend_keys_ceremony"
                    actions={<TopToolbar>{canAdminCeremony ? <CreateButton /> : null}</TopToolbar>}
                    filter={{
                        tenant_id: tenantId || undefined,
                        election_event_id: electionEvent?.id || undefined,
                    }}
                    empty={<Empty />}
                >
                    <DatagridConfigurable omit={OMIT_FIELDS} bulkActionButtons={<></>}>
                        <TextField source="id" />
                        <DateField source="created_at" showTime={true} />

                        <FunctionField
                            label={t("electionEventScreen.keys.statusLabel")}
                            render={(record: any) => <StatusChip record={record} />}
                        />

                        <ReferenceArrayField
                            perPage={10}
                            reference="sequent_backend_trustee"
                            source="trustee_ids"
                        >
                            <SingleFieldList linkType={false}>
                                <ChipField source="name" />
                            </SingleFieldList>
                        </ReferenceArrayField>
                        <ActionsColumn actions={actions} label={t("common.label.actions")} />
                    </DatagridConfigurable>
                </List>
            )}
        </>
    )
}
