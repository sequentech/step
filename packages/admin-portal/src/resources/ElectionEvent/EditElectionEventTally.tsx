// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {StartTallyDialog} from "@/components/StartTallyDialog"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {Box, Button} from "@mui/material"
import React, {useState} from "react"
import {useRecordContext} from "react-admin"

export const EditElectionEventTally: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const [showStartTallyDialog, setShowStartTallyDialog] = useState(false)

    const openStartTallyDialog = () => {
        console.log("opening...")
        setShowStartTallyDialog(true)
    }

    return (
        <Box>
            <StartTallyDialog
                show={showStartTallyDialog}
                handleClose={() => setShowStartTallyDialog(false)}
                electionEvent={record}
            />
            <Button onClick={openStartTallyDialog}>Start tally</Button>
        </Box>
    )
}
