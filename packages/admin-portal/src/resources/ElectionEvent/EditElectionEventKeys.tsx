// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {BreadCrumbSteps, BreadCrumbStepsVariant} from "@sequentech/ui-essentials"
import { Sequent_Backend_Election_Event } from "@/gql/graphql"
import { IElectionEventStatus, IKeyCeremony, IKeyCeremonyStatus, getStatus } from "@/services/ElectionEventStatus"
import {styled} from "@mui/material/styles"
import { Box } from "@mui/material"
import React, { useState, useMemo } from "react"
import { useRecordContext } from "react-admin"
import { KeysGenerationStep } from "@/components/KeysGenerationStep"

const StyledBox = styled(Box)`
`

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
    const noCeremonies = (
        !status ||
        !status.keys_ceremony ||
        status.keys_ceremony.length == 0
    )
    
    const [showSteps, setShowSteps] = useState(noCeremonies)
    // This is the index of the current key ceremony
    const [currentCeremonyIndex, setCurrentCeremonyIndex] = useState(0)
    const currentCeremony: IKeyCeremony | null = useMemo(
        () => {
            if (
                status &&
                status.keys_ceremony &&
                status.keys_ceremony.length > currentCeremonyIndex
            ) {
                return status.keys_ceremony[currentCeremonyIndex]
            } else {
                return null
            }
        },
        [currentCeremonyIndex]
    )
    const currentStep: number = useMemo(
        () => {
            if (!currentCeremony) {
                return 0;
            }
            if (currentCeremony.status == IKeyCeremonyStatus.NOT_STARTED) {
                return 0;
            } else if (currentCeremony.status == IKeyCeremonyStatus.IN_PROCESS) {
                return 1;
            } else {
                return 2;
            }
        },
        []
    )
    const onCreate = (index: number) => {
        setCurrentCeremonyIndex(index)
    }

    return (
        <>
            {showSteps
                ? <StyledBox>
                    <BreadCrumbSteps
                        labels={[
                            "electionEventScreen.keys.breadCrumbs.configure",
                            "electionEventScreen.keys.breadCrumbs.ceremony",
                            "electionEventScreen.keys.breadCrumbs.created",
                        ]}
                        selected={currentStep}
                        variant={BreadCrumbStepsVariant.Circle}
                        colorPreviousSteps={true}
                    />
                    {currentStep == 0 &&
                        <KeysGenerationStep
                            onCreate={onCreate}
                            electionEvent={electionEvent}
                        />
                    }
                </StyledBox>
                : <span>TODO: show list</span>
            }
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
