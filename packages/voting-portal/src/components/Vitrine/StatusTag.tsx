// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The pill that names a ballot's state next to its Tracking ID, such as "Cast"
// or "Not cast" (design board StatusTag).

import React from "react"
import {Box} from "@mui/material"
import type {SxProps, Theme} from "@mui/material/styles"

export enum EStatusTone {
    NEUTRAL = "neutral",
    SUCCESS = "success",
    INFO = "info",
    WARNING = "warning",
    DANGER = "danger",
}

interface IToneColors {
    background: string
    border: string
    text: string
    /** The dot's fill; the dot is hollow when it has none. */
    dot: string
    dotStroke: string
}

export const STATUS_TONES: Record<EStatusTone, IToneColors> = {
    [EStatusTone.NEUTRAL]: {
        background: "#F1F5F9",
        border: "#CBD5E1",
        text: "#475569",
        dot: "transparent",
        dotStroke: "#64748B",
    },
    [EStatusTone.SUCCESS]: {
        background: "#FFFFFF",
        border: "#047857",
        text: "#047857",
        dot: "#047857",
        dotStroke: "#047857",
    },
    [EStatusTone.INFO]: {
        background: "#EFF6FF",
        border: "#1A5FD6",
        text: "#1A5FD6",
        dot: "#1A5FD6",
        dotStroke: "#1A5FD6",
    },
    [EStatusTone.WARNING]: {
        background: "#FFF7ED",
        border: "#B45309",
        text: "#9A3412",
        dot: "#B45309",
        dotStroke: "#B45309",
    },
    [EStatusTone.DANGER]: {
        background: "#FEF2F2",
        border: "#B91C1C",
        text: "#B91C1C",
        dot: "#B91C1C",
        dotStroke: "#B91C1C",
    },
}

export interface IStatusTagProps {
    label: string
    tone?: EStatusTone
    sx?: SxProps<Theme>
}

export const StatusTag: React.FC<IStatusTagProps> = ({label, tone = EStatusTone.SUCCESS, sx}) => {
    const colors = STATUS_TONES[tone]
    return (
        <Box
            component="span"
            className="status-tag"
            data-tone={tone}
            sx={[
                {
                    display: "inline-flex",
                    alignItems: "center",
                    gap: "6px",
                    boxSizing: "border-box",
                    height: 24,
                    px: "10px",
                    border: `1px solid ${colors.border}`,
                    borderRadius: "999px",
                    bgcolor: colors.background,
                    color: colors.text,
                    fontSize: 12,
                    lineHeight: 1,
                    fontWeight: 600,
                    letterSpacing: "0.02em",
                    whiteSpace: "nowrap",
                },
                ...(Array.isArray(sx) ? sx : [sx]),
            ]}
        >
            <svg
                className="status-tag-dot"
                width="8"
                height="8"
                viewBox="0 0 8 8"
                aria-hidden="true"
                style={{flex: "none"}}
            >
                <circle
                    className="status-tag-dot-circle"
                    cx="4"
                    cy="4"
                    r="3.5"
                    fill={colors.dot}
                    stroke={colors.dotStroke}
                    strokeWidth="1"
                />
            </svg>
            {label}
        </Box>
    )
}
