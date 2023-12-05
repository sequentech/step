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
} from "react-admin"
import {Box, Button, Typography} from "@mui/material"
import {IconButton} from "@sequentech/ui-essentials"
import {Wizard} from "@/components/keys-ceremony/Wizard"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {Action, ActionsColumn} from "@/components/ActionButons"
import {useTranslation} from "react-i18next"
import {useContext} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import FileOpenIcon from "@mui/icons-material/FileOpen"

const EmptyBox = styled(Box)`
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    width: 100%;
`

export function useActionPermissions() {
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)

    const canAdminCeremony = authContext.isAuthorized(true, tenantId, IPermissions.ADMIN_CEREMONY)
    const canReadTrustee = authContext.isAuthorized(true, tenantId, IPermissions.TRUSTEE_READ)

    return {
        canAdminCeremony,
        canReadTrustee,
    }
}

const OMIT_FIELDS: Array<string> = []

export const EditElectionEventKeys: React.FC = () => {
    const {t} = useTranslation()
    const electionEvent = useRecordContext<Sequent_Backend_Election_Event>()
    const [tenantId] = useTenantStore()

    const {data: keyCeremonies} = useGetList<Sequent_Backend_Keys_Ceremony>(
        "sequent_backend_keys_ceremony",
        {
            sort: {field: "created_at", order: "DESC"},
            filter: {
                tenant_id: tenantId,
                election_event_id: electionEvent.id,
            },
        }
    )

    // This is the ceremony currently being shown
    const [currentCeremony, setCurrentCeremony] = useState<Sequent_Backend_Keys_Ceremony | null>(
        null
    )

    const [showCeremony, setShowCeremony] = useState(false)
    const {canAdminCeremony, canReadTrustee} = useActionPermissions()

    const CreateButton = () => (
        <Button
            onClick={() => setShowCeremony(true)}
            disabled={!keyCeremonies || keyCeremonies?.length > 0}
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
                        {t("electionEventScreen.keys.emptyBody")}
                    </Typography>
                    <CreateButton />
                </>
            ) : null}
        </EmptyBox>
    )

    const goBack = () => {
        setShowCeremony(false)
        setCurrentCeremony(null)
    }

    const viewAction = (id: Identifier) => {
        const ceremony: Sequent_Backend_Keys_Ceremony | undefined = keyCeremonies?.find(
            (element) => element.id === id
        )
        if (!ceremony) {
            return
        } else {
            setCurrentCeremony(ceremony)
            setShowCeremony(true)
        }
    }

    const actions: Action[] = [{icon: <FileOpenIcon />, action: viewAction}]

    return (
        <>
            {showCeremony ? (
                <Wizard
                    electionEvent={electionEvent}
                    currentCeremony={currentCeremony}
                    setCurrentCeremony={setCurrentCeremony}
                    goBack={goBack}
                />
            ) : (
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
                        <TextField source="execution_status" />
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
