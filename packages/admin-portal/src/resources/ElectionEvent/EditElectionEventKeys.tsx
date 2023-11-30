// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {BreadCrumbSteps, BreadCrumbStepsVariant} from "@sequentech/ui-essentials"
import { Sequent_Backend_Election_Event } from "@/gql/graphql"
import { IElectionEventStatus, getStatus } from "@/services/ElectionEventStatus"
import { Box, Button } from "@mui/material"
import React, { useState } from "react"
import { useRecordContext } from "react-admin"

const getSelected = (electionEvent: Sequent_Backend_Election_Event) => {
    const status: IElectionEventStatus = getStatus(electionEvent.status)
    if (electionEvent.public_key) {
        return 1; // created
    }
    return -1;
}

export const EditElectionEventKeys: React.FC = () => {
    const electionEvent = useRecordContext<Sequent_Backend_Election_Event>()
    const status: IElectionEventStatus = getStatus(electionEvent.status)
    
    // const [showCreateKeysDialog, setShowCreateKeysDialog] = useState(false)
    //const openKeysDialog = () => {
    //    console.log("opening...")
    //    setShowCreateKeysDialog(true)
    //}

    return (
        <>
            <Box sx={{width: 1024, marginX: "auto"}}>
                <BreadCrumbSteps
                    labels={[
                        "electionEventResource.keysTab.breadCrumbs.configure",
                        "electionEventResource.keysTab.breadCrumbs.ceremony",
                        "electionEventResource.keysTab.breadCrumbs.created",
                    ]}
                    selected={1}
                    variant={BreadCrumbStepsVariant.Circle}
                    colorPreviousSteps={true}
                />
            </Box>
        </>
    )
/*
    return <Box>

        <KeysGenerationDialog
            show={showCreateKeysDialog}
            handleClose={() => setShowCreateKeysDialog(false)}
            electionEvent={record}
        />
        <Button onClick={openKeysDialog}>Add keys</Button>

    </Box>*/
}