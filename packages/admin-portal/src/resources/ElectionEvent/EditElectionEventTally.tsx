// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {StartTallyDialog} from "@/components/StartTallyDialog"
import {Sequent_Backend_Election_Event, Sequent_Backend_Tally_Session} from "@/gql/graphql"
import {Box} from "@mui/material"
import React, {useState} from "react"
import {useRecordContext} from "react-admin"
import {ListTally} from "../Tally/ListTally"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {TallyCeremony} from "../Tally/TallyCeremony"
import {TallyCeremonyTrustees} from "../Tally/TallyCeremonyTrustees"
import {useTranslation} from "react-i18next"

export const EditElectionEventTally: React.FC = () => {
    const {t} = useTranslation()
    const recordEvent = useRecordContext<Sequent_Backend_Election_Event>()
    const recordTally = useRecordContext<Sequent_Backend_Tally_Session>()
    const [showStartTallyDialog, setShowStartTallyDialog] = useState(false)
    const {tallyId, isTrustee, isCreating, isCreated} = useElectionEventTallyStore()

    console.log("EditElectionEventTally :: tallyId ::  ", tallyId);
    console.log("EditElectionEventTally :: isCreating ::  ", isCreating);
    

    return (
        <Box>
            <StartTallyDialog
                show={showStartTallyDialog}
                handleClose={() => setShowStartTallyDialog(false)}
                electionEvent={recordEvent}
            />

            {isCreating || isCreated || tallyId ? (
                <>
                    {!isTrustee ? (
                        <TallyCeremony   />
                    ) : (
                        <TallyCeremonyTrustees />
                    )}
                </>
            ) : (
                <ListTally recordTally={recordTally} />
            )}
        </Box>
    )
}
