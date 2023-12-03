// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import { Sequent_Backend_Election_Event, Sequent_Backend_Keys_Ceremony } from "@/gql/graphql"
import {styled} from "@mui/material/styles"
import React, { useState } from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    ExportButton,
    SelectColumnsButton,
    TopToolbar,
    useGetList,
    useRecordContext,
    Link,
    DateField,
} from "react-admin"
import {Box, Button, Typography} from "@mui/material"
import {IconButton} from "@sequentech/ui-essentials"
import { KeyCeremonyWizard } from "@/components/key-ceremony/KeyCeremonyWizard"
import { faPlus } from "@fortawesome/free-solid-svg-icons"
import { useTenantStore } from "@/providers/TenantContextProvider"
import { Action, ActionsColumn } from "@/components/ActionButons"
import { useTranslation } from "react-i18next"

const EmptyBox = styled(Box)`
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
`


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
    const [currentCeremony, setCurrentCeremony] = 
        useState<Sequent_Backend_Keys_Ceremony | null>(null)

    const [showCeremony, setShowCeremony] = useState(false)

    const Empty = () => (
        <EmptyBox m={1}>
            <Typography variant="h4" paragraph>
                {t("electionEventScreen.keys.emptyHeader")}
            </Typography>
            <Typography variant="body1" paragraph>
            {t("electionEventScreen.keys.emptyBody")}
            </Typography>
            <Button onClick={() => setShowCeremony(true)}>
                <IconButton icon={faPlus} fontSize="24px" />
                {t("electionEventScreen.keys.createNew")}
            </Button>
        </EmptyBox>
    )

    const goBack = () => {
        setShowCeremony(false)
        setCurrentCeremony(null)
    }

    const actions: Action[] = [
        // access
    ]

    if (!showCeremony) {
        return <Empty />
    }

    return (
        <>
            {showCeremony
                ? <KeyCeremonyWizard
                    electionEvent={electionEvent}
                    keyCeremony={currentCeremony}
                    setCurrentCeremony={setCurrentCeremony}
                    goBack={goBack}
                    forceNew={false}
                />
                : <List
                    resource="keys_ceremony"
                    actions={
                        <TopToolbar>
                            <SelectColumnsButton />
                            <ExportButton />
                        </TopToolbar>
                    }
                    filter={{tenant_id: tenantId}}
                    empty={<Empty />}
                >
                    <DatagridConfigurable 
                        omit={OMIT_FIELDS}
                        bulkActionButtons={<></>}
                    >
                        <TextField source="id" />
                        <DateField source="created_at" />
                        <TextField source="status" />
                        <ActionsColumn actions={actions} />
                    </DatagridConfigurable>
                </List>
            }
        </>
    )
}
