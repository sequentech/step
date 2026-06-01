// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
import {useHeadlessTrustee} from "@/hooks/useHeadlessTrustee"
import {HeadlessTrusteeContext} from "@/providers/HeadlessTrusteeProvider"

const HeadlessTrusteeRunner: React.FC<{currentCeremony: Sequent_Backend_Keys_Ceremony}> = ({
    currentCeremony,
}) => {
    useHeadlessTrustee({currentCeremony})
    return null
}

/**
 * Returns true when the currently logged-in trustee user is expected to act
 * in the given ceremony.
 *
 * Both conditions must hold:
 *  1. The ceremony is in an actionable phase: USER_CONFIGURATION (trustees
 *     still being invited) or IN_PROGRESS (braid protocol actively running).
 *     SUCCESS, CANCELLED and STARTED are intentionally excluded.
 *  2. The logged-in user's trustee name (from JWT claims) appears in the
 *     ceremony's trustee list — i.e. they were explicitly assigned to it.
 *
 * Used as a gate throughout the wizard to decide whether to show the trustee
 * flow and, in automatic ceremonies, whether to start the headless WASM
 * trustee protocol runner.
 */
export const isTrusteeActionablePhase = (
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
    trusteeNames?: Array<{id?: string; name?: string | null; annotations?: any}>
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
    trusteeNames,
}) => {
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    // Log trusteeParticipating condition for debugging
    console.info(
        `[TrusteeWizard] Checking trustee participation: currentCeremony.execution_status=${currentCeremony.execution_status}, authContext.trustee=${authContext.trustee}, isParticipating=${isTrusteeActionablePhase(currentCeremony, authContext)}`
    )
    const trusteeIsInActionablePhase =
        currentCeremony && isTrusteeActionablePhase(currentCeremony, authContext)
    const trusteeCheckedKeys = hasTrusteeCheckedKeys(currentCeremony, authContext)
    const status: IExecutionStatus = currentCeremony.status
    const keysGenerated =
        status.public_key !== undefined &&
        currentCeremony.execution_status === EStatus.IN_PROGRESS &&
        !status.trustees.find((trustee) => trustee.status === TStatus.WAITING)

    const calculateCurrentStep: () => WizardStep = () => {
        // If trustee is not participating, show status step
        if (!trusteeIsInActionablePhase) {
            return WizardStep.Status
            // If trustee is participating but is not started, show status step
        } else if (currentCeremony.execution_status === EStatus.USER_CONFIGURATION) {
            return WizardStep.Status
            // If trustee is participating but cancelled or succeeded, show success step (with status)
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
        if (!trusteeCheckedKeys && trusteeIsInActionablePhase && keysGenerated) {
            setCurrentStep(WizardStep.Start)
        } else if (!keysGenerated) {
            setCurrentStep(WizardStep.Not_Generated)
        } else {
            setCurrentStep(WizardStep.Status)
        }
    }, [trusteeCheckedKeys, trusteeIsInActionablePhase, keysGenerated])

    const isWaitingForKeyGeneration = () => {
        return !trusteeCheckedKeys && trusteeIsInActionablePhase && !keysGenerated
    }

    // Computed before the early return so hooks below are always called unconditionally
    const isAutomaticCeremony =
        electionEvent?.presentation?.ceremonies_policy ===
            EElectionEventCeremoniesPolicy.AUTOMATED_CEREMONIES &&
        currentCeremony?.settings?.policy === EElectionEventCeremoniesPolicy.AUTOMATED_CEREMONIES

    const {isConnected} = useContext(HeadlessTrusteeContext)

    if (!electionEvent) {
        return <CircularProgress />
    }
    return (
        <WizardStyles.WizardWrapper>
            {/* Silently run the braid protocol — session provided by HeadlessTrusteeProvider */}
            {!!trusteeIsInActionablePhase && !isAutomaticCeremony && isConnected && (
                <HeadlessTrusteeRunner currentCeremony={currentCeremony} />
            )}
            <BreadCrumbSteps
                labels={
                    trusteeIsInActionablePhase
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
                    trusteeNames={trusteeNames}
                />
            )}
            {(currentStep === WizardStep.Status || currentStep === WizardStep.Not_Generated) && (
                <CeremonyStep
                    currentCeremonyId={currentCeremony?.id}
                    setCurrentCeremony={setCurrentCeremony}
                    electionEvent={electionEvent}
                    goBack={goBack}
                    trusteeNames={trusteeNames}
                    goNext={
                        currentStep === WizardStep.Not_Generated
                            ? () => setCurrentStep(WizardStep.Start)
                            : undefined
                    }
                    isNextDisabled={
                        isWaitingForKeyGeneration() ||
                        isAutomaticCeremony ||
                        currentCeremony.execution_status === EStatus.SUCCESS ||
                        currentCeremony.execution_status === EStatus.CANCELLED
                    }
                    message={
                        isWaitingForKeyGeneration() ? (
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
