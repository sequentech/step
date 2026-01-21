// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {useGetList} from "react-admin"

import {Sequent_Backend_Tally_Session_Execution} from "../../gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ILog, ITallyCeremonyStatus} from "@/types/ceremonies"
import {Logs} from "@/components/Logs"

interface TallyLogsProps {
    tallySessionExecution?: Sequent_Backend_Tally_Session_Execution
}

export const TallyLogs: React.FC<TallyLogsProps> = ({tallySessionExecution}) => {
    let status = tallySessionExecution?.status as ITallyCeremonyStatus | undefined

    return <Logs logs={status?.logs} />
}
