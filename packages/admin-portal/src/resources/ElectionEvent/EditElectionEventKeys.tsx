// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import { Sequent_Backend_Election_Event, Sequent_Backend_Keys_Ceremony } from "@/gql/graphql"
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
} from "react-admin"
import {Button} from "@mui/material"
import {IconButton} from "@sequentech/ui-essentials"
import { KeyCeremonyWizard } from "@/components/key-ceremony/KeyCeremonyWizard"
import {faPlusCircle} from "@fortawesome/free-solid-svg-icons"
import { useTenantStore } from "@/providers/TenantContextProvider"
import { Action, ActionsColumn } from "@/components/ActionButons"

const OMIT_FIELDS: Array<string> = []

export const EditElectionEventKeys: React.FC = () => {
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

    const empty = (
            <Button onClick={() => setShowCeremony(true)}>
                <IconButton icon={faPlusCircle} fontSize="24px" />
                Create new election event key ceremony
            </Button>
    )

    const actions: Action[] = [
        // view, edit
    ]

    if (!keyCeremonies || keyCeremonies.length == 0) {
        return empty
    }

    return (
        <>
            {showCeremony
                ? <KeyCeremonyWizard
                    electionEvent={electionEvent}
                    keyCeremony={currentCeremony}
                    setCurrentCeremony={setCurrentCeremony}
                    forceNew={false}
                />
                : <List
                    resource="role"
                    actions={
                        <TopToolbar>
                            <SelectColumnsButton />
                            <ExportButton />
                        </TopToolbar>
                    }
                    filter={{tenant_id: tenantId}}
                >
                    <DatagridConfigurable 
                        omit={OMIT_FIELDS}
                        bulkActionButtons={<></>}
                    >
                        <TextField source="name" />
                        <TextField source="id" />
                        <ActionsColumn actions={actions} />
                    </DatagridConfigurable>
                </List>
            }
        </>
    )
}
