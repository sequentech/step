// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Typography} from "@mui/material"
import {
    Sequent_Backend_Election_Event,
    Sequent_Backend_Keys_Ceremony,
} from "@/gql/graphql"
import {
    Toolbar,
} from "react-admin"
import ArrowForwardIosIcon from "@mui/icons-material/ArrowForwardIos"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import Button from "@mui/material/Button"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"


const StyledToolbar = styled(Toolbar)`
    flex-direction: row;
    justify-content: space-between;
`

const BackButton = styled(Button)`
    margin-right: auto;
    background-color: ${({theme}) => theme.palette.grey[100]};
    color: ${({theme}) => theme.palette.brandColor};
`

const NextButton = styled(Button)`
    margin-left: auto;
`

export interface ConfigureStepProps {
    goNext: () => void
    goBack: () => void
}

export const StartStep: React.FC<ConfigureStepProps> = ({
    goNext,
    goBack,
}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    return (
        <>
            <Typography variant="h4">{t("keysGeneration.startStep.title")}</Typography>
            <Typography variant="body2">
                {t("keysGeneration.startStep.subtitle")}
            </Typography>

            <StyledToolbar>
                <BackButton color="info" onClick={goBack}>
                    <ArrowBackIosIcon />
                    {t("common.label.back")}
                </BackButton>
                <NextButton color="info" onClick={goNext}>
                    <ArrowForwardIosIcon />
                    {t("common.label.next")}
                </NextButton>
            </StyledToolbar>
        </>
    )
}
