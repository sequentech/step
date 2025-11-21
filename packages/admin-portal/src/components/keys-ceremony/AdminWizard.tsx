// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {BreadCrumbSteps, BreadCrumbStepsVariant} from "@sequentech/ui-essentials"
import {IKeysCeremonyExecutionStatus as EStatus} from "@/services/KeyCeremony"
import {Sequent_Backend_Election_Event, Sequent_Backend_Keys_Ceremony} from "@/gql/graphql"
import React, {useCallback, useEffect, useMemo, useState} from "react"
import {ConfigureStep} from "@/components/keys-ceremony/ConfigureStep"
import {CeremonyStep} from "@/components/keys-ceremony/CeremonyStep"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {EElectionEventCeremoniesPolicy} from "@sequentech/ui-core"
import {CircularProgress} from "@mui/material"

interface AdminWizardProps {
    electionEvent?: Sequent_Backend_Election_Event
    currentCeremony: Sequent_Backend_Keys_Ceremony | null
    setCurrentCeremony: (keysCeremony: Sequent_Backend_Keys_Ceremony) => void

    goBack: () => void
}

export const AdminWizard: React.FC<AdminWizardProps> = ({
    electionEvent,
    currentCeremony,
    setCurrentCeremony,
    goBack,
}) => {
    const calculateCurrentStep: () => number = useCallback(() => {
        if (!currentCeremony) {
            return 0 // configure
        } else {
            if (
                currentCeremony.execution_status === EStatus.STARTED ||
                currentCeremony.execution_status === EStatus.IN_PROGRESS
            ) {
                return 1 // ceremony, created
            } else {
                return 2 // final state
            }
        }
    }, [currentCeremony?.execution_status, currentCeremony?.settings])

    const [currentStep, setCurrentStep] = useState<number>(calculateCurrentStep())

    const openCeremonyStep = () => {
        setCurrentStep(1)
    }

    const isAutomaticPolicy =
        currentCeremony?.settings?.policy === EElectionEventCeremoniesPolicy.AUTOMATED_CEREMONIES

    useEffect(() => {
        if (isAutomaticPolicy) {
            setCurrentStep(calculateCurrentStep())
        }
    }, [currentCeremony?.execution_status])

    if (!electionEvent) {
        return <CircularProgress />
    }
    return (
        <WizardStyles.WizardWrapper>
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
            {currentStep === 0 && (
                <ConfigureStep
                    currentCeremony={currentCeremony}
                    setCurrentCeremony={setCurrentCeremony}
                    electionEvent={electionEvent}
                    openCeremonyStep={openCeremonyStep}
                    goBack={goBack}
                />
            )}
            {currentStep > 0 && (
                <CeremonyStep
                    currentCeremonyId={currentCeremony?.id}
                    electionEvent={electionEvent}
                    goBack={goBack}
                    setCurrentCeremony={isAutomaticPolicy ? setCurrentCeremony : undefined}
                />
            )}
        </WizardStyles.WizardWrapper>
    )
}
