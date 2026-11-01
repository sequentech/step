// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// "Audit this ballot instead of casting it?" (design board AuditConfirm), for
// an event that keeps a transparency log. Auditing isn't a mistake to warn
// against: it is the check that keeps the portal honest, so the dialog says
// what happens, calmly, and its main button is the audit.

import React, {useId} from "react"
import {Box, ButtonBase, Dialog} from "@mui/material"
import {Trans, useTranslation} from "react-i18next"
import {shortBallotId} from "../../services/vitrine/ballotId"

const COLORS = {
    navy: "#0F054C",
    text: "#191D23",
    body: "#475569",
    list: "#334155",
} as const

const MONO = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace"

const KEYS = "reviewScreen.auditConfirm"

const buttonSx = {
    "display": "inline-flex",
    "alignItems": "center",
    "justifyContent": "center",
    "flex": "1 1 0",
    "minHeight": 44,
    "px": "12px",
    "py": "6px",
    "border": `1px solid ${COLORS.navy}`,
    "borderRadius": "4px",
    "fontSize": "16px",
    "fontWeight": 500,
    "&.Mui-focusVisible": {outline: `2px solid ${COLORS.navy}`, outlineOffset: 2},
} as const

const NeverIcon: React.FC = () => (
    <svg
        className="audit-confirm-never-icon"
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="none"
        stroke={COLORS.navy}
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
        style={{flex: "none", marginTop: 1}}
    >
        <circle className="audit-confirm-never-ring" cx="12" cy="12" r="9" />
        <path className="audit-confirm-never-bar" d="M8 12h8" />
    </svg>
)

const AgainIcon: React.FC = () => (
    <svg
        className="audit-confirm-again-icon"
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="none"
        stroke={COLORS.navy}
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
        style={{flex: "none", marginTop: 1}}
    >
        <path className="audit-confirm-again-path" d="M4 12a8 8 0 0 1 13.7-5.6L20 9" />
        <path className="audit-confirm-again-path" d="M20 4v5h-5" />
        <path className="audit-confirm-again-path" d="M20 12a8 8 0 0 1-13.7 5.6L4 15" />
        <path className="audit-confirm-again-path" d="M4 20v-5h5" />
    </svg>
)

export interface IAuditConfirmDialogProps {
    open: boolean
    /** The ballot's Tracking ID; the dialog names its short ID. */
    trackingId: string
    /** True when the voter chose to audit. */
    onClose: (audit: boolean) => void
}

export const AuditConfirmDialog: React.FC<IAuditConfirmDialogProps> = ({
    open,
    trackingId,
    onClose,
}) => {
    const {t} = useTranslation()
    const titleId = useId()
    const textId = useId()
    const item = {display: "flex", gap: "10px"} as const

    return (
        <Dialog
            open={open}
            onClose={() => onClose(false)}
            aria-labelledby={titleId}
            aria-describedby={textId}
            maxWidth={false}
            classes={{paper: "audit-confirm-dialog"}}
            slotProps={{
                paper: {
                    sx: {
                        width: {xs: "calc(100vw - 32px)", sm: 520},
                        m: {xs: "16px", sm: "32px"},
                        border: 0,
                        borderRadius: "4px",
                        color: COLORS.text,
                    },
                },
            }}
        >
            <Box className="audit-confirm-body" sx={{p: "24px 24px 0"}}>
                <Box
                    component="h2"
                    id={titleId}
                    className="audit-confirm-title"
                    sx={{m: 0, fontSize: "20px", lineHeight: "28px", fontWeight: 700}}
                >
                    {t(`${KEYS}.title`)}
                </Box>
                <Box
                    component="p"
                    id={textId}
                    className="audit-confirm-text"
                    sx={{m: "8px 0 0", fontSize: "15px", lineHeight: "22px", color: COLORS.body}}
                >
                    <Trans
                        i18nKey={`${KEYS}.text`}
                        values={{id: shortBallotId(trackingId)}}
                        components={{
                            Id: (
                                <b
                                    className="audit-confirm-id"
                                    style={{fontFamily: MONO, color: COLORS.text}}
                                />
                            ),
                        }}
                    />
                </Box>
                <Box
                    component="ul"
                    className="audit-confirm-points"
                    sx={{
                        display: "grid",
                        gap: "8px",
                        m: "16px 0 0",
                        p: 0,
                        listStyle: "none",
                        fontSize: "14px",
                        lineHeight: "20px",
                        color: COLORS.list,
                    }}
                >
                    <Box component="li" className="audit-confirm-never" sx={item}>
                        <NeverIcon />
                        <span className="audit-confirm-never-text">{t(`${KEYS}.never`)}</span>
                    </Box>
                    <Box component="li" className="audit-confirm-again" sx={item}>
                        <AgainIcon />
                        <span className="audit-confirm-again-text">{t(`${KEYS}.again`)}</span>
                    </Box>
                </Box>
            </Box>
            <Box
                className="audit-confirm-actions"
                sx={{
                    display: "flex",
                    // On a phone the audit comes first, above Cancel.
                    flexDirection: {xs: "column-reverse", sm: "row"},
                    gap: "10px",
                    p: "24px",
                }}
            >
                <ButtonBase
                    className="audit-confirm-cancel"
                    onClick={() => onClose(false)}
                    sx={{...buttonSx, bgcolor: "#fff", color: COLORS.navy}}
                >
                    {t(`${KEYS}.cancel`)}
                </ButtonBase>
                <ButtonBase
                    className="audit-confirm-audit"
                    onClick={() => onClose(true)}
                    sx={{...buttonSx, bgcolor: COLORS.navy, color: "#fff"}}
                >
                    {t(`${KEYS}.audit`)}
                </ButtonBase>
            </Box>
        </Dialog>
    )
}
