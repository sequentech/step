// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {BreadCrumbSteps, BreadCrumbStepsVariant} from "@sequentech/ui-essentials"
import {
    Sequent_Backend_Election_Event,
    Sequent_Backend_Keys_Ceremony
} from "@/gql/graphql"
import {styled} from "@mui/material/styles"
import { Box } from "@mui/material"
import React, { useState } from "react"
import { ConfigureStep } from "@/components/keys-ceremony/ConfigureStep"
import { CeremonyStep } from "@/components/keys-ceremony/CeremonyStep"
import { IKeysCeremonyExecutionStatus as EStatus } from "@/services/KeyCeremony"

const StyledBox = styled(Box)`
`

interface WizardProps {
    electionEvent: Sequent_Backend_Election_Event

    currentCeremony: Sequent_Backend_Keys_Ceremony | null
    setCurrentCeremony: (keysCeremony: Sequent_Backend_Keys_Ceremony) => void

    goBack: () => void
}

export const Wizard: React.FC<WizardProps> = ({
    electionEvent,
    currentCeremony,
    setCurrentCeremony,
    goBack,
}) => {
    const calculateCurrentStep: () => number = () => {
        if (!currentCeremony) {
            return 0 // configure
        } else {
            if (
                currentCeremony.execution_status == EStatus.NOT_STARTED ||
                currentCeremony.execution_status == EStatus.IN_PROCESS
            ) {
                return 1 // ceremony, created
            } else {
                return 2 // final state
            }
        }
    }

    const [currentStep, setCurrentStep] = useState<number>(
        calculateCurrentStep()
    )
    const openCeremonyStep = () => { setCurrentStep(1) }

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
                    currentCeremony={currentCeremony}
                    setCurrentCeremony={setCurrentCeremony}
                    electionEvent={electionEvent}
                    openCeremonyStep={openCeremonyStep}
                    goBack={goBack}
                />
            }
            {currentStep > 0 &&
                <CeremonyStep
                    currentCeremony={currentCeremony}
                    setCurrentCeremony={setCurrentCeremony}
                    electionEvent={electionEvent}
                    goBack={goBack}
                />
            }
        </StyledBox>
    )
}
