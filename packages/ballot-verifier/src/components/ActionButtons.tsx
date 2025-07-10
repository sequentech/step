// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import Box from "@mui/material/Box"
import {Link as RouterLink} from "react-router-dom"
import {useTranslation} from "react-i18next"
import {styled} from "@mui/material/styles"
import Button from "@mui/material/Button"
import {faPrint, faAngleLeft} from "@fortawesome/free-solid-svg-icons"
import {Icon} from "@sequentech/ui-essentials"

const StyledLink = styled(RouterLink)`
    margin: auto 0;
    text-decoration: none;
`

const ActionsContainer = styled(Box)`
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    margin-bottom: 20px;
    margin-top: 10px;
    gap: 2px;
`

const StyledButton = styled(Button)`
    display flex;
    padding: 5px;

    span {
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        padding: 5px;
    }
`

interface VoteChoiceProps {
    text?: string
    points: number | null
    ordered: boolean
}

interface ActionButtonProps {}

export const ActionButtons: React.FC<ActionButtonProps> = ({}) => {
    const {t} = useTranslation()
    const triggerPrint = () => window.print()

    return (
        <ActionsContainer>
            <StyledLink to="/" sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}>
                <StyledButton sx={{width: {xs: "100%", sm: "200px"}}}>
                    <Icon icon={faAngleLeft} size="sm" />
                    <span>{t("confirmationScreen.backButton")}</span>
                </StyledButton>
            </StyledLink>
            <StyledButton
                onClick={triggerPrint}
                variant="secondary"
                sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
            >
                <Icon icon={faPrint} size="sm" />
                <Box>{t("confirmationScreen.printButton")}</Box>
            </StyledButton>
            {/*<StyledButton sx={{width: {xs: "100%", sm: "200px"}}}>
                <span>{t("confirmationScreen.finishButton")}</span>
                <Icon icon={faAngleRight} size="sm" />
            </StyledButton>*/}
        </ActionsContainer>
    )
}
