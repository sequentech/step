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
import {Alert, CircularProgress} from "@mui/material"
import React, {useContext, useState, useEffect} from "react"
import {StartStep} from "@/components/keys-ceremony/StartStep"
import {CeremonyStep} from "@/components/keys-ceremony/CeremonyStep"
import {useTranslation} from "react-i18next"
import {DownloadStep} from "./DownloadStep"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {CheckStep} from "./CheckStep"
import {EElectionEventCeremoniesPolicy} from "@sequentech/ui-core"

export const isTrusteeParticipating = (
    ceremony: Sequent_Backend_Keys_Ceremony,
    authContext: AuthContextValues
) => {
    const status: IExecutionStatus = ceremony.status
    return (
        (ceremony.execution_status === EStatus.USER_CONFIGURATION ||
            ceremony.execution_status === EStatus.IN_PROGRESS) &&
        !!status.trustees.find((trustee) => trustee.name === authContext.trustee)
    )
}

const hasTrusteeCheckedKeys = (
    ceremony: Sequent_Backend_Keys_Ceremony,
    authContext: AuthContextValues
) => {
    const status: IExecutionStatus = ceremony.status
    return status.trustees.find(
        (trustee) => trustee.name === authContext.trustee && trustee.status === TStatus.KEY_CHECKED
    )
}

interface TrusteeWizardProps {
    electionEvent?: Sequent_Backend_Election_Event
    currentCeremony: Sequent_Backend_Keys_Ceremony
    setCurrentCeremony?: (keysCeremony: Sequent_Backend_Keys_Ceremony) => void
    goBack: () => void
}

enum WizardStep {
    Not_Generated = -1,
    Start = 0,
    Download = 1,
    Check = 2,
    Success = 3,
    Status = 4,
}

export const TrusteeWizard: React.FC<TrusteeWizardProps> = ({
    electionEvent,
    currentCeremony,
    setCurrentCeremony,
    goBack,
}) => {
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const trusteeParticipating =
        currentCeremony && isTrusteeParticipating(currentCeremony, authContext)
    const trusteeCheckedKeys = hasTrusteeCheckedKeys(currentCeremony, authContext)
    const status: IExecutionStatus = currentCeremony.status
    const keysGenerated =
        status.public_key !== undefined &&
        currentCeremony.execution_status === EStatus.IN_PROGRESS &&
        !status.trustees.find((trustee) => trustee.status === TStatus.WAITING)

    const calculateCurrentStep: () => WizardStep = () => {
        // If trustee is not participating, show status step
        if (!trusteeParticipating) {
            return WizardStep.Status
            // If trustee is participating but is not started, show status step
        } else if (currentCeremony.execution_status === EStatus.USER_CONFIGURATION) {
            return WizardStep.Status
            // If trustee is participating but is not started, show status step
        } else if (
            currentCeremony.execution_status === EStatus.CANCELLED ||
            currentCeremony.execution_status === EStatus.SUCCESS
        ) {
            return WizardStep.Success
            // if the trustee has not checked the key, then show the start screen
        } else if (
            currentCeremony.execution_status === EStatus.IN_PROGRESS &&
            !trusteeCheckedKeys
        ) {
            return WizardStep.Start
            // In all other cases, just show the status
        } else {
            return WizardStep.Success
        }
    }
    const [currentStep, setCurrentStep] = useState<WizardStep>(calculateCurrentStep())

    useEffect(() => {
        if (!trusteeCheckedKeys && trusteeParticipating && keysGenerated) {
            setCurrentStep(WizardStep.Start)
        } else if (!keysGenerated) {
            setCurrentStep(WizardStep.Not_Generated)
        } else {
            setCurrentStep(WizardStep.Status)
        }
    }, [trusteeCheckedKeys, trusteeParticipating, keysGenerated])

    const checkKeysGenerated = () => {
        return !trusteeCheckedKeys && trusteeParticipating && !keysGenerated
    }

    if (!electionEvent) {
        return <CircularProgress />
    }

    const isAutomaticCeremony =
        electionEvent.presentation?.ceremonies_policy ===
            EElectionEventCeremoniesPolicy.AUTOMATED_CEREMONIES &&
        currentCeremony?.settings?.policy === EElectionEventCeremoniesPolicy.AUTOMATED_CEREMONIES

    return (
        <WizardStyles.WizardWrapper>
            <BreadCrumbSteps
                labels={
                    trusteeParticipating
                        ? [
                              "electionEventScreen.keys.breadCrumbs.start",
                              "electionEventScreen.keys.breadCrumbs.download",
                              "electionEventScreen.keys.breadCrumbs.check",
                              "electionEventScreen.keys.breadCrumbs.success",
                          ]
                        : ["electionEventScreen.keys.breadCrumbs.status"]
                }
                selected={currentStep}
                variant={BreadCrumbStepsVariant.Circle}
                colorPreviousSteps={true}
            />

            {currentStep === WizardStep.Start && (
                <StartStep goNext={() => setCurrentStep(WizardStep.Download)} goBack={goBack} />
            )}
            {currentStep === WizardStep.Download && (
                <DownloadStep
                    currentCeremony={currentCeremony}
                    electionEvent={electionEvent}
                    goBack={() => setCurrentStep(WizardStep.Start)}
                    goNext={() => setCurrentStep(WizardStep.Check)}
                />
            )}
            {currentStep === WizardStep.Check && (
                <CheckStep
                    currentCeremony={currentCeremony}
                    electionEvent={electionEvent}
                    goBack={() => setCurrentStep(WizardStep.Download)}
                    goNext={() => setCurrentStep(WizardStep.Success)}
                />
            )}
            {currentStep === WizardStep.Success && (
                <CeremonyStep
                    currentCeremonyId={currentCeremony?.id}
                    electionEvent={electionEvent}
                    goBack={goBack}
                />
            )}
            {(currentStep === WizardStep.Status || currentStep === WizardStep.Not_Generated) && (
                <CeremonyStep
                    currentCeremonyId={currentCeremony?.id}
                    setCurrentCeremony={setCurrentCeremony}
                    electionEvent={electionEvent}
                    goBack={goBack}
                    goNext={
                        currentStep === WizardStep.Not_Generated
                            ? () => setCurrentStep(WizardStep.Start)
                            : undefined
                    }
                    isNextDisabled={checkKeysGenerated() || isAutomaticCeremony}
                    message={
                        checkKeysGenerated() ? (
                            <>
                                <Alert severity="warning">
                                    {t("electionEventScreen.keys.waitingKeys")}
                                </Alert>
                            </>
                        ) : undefined
                    }
                />
            )}
        </WizardStyles.WizardWrapper>
    )
}
