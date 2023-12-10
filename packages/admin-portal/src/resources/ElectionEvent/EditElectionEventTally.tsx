// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {StartTallyDialog} from "@/components/StartTallyDialog"
import {Sequent_Backend_Election_Event, Sequent_Backend_Tally_Session} from "@/gql/graphql"
import {Box, Button, Typography} from "@mui/material"
import React, {useState} from "react"
import {CreateButton, useRecordContext} from "react-admin"
import {ListTally} from "../Tally/ListTally"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {TallyCeremony} from "../Tally/TallyCeremony"
import {TallyCeremonyTrustees} from "../Tally/TallyCeremonyTrustees"
import {AuthContext, AuthContextValues} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {styled as MUIStiled} from "@mui/material/styles"
import {useActionPermissions} from "./EditElectionEventKeys"
import {useTranslation} from "react-i18next"

const EmptyBox = MUIStiled(Box)`
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    width: 100%;
`

export const EditElectionEventTally: React.FC = () => {
    const {t} = useTranslation()
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
                    {!isTrustee ? (
                        <TallyCeremony completed={record.is_execution_completed} />
                    ) : (
                        <TallyCeremonyTrustees />
                    )}
                </>
            ) : (
                <ListTally record={record} />
            )}
        </Box>
    )
}
