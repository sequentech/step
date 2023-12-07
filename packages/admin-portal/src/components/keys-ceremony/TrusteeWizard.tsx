// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {BreadCrumbSteps, BreadCrumbStepsVariant} from "@sequentech/ui-essentials"
import {AuthContext, AuthContextValues} from "@/providers/AuthContextProvider"
import {
    IKeysCeremonyExecutionStatus as EStatus,
    IKeysCeremonyTrusteeStatus as TStatus,
    IExecutionStatus,
} from "@/services/KeyCeremony"
import {Sequent_Backend_Election_Event, Sequent_Backend_Keys_Ceremony} from "@/gql/graphql"
import {styled} from "@mui/material/styles"
import {Box} from "@mui/material"
import React, {useContext, useState} from "react"
import {StartStep} from "@/components/keys-ceremony/StartStep"
import {CeremonyStep} from "@/components/keys-ceremony/CeremonyStep"

export const isTrusteeParticipating =
    (
        ceremony: Sequent_Backend_Keys_Ceremony,
        authContext: AuthContextValues
    ) => {
    const status: IExecutionStatus = ceremony.status
    return (
        (
            ceremony.execution_status == EStatus.NOT_STARTED ||
            ceremony.execution_status == EStatus.IN_PROCESS
        ) &&
        !!status
            .trustees
            .find((trustee) => trustee.name == authContext.username)
    )
}

const hasTrusteeCheckedKeys = 
(
    ceremony: Sequent_Backend_Keys_Ceremony,
    authContext: AuthContextValues
) => {
    const status: IExecutionStatus = ceremony.status
    return status
            .trustees
            .find((trustee) => (
                trustee.name == authContext.username &&
                trustee.status == TStatus.KEY_CHECKED
            ))
}

const StyledBox = styled(Box)``

interface TrusteeWizardProps {
    electionEvent: Sequent_Backend_Election_Event
    currentCeremony: Sequent_Backend_Keys_Ceremony
    setCurrentCeremony: (keysCeremony: Sequent_Backend_Keys_Ceremony) => void

    goBack: () => void
}

enum WizardStep {
    Status = 0,
    Start = 1,
    Success = 2,
}

export const TrusteeWizard: React.FC<TrusteeWizardProps> = ({
    electionEvent,
    currentCeremony,
    setCurrentCeremony,
    goBack,
}) => {
    const authContext = useContext(AuthContext)
    const trusteeParticipating = (
        currentCeremony &&
        isTrusteeParticipating(currentCeremony, authContext)
    )
    const trusteeCheckedKeys = hasTrusteeCheckedKeys(
        currentCeremony,
        authContext
    )
    const calculateCurrentStep: () => WizardStep = () => {
        // If trustee is not participating, it can only status view
        if (!trusteeParticipating) {
            return WizardStep.Status
        // If trustee is participating but is not in process, then it can only
        // view the ceremony
        } else if (currentCeremony.execution_status != EStatus.IN_PROCESS) {
            return WizardStep.Status
        // if the trustee has not checked the key, then show the start screen
        } else if (!trusteeCheckedKeys) {
            return WizardStep.Start
        // In all other cases, just show the status
        } else {
            return WizardStep.Success
        }
    }
    const showStatusNext = trusteeCheckedKeys && trusteeParticipating
    const [currentStep, setCurrentStep] = useState<WizardStep>(calculateCurrentStep())
    const openCeremonyStep = () => {
        setCurrentStep(1)
    }

    return (
        <StyledBox>
            <BreadCrumbSteps
                labels={showStatusNext
                    ? [
                        "electionEventScreen.keys.breadCrumbs.status",
                        "electionEventScreen.keys.breadCrumbs.start",
                        "electionEventScreen.keys.breadCrumbs.download",
                        "electionEventScreen.keys.breadCrumbs.check",
                        "electionEventScreen.keys.breadCrumbs.success",
                    ]
                    : [
                        "electionEventScreen.keys.breadCrumbs.status",
                    ]}
                selected={currentStep}
                variant={BreadCrumbStepsVariant.Circle}
                colorPreviousSteps={true}
            />
            {(
                currentStep == WizardStep.Status ||
                currentStep == WizardStep.Success
            ) && (
                <CeremonyStep
                    currentCeremony={currentCeremony}
                    electionEvent={electionEvent}
                    goBack={goBack}
                    goNext={showStatusNext && currentStep == WizardStep.Status
                        ? () => setCurrentStep(WizardStep.Start)
                        : undefined}
                />
            )}
            {currentStep == WizardStep.Start && (
                <StartStep
                    currentCeremony={currentCeremony}
                    setCurrentCeremony={setCurrentCeremony}
                    electionEvent={electionEvent}
                    openCeremonyStep={openCeremonyStep}
                    goBack={() => setCurrentStep(WizardStep.Status)}
                />
            )}
        </StyledBox>
    )
}
