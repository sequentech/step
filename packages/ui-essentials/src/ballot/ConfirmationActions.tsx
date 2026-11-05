// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, CircularProgress} from "@mui/material"
import {styled} from "@mui/material/styles"
import React from "react"
import {useTranslation} from "react-i18next"

import VisuallyHidden from "../components/VisuallyHidden/VisuallyHidden"
import {ActionsContainer, StyledButton} from "../components/ActionsRow/ActionsRow"
import {theme} from "../services/theme"

const PHONE = theme.breakpoints.down("sm")

const StyledCircularProgress = styled(CircularProgress)`
    width: 16px !important;
    height: 16px !important;
`

/** A download arrow: Save receipt (PDF) downloads a file. */
const DownloadIcon: React.FC = () => (
    <svg
        className="print-receipt-icon"
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
        focusable="false"
        style={{flex: "none"}}
    >
        <path className="print-receipt-icon-path" d="M12 4v11" />
        <path className="print-receipt-icon-path" d="M7 10.5l5 5 5-5" />
        <path className="print-receipt-icon-path" d="M5 19.5h14" />
    </svg>
)

const MailIcon: React.FC = () => (
    <svg
        className="cast-email-note-icon"
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
        focusable="false"
        style={{flex: "none"}}
    >
        <rect className="cast-email-note-icon-path" x="3" y="5" width="18" height="14" rx="2" />
        <path className="cast-email-note-icon-path" d="M3.5 6.5l8.5 6.5 8.5-6.5" />
    </svg>
)

export interface IConfirmationActionsProps {
    /** The receipt is being made: the button waits with a spinner. */
    printing?: boolean
    /**
     * Whether there is a receipt to save at all. A fully acclaimed election casts
     * nothing, so the portal leaves Save receipt out rather than disabling it.
     */
    withPrint?: boolean
    /** Saves the receipt: the portal downloads its PDF. */
    onPrint?: () => void
    onFinish?: () => void
    /**
     * Says that a copy of the receipt was emailed too. Only for a caller that knows
     * this election sends the cast email; the portal does not, and leaves it out.
     */
    castEmailSent?: boolean
}

/**
 * The row under *Keep your receipt*: *Save receipt (PDF)*, the cast email line when
 * there is one, and *Finish* at the right end (design v42).
 *
 * Lifted out of the portal's `ConfirmationScreen`. Save receipt is the outlined
 * `variant="secondary"` with a download arrow; Finish is the only filled button and
 * carries `finish-button`, which a client's stylesheet targets. Neither has a fixed
 * width, so a long translation wraps; on a phone both take the full width.
 *
 * What Save receipt *does* stays with the caller: in the portal it renders a receipt
 * from a cast vote, and a preview has no vote to render. Given no `onPrint` it is
 * drawn disabled, which is the honest way to show a button whose page does not exist
 * yet.
 */
export const ConfirmationActions = ({
    printing = false,
    withPrint = true,
    onPrint,
    onFinish,
    castEmailSent = false,
}: IConfirmationActionsProps): React.JSX.Element => {
    const {t} = useTranslation()

    return (
        <ActionsContainer
            className="actions-container confirmation-actions"
            sx={{
                "flexWrap": "wrap",
                "justifyContent": "flex-start",
                "gap": "12px 20px",
                "mt": "18px",
                "&:first-child": {mt: 0},
                [PHONE]: {flexDirection: "column", alignItems: "stretch", gap: "14px"},
            }}
        >
            {withPrint ? (
                <>
                    <StyledButton
                        className="print-receipt-button save-receipt-button"
                        onClick={onPrint}
                        disabled={printing || onPrint === undefined}
                        variant="secondary"
                        sx={{
                            "gap": "8px",
                            "minHeight": "44px",
                            "p": "6px 18px",
                            "fontWeight": 600,
                            "& .print-receipt-label": {whiteSpace: "normal", textAlign: "center"},
                            [PHONE]: {width: "100%"},
                        }}
                    >
                        {printing ? (
                            <StyledCircularProgress
                                className="print-receipt-progress"
                                color="inherit"
                                aria-hidden="true"
                            />
                        ) : (
                            <DownloadIcon />
                        )}
                        <Box className="print-receipt-label">
                            {t("confirmationScreen.saveReceiptButton")}
                        </Box>
                    </StyledButton>
                    {/* Generating the receipt is an asynchronous poll, so the wait
                        and its end are announced rather than shown only as a spinner. */}
                    <VisuallyHidden className="print-receipt-status" role="status">
                        {printing ? t("a11y.loading") : ""}
                    </VisuallyHidden>
                </>
            ) : null}
            {castEmailSent ? (
                <Box
                    component="p"
                    className="cast-email-note"
                    sx={{
                        "display": "flex",
                        "alignItems": "center",
                        "gap": "8px",
                        "flex": "1 1 180px",
                        "minWidth": 0,
                        "m": 0,
                        "fontSize": "14px",
                        "lineHeight": "20px",
                        "color": "#4A5262",
                        "& .cast-email-note-icon": {color: theme.palette.green.dark},
                        [PHONE]: {flex: "none"},
                    }}
                >
                    <MailIcon />
                    {t("confirmationScreen.castEmailNote")}
                </Box>
            ) : null}
            <StyledButton
                className="finish-button"
                onClick={onFinish}
                disabled={onFinish === undefined}
                sx={{
                    "ml": "auto",
                    "minWidth": "200px",
                    "minHeight": "44px",
                    "p": "6px 24px",
                    "fontWeight": 600,
                    "& .finish-button-label": {whiteSpace: "normal", textAlign: "center"},
                    [PHONE]: {width: "100%", minWidth: 0, ml: 0, mt: "2px"},
                }}
            >
                <Box className="finish-button-label">{t("confirmationScreen.finishButton")}</Box>
            </StyledButton>
        </ActionsContainer>
    )
}
