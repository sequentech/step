// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useMutation} from "@apollo/client"
import React, { useContext, useState } from "react"
import {Typography} from "@mui/material"
import ArrowForwardIosIcon from "@mui/icons-material/ArrowForwardIos"
import DownloadIcon from '@mui/icons-material/Download'
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import {Trans, useTranslation} from "react-i18next"

import {
    GetPrivateKeyMutation,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Keys_Ceremony,
} from "@/gql/graphql"
import { AuthContext } from "@/providers/AuthContextProvider"
import { WizardStyles } from "@/components/styles/WizardStyles"
import { GET_PRIVATE_KEY } from "@/queries/GetPrivateKey"

export interface DownloadStepProps {
    electionEvent: Sequent_Backend_Election_Event
    currentCeremony: Sequent_Backend_Keys_Ceremony
    goNext: () => void
    goBack: () => void
}

export const DownloadStep: React.FC<DownloadStepProps> = ({
    electionEvent,
    currentCeremony,
    goNext,
    goBack,
}) => {
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const [downloaded, setDownloaded] = useState<boolean>(false)
    const [downloading, setDownloading] = useState<boolean>(false)
    const [errors, setErrors] = useState<String | null>(null)

    const [getPrivateKeysMutation] =
    useMutation<GetPrivateKeyMutation>(GET_PRIVATE_KEY)
    const download = async () => {
        setErrors(null)
        setDownloading(true)
        try {
            const {data, errors} = await getPrivateKeysMutation({
                variables: {
                    electionEventId: electionEvent.id,
                    keysCeremonyId: currentCeremony.id,
                },
            })
            setDownloading(false)
            if (errors) {
                setDownloaded(false)
                setErrors(t(
                    "keysGeneration.downloadStep.errorDownloading",
                    {error: errors.toString()}
                ))
                return null
            } else {
                const privateKey = "whatever" //data?.get_private_key?.private_key_base64
                if (!privateKey) {
                    setErrors(t("keysGeneration.downloadStep.errorEmptyKey"))
                    return
                }
                const blob = new Blob(
                    [privateKey], {type: 'application/octet-stream'}
                )
                const blobUrl = window.URL.createObjectURL(blob)
                const fileName = `encrypted_private_key_trustee_${authContext.username}_${currentCeremony.id}.bin`
                var tempLink = document.createElement('a')
                tempLink.href = blobUrl
                tempLink.setAttribute('download', fileName)
                tempLink.click()
                setDownloaded(true)
            }
        } catch (exception: any) {
            setDownloading(false)
            setDownloaded(false)
            setErrors(t(
                "keysGeneration.downloadStep.errorDownloading",
                {error: exception.toString()}
            ))
            return null
        }
    }
    return (
        <>
            <WizardStyles.ContentBox>
                <WizardStyles.StepHeader variant="h4">
                    {t("keysGeneration.downloadStep.title")}
                </WizardStyles.StepHeader>
                <WizardStyles.MainContent>
                    <Typography variant="body1">
                        <Trans
                            i18nKey="keysGeneration.downloadStep.subtitle"
                            values={{name: authContext.username}}
                        ></Trans>
                    </Typography>
                    <WizardStyles.DownloadButton
                        color="primary"
                        onClick={download}
                    >
                        <DownloadIcon />
                        {t("keysGeneration.downloadStep.downloadButton")}
                    </WizardStyles.DownloadButton>
                    <WizardStyles.StatusBox>
                        {downloading ? <WizardStyles.DownloadProgress /> : null}
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
                    disabled={!downloaded}
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
