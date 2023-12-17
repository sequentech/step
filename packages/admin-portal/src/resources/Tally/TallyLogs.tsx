// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {useGetList} from "react-admin"

import {Sequent_Backend_Tally_Session_Execution} from "../../gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ILog, ITallyCeremonyStatus} from "@/types/ceremonies"
import globalSettings from "@/global-settings"
import {Logs} from "@/components/Logs"

export const TallyLogs: React.FC = () => {
    const {tallyId} = useElectionEventTallyStore()
    const [tenantId] = useTenantStore()
    const [dataTally, setDataTally] = useState<Array<ILog>>([])

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
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
        }
    )

    useEffect(() => {
        if (!tallySessionExecutions?.[0].status) {
            return
        }

        let status = tallySessionExecutions?.[0].status as ITallyCeremonyStatus | undefined

        if (status?.logs) {
            setDataTally(status.logs)
        }
    }, [tallySessionExecutions])

    return <Logs logs={dataTally} />
}
