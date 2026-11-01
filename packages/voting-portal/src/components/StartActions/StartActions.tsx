// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {Box} from "@mui/material"
import Button from "@mui/material/Button"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {Link, useLocation, useNavigate, useParams} from "react-router-dom"
import {faAngleLeft, faAngleRight} from "@fortawesome/free-solid-svg-icons"
import {ESecurityConfirmationPolicy, IElection} from "@sequentech/ui-core"
import {Icon} from "@sequentech/ui-essentials"
import SecurityConfirmation from "../SecurityConfirmation/SecurityConfirmation"
import type {TenantEventType} from "../.."

const ActionsContainer = styled(Box)`
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    margin-bottom: 20px;
    margin-top: 10px;
    gap: 8px;
`

const StyledButton = styled(Button)`
    display: flex;
    padding: 5px;

    span {
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        padding: 5px;
    }
` as typeof Button

export interface StartActionsProps {
    election: IElection
    isDeclineToVotePolicyEnabled: boolean
    onDeclineToVoteClick: () => void
    // The ballot list, when the voter has one to go back to.
    backTo?: string
}

/**
 * The start screen's call to action, together with the eligibility declaration
 * that gates it when the election requires one.
 */
export const StartActions: React.FC<StartActionsProps> = ({
    election,
    isDeclineToVotePolicyEnabled,
    onDeclineToVoteClick,
    backTo,
}) => {
    const {t} = useTranslation()
    const {tenantId, eventId} = useParams<TenantEventType>()
    const location = useLocation()
    const navigate = useNavigate()
    const [checkboxChecked, setCheckboxChecked] = useState(false)

    const hasSecurityCheckbox =
        ESecurityConfirmationPolicy.MANDATORY ===
        election?.presentation?.security_confirmation_policy
    const disabledStart = hasSecurityCheckbox && !checkboxChecked

    return (
        <>
            {hasSecurityCheckbox ? (
                <SecurityConfirmation
                    election={election}
                    checked={checkboxChecked}
                    onChange={setCheckboxChecked}
                />
            ) : null}
            <ActionsContainer className="actions-container">
                {backTo !== undefined ? (
                    <StyledButton
                        component={Link}
                        to={backTo}
                        className="back-button"
                        variant="secondary"
                        sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}, flex: "none"}}
                    >
                        <Icon className="back-button-icon" icon={faAngleLeft} size="sm" />
                        <Box className="back-button-label">{t("votingScreen.backButton")}</Box>
                    </StyledButton>
                ) : null}
                <StyledButton
                    className="start-voting-button"
                    onClick={() =>
                        navigate(
                            `/tenant/${tenantId}/event/${eventId}/election/${election.id}/vote${location.search}`
                        )
                    }
                    sx={{margin: "auto 0", width: "100%"}}
                    disabled={disabledStart}
                >
                    {t("startScreen.startButton")}
                    <Icon className="start-voting-button-icon" icon={faAngleRight} size="sm" />
                </StyledButton>
                {isDeclineToVotePolicyEnabled ? (
                    <StyledButton
                        className="decline-to-vote-button"
                        sx={{width: "100%"}}
                        variant="secondary"
                        disabled={disabledStart}
                        onClick={onDeclineToVoteClick}
                    >
                        {t("startScreen.declineToVoteButton")}
                    </StyledButton>
                ) : null}
            </ActionsContainer>
        </>
    )
}

export default StartActions
