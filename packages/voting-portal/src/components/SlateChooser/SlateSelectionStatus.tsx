// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {ESlateSelectionStatus, ISlateSelectionSummary} from "../../services/SlateSelection"

const StatusLine = styled("p")(({theme}) => ({
    "margin": "0 0 12px",
    "fontSize": "0.875rem",
    "fontWeight": 700,
    "color": theme.palette.brandColor,
    "&.slate-selection-status-all": {
        color: theme.palette.green.dark,
    },
}))

export interface SlateSelectionStatusProps {
    summary: ISlateSelectionSummary
}

export const SlateSelectionStatus: React.FC<SlateSelectionStatusProps> = ({summary}) => {
    const {t} = useTranslation()

    if (summary.status === ESlateSelectionStatus.NONE) {
        return null
    }

    return (
        <StatusLine className={`slate-selection-status slate-selection-status-${summary.status}`}>
            {t(`slates.selection.${summary.status}`, {
                selected: summary.selected,
                total: summary.total,
            })}
        </StatusLine>
    )
}
