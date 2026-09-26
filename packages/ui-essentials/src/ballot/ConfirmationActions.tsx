// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {faPrint} from "@fortawesome/free-solid-svg-icons"
import {Box, CircularProgress} from "@mui/material"
import {styled} from "@mui/material/styles"
import React from "react"
import {useTranslation} from "react-i18next"

import Icon from "../components/Icon/Icon"
import VisuallyHidden from "../components/VisuallyHidden/VisuallyHidden"
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
    /**
     * Whether there is a receipt to print at all. A fully acclaimed election casts
     * nothing, so the portal leaves Print out rather than disabling it.
     */
    withPrint?: boolean
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
    withPrint = true,
    onPrint,
    onFinish,
}: IConfirmationActionsProps): React.JSX.Element => {
    const {t} = useTranslation()

    return (
        <ActionsContainer className="actions-container">
            {withPrint ? (
                <>
                    <StyledButton
                        className="print-receipt-button"
                        onClick={onPrint}
                        disabled={printing || onPrint === undefined}
                        variant="secondary"
                        sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
                    >
                        {printing ? (
                            <StyledCircularProgress
                                className="print-receipt-progress"
                                color="inherit"
                                aria-hidden="true"
                            />
                        ) : (
                            <StyledIcon className="print-receipt-icon" icon={faPrint} size="sm" />
                        )}
                        <Box className="print-receipt-label">
                            {t("confirmationScreen.printButton")}
                        </Box>
                    </StyledButton>
                    {/* Generating the receipt is an asynchronous poll, so the wait
                        and its end are announced rather than shown only as a spinner. */}
                    <VisuallyHidden className="print-receipt-status" role="status">
                        {printing ? t("a11y.loading") : ""}
                    </VisuallyHidden>
                </>
            ) : null}
            <StyledButton
                className="finish-button"
                onClick={onFinish}
                disabled={onFinish === undefined}
                sx={{width: {xs: "100%", sm: "200px"}}}
            >
                <Box className="finish-button-label">{t("confirmationScreen.finishButton")}</Box>
            </StyledButton>
        </ActionsContainer>
    )
}
