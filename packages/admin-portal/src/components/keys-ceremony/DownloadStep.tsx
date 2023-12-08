// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, { useContext, useState } from "react"
import {Typography} from "@mui/material"
import ArrowForwardIosIcon from "@mui/icons-material/ArrowForwardIos"
import DownloadIcon from '@mui/icons-material/Download'
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import {Trans, useTranslation} from "react-i18next"
import { AuthContext } from "@/providers/AuthContextProvider"
import { WizardStyles } from "@/components/styles/WizardStyles"

export interface DownloadStepProps {
    goNext: () => void
    goBack: () => void
}

export const DownloadStep: React.FC<DownloadStepProps> = ({
    goNext,
    goBack,
}) => {
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const [downloaded, setDownloaded] = useState<boolean>(false)
    const download = () => {}
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
