// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {useTranslation} from "react-i18next"
import {styled} from "@mui/material/styles"
import CancelIcon from "@mui/icons-material/Cancel"
import CheckCircleIcon from "@mui/icons-material/CheckCircle"
import HourglassBottomIcon from "@mui/icons-material/HourglassBottom"
import PersonSearchIcon from "@mui/icons-material/PersonSearch"
import {IApplicationsStatus} from "@/types/applications"

const Chip = styled("span")({
    "display": "inline-flex",
    "alignItems": "center",
    "gap": "6px",
    "padding": "4px 12px 4px 8px",
    "borderRadius": "999px",
    "fontSize": "14px",
    "fontWeight": 600,
    "lineHeight": "20px",
    "whiteSpace": "nowrap",
    "& svg": {fontSize: "18px"},
    "&[data-status='PENDING']": {
        background: "#FFF3CD",
        border: "1px solid #F0D58C",
        color: "#7A5200",
    },
    "&[data-status='ACCEPTED']": {
        background: "#E7F8EF",
        border: "1px solid #A9E0C3",
        color: "#0B6B43",
    },
    "&[data-status='REJECTED']": {
        background: "#FDECEC",
        border: "1px solid #F5B5B5",
        color: "#B42318",
    },
})

const status = (value: string | null | undefined): IApplicationsStatus =>
    Object.values(IApplicationsStatus).find(
        (candidate) => candidate === (value ?? "").toUpperCase()
    ) ?? IApplicationsStatus.PENDING

export interface ApprovalStatusChipProps {
    /** The application's status, in any case. */
    status: string | null | undefined
}

/** Where an enrollment stands: needs review, approved or rejected. */
export const ApprovalStatusChip: React.FC<ApprovalStatusChipProps> = (props) => {
    const {t} = useTranslation()
    const value = status(props.status)
    return (
        <Chip data-status={value}>
            {value === IApplicationsStatus.ACCEPTED ? (
                <CheckCircleIcon aria-hidden />
            ) : value === IApplicationsStatus.REJECTED ? (
                <CancelIcon aria-hidden />
            ) : (
                <HourglassBottomIcon aria-hidden />
            )}
            {t(`approvalsScreen.status.${value}`)}
        </Chip>
    )
}

export interface ApprovalOutcomeChipProps {
    /** What a rule does with an enrollment. */
    decision: string | null | undefined
}

/** What a rule does: approve automatically, send to a person or reject. */
export const ApprovalOutcomeChip: React.FC<ApprovalOutcomeChipProps> = ({decision}) => {
    const {t} = useTranslation()
    const value = status(decision)
    return (
        <Chip data-status={value}>
            {value === IApplicationsStatus.ACCEPTED ? (
                <CheckCircleIcon aria-hidden />
            ) : value === IApplicationsStatus.REJECTED ? (
                <CancelIcon aria-hidden />
            ) : (
                <PersonSearchIcon aria-hidden />
            )}
            {t(`approvalsScreen.matrix.decisions.${value}`)}
        </Chip>
    )
}
