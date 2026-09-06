// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {Box} from "@mui/material"
import Button from "@mui/material/Button"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {Link as RouterLink, useLocation, useParams} from "react-router-dom"
import {ESecurityConfirmationPolicy, IElection} from "@sequentech/ui-core"
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
}

/**
 * The start screen's call to action, together with the eligibility declaration
 * that gates it when the election requires one.
 */
export const StartActions: React.FC<StartActionsProps> = ({
    election,
    isDeclineToVotePolicyEnabled,
    onDeclineToVoteClick,
}) => {
    const {t} = useTranslation()
    const {tenantId, eventId} = useParams<TenantEventType>()
    const location = useLocation()
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
            <ActionsContainer>
                {disabledStart ? (
                    <StyledButton
                        className="start-voting-button"
                        sx={{width: "100%"}}
                        disabled={true}
                    >
                        {t("startScreen.startButton")}
                    </StyledButton>
                ) : (
                    <StyledButton
                        component={RouterLink}
                        className="start-voting-button"
                        to={`/tenant/${tenantId}/event/${eventId}/election/${election.id}/vote${location.search}`}
                        sx={{margin: "auto 0", width: "100%"}}
                    >
                        {t("startScreen.startButton")}
                    </StyledButton>
                )}
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
