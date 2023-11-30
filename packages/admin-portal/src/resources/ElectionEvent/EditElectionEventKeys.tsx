// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {KeysGenerationDialog} from "@/components/KeysGenerationDialog"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {Box, Button} from "@mui/material"
import React, {useState} from "react"
import {useRecordContext} from "react-admin"

export const EditElectionEventKeys: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const [showCreateKeysDialog, setShowCreateKeysDialog] = useState(false)

    const openKeysDialog = () => {
        console.log("opening...")
        setShowCreateKeysDialog(true)
    }

    return (
        <Box>
            <KeysGenerationDialog
                show={showCreateKeysDialog}
                handleClose={() => setShowCreateKeysDialog(false)}
                electionEvent={record}
            />
            <Button onClick={openKeysDialog}>Add keys</Button>
        </Box>
    )
}
