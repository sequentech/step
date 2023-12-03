// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {BreadCrumbSteps, BreadCrumbStepsVariant} from "@sequentech/ui-essentials"
import { Sequent_Backend_Election_Event, Sequent_Backend_Keys_Ceremony } from "@/gql/graphql"
import {styled} from "@mui/material/styles"
import { Box } from "@mui/material"
import React, { useState } from "react"
import { KeysGenerationStep } from "@/components/keys-ceremony/KeysGenerationStep"
import { IKeyCeremonyStatusStatus } from "@/services/KeyCeremony"

const StyledBox = styled(Box)`
`

interface KeysCeremonyWizardProps {
    electionEvent: Sequent_Backend_Election_Event

    keysCeremony: Sequent_Backend_Keys_Ceremony | null
    setCurrentCeremony: (keysCeremony: Sequent_Backend_Keys_Ceremony) => void

    goBack: () => void
}

export const KeysCeremonyWizard: React.FC<KeysCeremonyWizardProps> = ({
    electionEvent,
    keysCeremony,
    setCurrentCeremony,
    goBack,
}) => {
    const calculateCurrentStep: () => number = () => {
        if (!keysCeremony) {
            return 0 // configure
        } else {
            if (keysCeremony.execution_status == IKeyCeremonyStatusStatus.NOT_STARTED) {
                return 0 // configure
            } else if (keysCeremony.execution_status == IKeyCeremonyStatusStatus.IN_PROCESS) {
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
                        <KeysGenerationStep
                            currentCeremony={keysCeremony}
                            setCurrentCeremony={setCurrentCeremony}
                            electionEvent={electionEvent}
                            goBack={goBack}
                        />
                    }
                </StyledBox>
    )
}
