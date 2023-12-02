// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import { Sequent_Backend_Election_Event, Sequent_Backend_Keys_Ceremony } from "@/gql/graphql"
import React, { useState } from "react"
import { useGetList, useRecordContext } from "react-admin"
import { KeyCeremonyWizard } from "@/components/key-ceremony/KeyCeremonyWizard"
import { useTenantStore } from "@/providers/TenantContextProvider"


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

    const empty = (
        <KeyCeremonyWizard
            electionEvent={electionEvent}
            keyCeremony={currentCeremony}
            setCurrentCeremony={setCurrentCeremony}
            forceNew={true}
            showCancel={false}
        />
    )

    if (!keyCeremonies || keyCeremonies.length == 0) {
        return empty
    }

    return (
        <>
            {currentCeremony
                ? <KeyCeremonyWizard
                    electionEvent={electionEvent}
                    keyCeremony={currentCeremony}
                    setCurrentCeremony={setCurrentCeremony}
                    forceNew={false}
                    showCancel={true}
                />
                : <span>TODO: show list</span>
            }
        </>
    )
}
