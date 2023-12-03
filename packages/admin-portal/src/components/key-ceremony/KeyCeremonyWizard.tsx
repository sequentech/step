// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {BreadCrumbSteps, BreadCrumbStepsVariant} from "@sequentech/ui-essentials"
import { Sequent_Backend_Election_Event, Sequent_Backend_Keys_Ceremony } from "@/gql/graphql"
import {styled} from "@mui/material/styles"
import { Box } from "@mui/material"
import React, { useState, useMemo } from "react"
import { Identifier, useGetList, useRecordContext } from "react-admin"
import { KeysGenerationStep } from "@/components/key-ceremony/KeysGenerationStep"
import { useTenantStore } from "@/providers/TenantContextProvider"
import { IKeyCeremonyStatusStatus } from "@/services/KeyCeremony"

const StyledBox = styled(Box)`
`

interface KeyCeremonyWizardProps {
    electionEvent: Sequent_Backend_Election_Event

    keyCeremony: Sequent_Backend_Keys_Ceremony | null
    setCurrentCeremony: (keyCeremony: Sequent_Backend_Keys_Ceremony) => void

    forceNew: boolean
    goBack: () => void
}

export const KeyCeremonyWizard: React.FC<KeyCeremonyWizardProps> = ({
    electionEvent,
    keyCeremony,
    setCurrentCeremony,
    goBack,
    forceNew,
}) => {
    const calculateCurrentStep: (forceNew: boolean) => number = (forceNew) => {
        if (forceNew || !keyCeremony) {
            return 0 // configure
        } else {
            if (keyCeremony.execution_status == IKeyCeremonyStatusStatus.NOT_STARTED) {
                return 0 // configure
            } else if (keyCeremony.execution_status == IKeyCeremonyStatusStatus.IN_PROCESS) {
                return 1 // ceremony
            } else {
                return 2 // created
            }
        }
    }

    const [currentStep, setCurrentStep] = useState<number>(
        calculateCurrentStep(forceNew)
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
                            currentCeremony={keyCeremony}
                            setCurrentCeremony={setCurrentCeremony}
                            electionEvent={electionEvent}
                            goBack={goBack}
                        />
                    }
                </StyledBox>
    )
}
