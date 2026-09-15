// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {TextField} from "@mui/material"

export interface MaterialLanguageFieldsProps {
    titleLabel: string
    subtitleLabel: string
    titleValue: string
    subtitleValue: string
    onTitleChange: (value: string) => void
    onSubtitleChange: (value: string) => void
}

export const MaterialLanguageFields: React.FC<MaterialLanguageFieldsProps> = ({
    titleLabel,
    subtitleLabel,
    titleValue,
    subtitleValue,
    onTitleChange,
    onSubtitleChange,
}) => (
    <>
        <TextField
            label={titleLabel}
            size="small"
            fullWidth
            value={titleValue}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => onTitleChange(e.target.value)}
        />
        <TextField
            label={subtitleLabel}
            size="small"
            fullWidth
            sx={{marginTop: "1rem"}}
            value={subtitleValue}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => onSubtitleChange(e.target.value)}
        />
    </>
)
