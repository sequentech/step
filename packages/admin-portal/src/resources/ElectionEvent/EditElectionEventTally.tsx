// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {StartTallyDialog} from "@/components/StartTallyDialog"
import {Sequent_Backend_Election_Event, Sequent_Backend_Tally_Session} from "@/gql/graphql"
import {Box, Button} from "@mui/material"
import React, {useState} from "react"
import {useRecordContext} from "react-admin"
import {ListTally} from "../Tally/ListTally"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {TallyCeremony} from "../Tally/TallyCeremony"

export const EditElectionEventTally: React.FC = () => {
    const recordEvent = useRecordContext<Sequent_Backend_Election_Event>()
    const record = useRecordContext<Sequent_Backend_Tally_Session>()
    const [showStartTallyDialog, setShowStartTallyDialog] = useState(false)
    const [tallyId, setTallyId, isTrustee] = useElectionEventTallyStore()

    console.log("EditElectionEventTally :: tallyId :: ", tallyId)

    const openStartTallyDialog = () => {
        console.log("opening...")
        setShowStartTallyDialog(true)
    }

    return (
        <Box>
            <StartTallyDialog
                show={showStartTallyDialog}
                handleClose={() => setShowStartTallyDialog(false)}
                electionEvent={recordEvent}
            />
            {/* <Button onClick={openStartTallyDialog}>Start tally</Button> */}
            {tallyId ? (
                <>
                    {!isTrustee ? <TallyCeremony completed={record.is_execution_completed} /> : null}
                </>
            ) : (
                <ListTally record={record} />
            )}
        </Box>
    )
}
