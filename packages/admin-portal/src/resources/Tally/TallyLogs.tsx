// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {memo, useEffect, useState} from "react"
import {useGetOne} from "react-admin"

import {Sequent_Backend_Tally_Session} from "../../gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {JsonView} from "@/components/JsonView"
import {JSON_MOCK} from "./constants"

// interface TallyLogsProps {
//     tally: Sequent_Backend_Tally_Session | undefined
// }

const TallyLogs: React.FC = () => {
    const [tallyId] = useElectionEventTallyStore()
    const [dataTally, setDataTally] = useState<Sequent_Backend_Tally_Session>()

    const {data} = useGetOne<Sequent_Backend_Tally_Session>("sequent_backend_tally_session", {
        id: tallyId,
    })

    useEffect(() => {
        if (data) {
            console.log("data in resultas", data)

            setDataTally(data)
        }
    }, [data])

    return <JsonView origin={JSON_MOCK} />
}

export default memo(TallyLogs)
