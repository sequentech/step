// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useNotify} from "react-admin"
import {useTranslation} from "react-i18next"
import {Box, IconButton, Tooltip} from "@mui/material"
import ContentCopyIcon from "@mui/icons-material/ContentCopy"
import {shortHash} from "@/components/signing/format"

interface ICopyableFingerprintProps {
    /** The complete value: in the tooltip, and what the button copies. */
    value: string
    /** The shortened text to show; by default the value's start and end. */
    short?: string
}

/**
 * A shortened SHA-256 with its complete value in a tooltip, where it can also
 * be selected, and a button that copies it to compare it with another record.
 */
export const CopyableFingerprint: React.FC<ICopyableFingerprintProps> = ({value, short}) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const copyLabel = String(t("copyableFingerprint.copy"))

    const copy = async () => {
        try {
            await navigator.clipboard.writeText(value)
            notify(t("copyableFingerprint.copied"), {type: "success"})
        } catch {
            notify(t("copyableFingerprint.copyError"), {type: "error"})
        }
    }

    return (
        <Box component="span" sx={{display: "inline-flex", alignItems: "center"}}>
            <Tooltip title={value} placement="top" describeChild>
                <span tabIndex={0}>{short ?? shortHash(value)}</span>
            </Tooltip>
            <Tooltip title={copyLabel} placement="top">
                <IconButton size="small" aria-label={copyLabel} onClick={copy} sx={{ml: 0.25}}>
                    <ContentCopyIcon sx={{fontSize: "1rem"}} />
                </IconButton>
            </Tooltip>
        </Box>
    )
}
