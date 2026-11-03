// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Chip} from "@mui/material"
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome"
import {faUserSecret} from "@fortawesome/free-solid-svg-icons"

/** Pseudonyms remain visually distinct from Ballot IDs in both portals. */
export const PseudonymChip: React.FC<{children: React.ReactNode; className?: string}> = ({
    children,
    className,
}) => (
    <Chip
        className={className}
        icon={<FontAwesomeIcon icon={faUserSecret} />}
        label={children}
        size="small"
        variant="outlined"
        sx={{
            "height": 24,
            "maxWidth": "100%",
            "borderRadius": "4px",
            "bgcolor": "#F7F9FC",
            "borderColor": "#DFE6EF",
            "color": "#405269",
            "fontFamily": "monospace",
            "fontSize": 12,
            "fontWeight": 600,
            "letterSpacing": 0.5,
            "& .MuiChip-icon": {color: "#64748B", fontSize: 11, ml: 1},
        }}
    />
)
