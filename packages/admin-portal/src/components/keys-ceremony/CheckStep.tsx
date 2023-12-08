// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useMutation} from "@apollo/client"
import React, { useContext, useState } from "react"
import {Typography} from "@mui/material"
import ArrowForwardIosIcon from "@mui/icons-material/ArrowForwardIos"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import {Trans, useTranslation} from "react-i18next"

import {
    CheckPrivateKeyMutation,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Keys_Ceremony,
} from "@/gql/graphql"
import { AuthContext } from "@/providers/AuthContextProvider"
import { WizardStyles } from "@/components/styles/WizardStyles"
import { CHECK_PRIVATE_KEY } from "@/queries/CheckPrivateKey"
import {DropFile} from "@sequentech/ui-essentials"

export interface DownloadStepProps {
    electionEvent: Sequent_Backend_Election_Event
    currentCeremony: Sequent_Backend_Keys_Ceremony
    goNext: () => void
    goBack: () => void
}

export const CheckStep: React.FC<DownloadStepProps> = ({
    electionEvent,
    currentCeremony,
    goNext,
    goBack,
}) => {
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const [verified, setVerified] = useState<boolean>(false)
    const [uploading, setUploading] = useState<boolean>(false)
    const [errors, setErrors] = useState<String | null>(null)

    const [checkPrivateKeysMutation] =
    useMutation<CheckPrivateKeyMutation>(CHECK_PRIVATE_KEY)
    const uploadPrivateKey = async (files: FileList | null) => {
        const fileContent = files?.[0]?.text()
        console.log(`uploadPrivateKey(): fileContent: ${fileContent}`)
        setErrors(null)
        setVerified(false)
        setUploading(true)
        try {
            const {data, errors} = await checkPrivateKeysMutation({
                variables: {
                    electionEventId: electionEvent.id,
                    keysCeremonyId: currentCeremony.id,
                    privateKeyBase64: fileContent
                },
            })
            setUploading(false)
            if (errors) {
                setErrors(t(
                    "keysGeneration.checkStep.errorUploading",
                    {error: errors.toString()}
                ))
                return
            } else {
                const isValid = data?.check_private_key?.is_valid
                if (!isValid) {
                    setErrors(t(
                        "keysGeneration.checkStep.errorUploading",
                        {error: "empty"}
                    ))
                    return
                }
                setVerified(true)
            }
        } catch (exception: any) {
            setUploading(false)
            setErrors(t(
                "keysGeneration.checkStep.errorUploading",
                {error: exception.toString()}
            ))
        }
    }
    return (
        <>
            <WizardStyles.ContentBox>
                <WizardStyles.StepHeader variant="h4">
                    {t("keysGeneration.checkStep.title")}
                </WizardStyles.StepHeader>
                <WizardStyles.MainContent>
                    <Typography variant="body1">
                        <Trans
                            i18nKey="keysGeneration.checkStep.subtitle"
                            values={{name: authContext.username}}
                        ></Trans>
                    </Typography>

                    <DropFile
                        handleFiles={uploadPrivateKey}
                    />
                    <WizardStyles.StatusBox>
                        {uploading ? <WizardStyles.DownloadProgress /> : null}
                        {errors
                            ? <WizardStyles.ErrorMessage variant="body2">
                                {errors}
                            </WizardStyles.ErrorMessage>
                            : null}
                    </WizardStyles.StatusBox>
                </WizardStyles.MainContent>
            </WizardStyles.ContentBox>

            <WizardStyles.Toolbar>
                <WizardStyles.BackButton color="info" onClick={goBack}>
                    <ArrowBackIosIcon />
                    {t("common.label.back")}
                </WizardStyles.BackButton>
                <WizardStyles.NextButton
                    disabled={!verified}
                    color="info"
                    onClick={goNext}
                >
                    <ArrowForwardIosIcon />
                    {t("common.label.next")}
                </WizardStyles.NextButton>
            </WizardStyles.Toolbar>
        </>
    )
}
