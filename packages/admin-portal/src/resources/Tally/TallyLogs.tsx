// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {useGetList} from "react-admin"

import {Sequent_Backend_Tally_Session_Execution} from "../../gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {JsonView} from "@/components/JsonView"
import {useTenantStore} from "@/providers/TenantContextProvider"

export const TallyLogs: React.FC = () => {
    const {tallyId} = useElectionEventTallyStore()
    const [tenantId] = useTenantStore()
    const [dataTally, setDataTally] = useState<object | null>(null)

    const {data: tallySessionExecutions} = useGetList<Sequent_Backend_Tally_Session_Execution>(
        "sequent_backend_tally_session_execution",
        {
            pagination: {page: 1, perPage: 1},
            sort: {field: "created_at", order: "DESC"},
            filter: {
                tally_session_id: tallyId,
                tenant_id: tenantId,
            },
        },
        {
            refetchInterval: 5000,
        }
    )

    useEffect(() => {
        if (!tallySessionExecutions?.[0].status) {
            return
        }

        let jsonData = null
        if (tallySessionExecutions?.[0].status.logs[0]) {
            jsonData = JSON.parse(tallySessionExecutions?.[0].status.logs[0])
        }
        setDataTally(jsonData)
    }, [tallySessionExecutions])

    return <>{dataTally ? <JsonView origin={dataTally} /> : <p>No logs available</p>}</>
}
