// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, { useState } from "react"
import { CreateScheduledEventMutation, Sequent_Backend_Election_Event } from "@/gql/graphql"
import { useRecordContext, useRefresh } from "react-admin"
import { Box, Button, CircularProgress } from "@mui/material"
import { useMutation } from "@apollo/client"
import { CREATE_SCHEDULED_EVENT } from "@/queries/CreateScheduledEvent"
import { useTenantStore } from "@/providers/TenantContextProvider"
import { ScheduledEventType } from "@/services/ScheduledEvent"

export const EditElectionEventPublish: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const [createScheduledEvent] = useMutation<CreateScheduledEventMutation>(CREATE_SCHEDULED_EVENT)
    const refresh = useRefresh()
    const [showProgress, setShowProgress] = useState(false)
    const [tenantId] = useTenantStore()


    const createBallotStylesAction = async () => {
        setShowProgress(true)

        const {data, errors} = await createScheduledEvent({
            variables: {
                tenantId: tenantId,
                electionEventId: record.id,
                eventProcessor: ScheduledEventType.CREATE_ELECTION_EVENT_BALLOT_STYLES,
                cronConfig: undefined,
                eventPayload: {},
                createdBy: "admin",
            },
        })
        if (errors) {
            console.log(errors)
        }
        if (data) {
            console.log(data)
        }
        setShowProgress(false)
        refresh()
    }

    return <Box>
         Actions {showProgress ? <CircularProgress /> : null}
        <Button onClick={createBallotStylesAction}>Publish</Button>

    </Box>
}