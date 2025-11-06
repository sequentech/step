// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import {Typography} from "@mui/material"
import ArrowForwardIosIcon from "@mui/icons-material/ArrowForwardIos"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import {Trans, useTranslation} from "react-i18next"
import {AuthContext} from "@/providers/AuthContextProvider"
import {WizardStyles} from "@/components/styles/WizardStyles"

export interface ConfigureStepProps {
    goNext: () => void
    goBack: () => void
}

export const StartStep: React.FC<ConfigureStepProps> = ({goNext, goBack}) => {
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    return (
        <>
            <WizardStyles.ContentBox>
                <WizardStyles.StepHeader variant="h4">
                    {t("keysGeneration.startStep.title")}
                </WizardStyles.StepHeader>
                <WizardStyles.MainContent>
                    <Typography variant="body1">
                        <p>
                            <Trans
                                i18nKey="keysGeneration.startStep.subtitle"
                                values={{name: authContext.username}}
                            ></Trans>
                        </p>
                        <WizardStyles.OrderedList>
                            <WizardStyles.ListItem>
                                <Trans i18nKey="keysGeneration.startStep.one"></Trans>
                            </WizardStyles.ListItem>
                            <WizardStyles.ListItem>
                                <Trans i18nKey="keysGeneration.startStep.two"></Trans>
                            </WizardStyles.ListItem>
                            <WizardStyles.ListItem>
                                <Trans i18nKey="keysGeneration.startStep.three"></Trans>
                            </WizardStyles.ListItem>
                        </WizardStyles.OrderedList>
                    </Typography>
                </WizardStyles.MainContent>
            </WizardStyles.ContentBox>

            <WizardStyles.Toolbar>
                <WizardStyles.BackButton
                    color="info"
                    onClick={goBack}
                    className="keys-start-back-button"
                >
                    <ArrowBackIosIcon />
                    {t("common.label.back")}
                </WizardStyles.BackButton>
                <WizardStyles.NextButton
                    color="info"
                    onClick={goNext}
                    className="keys-start-next-button"
                >
                    <ArrowForwardIosIcon />
                    {t("common.label.next")}
                </WizardStyles.NextButton>
            </WizardStyles.Toolbar>
        </>
    )
}
