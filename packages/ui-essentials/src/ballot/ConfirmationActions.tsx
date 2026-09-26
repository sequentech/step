// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {faPrint} from "@fortawesome/free-solid-svg-icons"
import {Box, CircularProgress} from "@mui/material"
import {styled} from "@mui/material/styles"
import React from "react"
import {useTranslation} from "react-i18next"

import Icon from "../components/Icon/Icon"
import {ActionsContainer, StyledButton} from "../components/ActionsRow/ActionsRow"

/** The printer, at the size the portal draws it. */
const StyledIcon = styled(Icon)`
    min-width: 14px;
    padding: 5px;
`

const StyledCircularProgress = styled(CircularProgress)`
    width: 14px !important;
    height: 14px !important;
`

export interface IConfirmationActionsProps {
    /** The receipt is being made: the printer waits with a spinner. */
    printing?: boolean
    onPrint?: () => void
    onFinish?: () => void
}

/**
 * The row under the confirmation screen: *Print* and *Finish*.
 *
 * Lifted out of the portal's `ConfirmationScreen`. Print is `variant="secondary"` with a
 * printer icon; Finish carries `finish-button`, which a client's stylesheet targets. The
 * Election Architect drew two plain buttons of its own — same words, neither shape.
 *
 * What Print *does* stays with the caller: in the portal it renders a receipt from a cast
 * vote, and a preview has no vote to render. Given no `onPrint` it is drawn disabled,
 * which is the honest way to show a button whose page does not exist yet.
 */
export const ConfirmationActions = ({
    printing = false,
    onPrint,
    onFinish,
}: IConfirmationActionsProps): React.JSX.Element => {
    const {t} = useTranslation()

    return (
        <ActionsContainer>
            <StyledButton
                onClick={onPrint}
                disabled={printing || onPrint === undefined}
                variant="secondary"
                sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
            >
                {printing ? (
                    <StyledCircularProgress color="inherit" />
                ) : (
                    <StyledIcon icon={faPrint} size="sm" />
                )}
                <Box>{t("confirmationScreen.printButton")}</Box>
            </StyledButton>
            <StyledButton
                className="finish-button"
                onClick={onFinish}
                disabled={onFinish === undefined}
                sx={{width: {xs: "100%", sm: "200px"}}}
            >
                <Box>{t("confirmationScreen.finishButton")}</Box>
            </StyledButton>
        </ActionsContainer>
    )
}
