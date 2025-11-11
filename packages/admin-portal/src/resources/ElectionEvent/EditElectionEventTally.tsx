// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Sequent_Backend_Election_Event, Sequent_Backend_Tally_Session} from "@/gql/graphql"
import {Box} from "@mui/material"
import React, {useState} from "react"
import {useRecordContext} from "react-admin"
import {ListTally} from "../Tally/ListTally"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {TallyCeremony} from "../Tally/TallyCeremony"
import {TallyCeremonyTrustees} from "../Tally/TallyCeremonyTrustees"
import {MiruExportWizard} from "@/components/MiruExportWizard"

export const EditElectionEventTally: React.FC = () => {
    const {
        tallyId,
        isTrustee,
        creatingType: isCreatingType,
        isCreated,
        selectedTallySessionData,
    } = useElectionEventTallyStore()

    return (
        <Box>
            {selectedTallySessionData ? (
                <MiruExportWizard />
            ) : isCreatingType || isCreated || tallyId ? (
                <>{!isTrustee ? <TallyCeremony /> : <TallyCeremonyTrustees />}</>
            ) : (
                <ListTally />
            )}
        </Box>
    )
}
