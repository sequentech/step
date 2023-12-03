// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {BreadCrumbSteps, BreadCrumbStepsVariant} from "@sequentech/ui-essentials"
import { Sequent_Backend_Election_Event, Sequent_Backend_Keys_Ceremony } from "@/gql/graphql"
import {styled} from "@mui/material/styles"
import { Box } from "@mui/material"
import React, { useState } from "react"
import { ConfigureStep } from "@/components/keys-ceremony/ConfigureStep"
import { CeremonyStep } from "@/components/keys-ceremony/CeremonyStep"
import { IKeysCeremonyExecutionStatus } from "@/services/KeyCeremony"

const StyledBox = styled(Box)`
`

interface WizardProps {
    electionEvent: Sequent_Backend_Election_Event

    keysCeremony: Sequent_Backend_Keys_Ceremony | null
    setCurrentCeremony: (keysCeremony: Sequent_Backend_Keys_Ceremony) => void

    goBack: () => void
}

export const Wizard: React.FC<WizardProps> = ({
    electionEvent,
    keysCeremony,
    setCurrentCeremony,
    goBack,
}) => {
    const calculateCurrentStep: () => number = () => {
        if (!keysCeremony) {
            return 0 // configure
        } else {
            if (keysCeremony.execution_status == IKeysCeremonyExecutionStatus.NOT_STARTED) {
                return 1 // ceremony
            } else if (keysCeremony.execution_status == IKeysCeremonyExecutionStatus.IN_PROCESS) {
                return 1 // ceremony
            } else {
                return 2 // created
            }
        }
    }

    const [currentStep, setCurrentStep] = useState<number>(
        calculateCurrentStep()
    )

    return (
        <StyledBox>
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
                        <ConfigureStep
                            currentCeremony={keysCeremony}
                            setCurrentCeremony={setCurrentCeremony}
                            electionEvent={electionEvent}
                            goBack={goBack}
                        />
                    }
                    {currentStep == 1 &&
                        <CeremonyStep
                            currentCeremony={keysCeremony}
                            setCurrentCeremony={setCurrentCeremony}
                            electionEvent={electionEvent}
                            goBack={goBack}
                        />
                    }
                </StyledBox>
    )
}
