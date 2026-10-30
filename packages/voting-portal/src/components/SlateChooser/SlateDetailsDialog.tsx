// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Typography} from "@mui/material"
import {styled} from "@mui/material/styles"
import {translate} from "@sequentech/ui-core"
import {useTranslation} from "react-i18next"
import {SlateDialog} from "../SlateDialog/SlateDialog"

import {ESlateCoverageLabel, getSlateCoverageLabel} from "../../services/SlateCoverage"
import type {IResolvedSlate} from "../../services/Slates"

const OfficeList = styled("ul")(({theme}) => ({
    "listStyle": "none",
    "margin": "22px 0 0",
    "padding": 0,
    "overflowWrap": "anywhere",
    "& .slate-details-office": {
        padding: "12px 0",
        borderBottom: "1px solid #e7eaee",
    },
    "& .slate-details-office:last-of-type": {
        borderBottom: "none",
    },
    "& .slate-details-office-name": {
        display: "block",
        marginBottom: "5px",
        fontSize: "14px",
        fontWeight: 700,
    },
    "& .slate-details-candidate": {
        display: "block",
        fontSize: "14px",
        lineHeight: 1.5,
        color: theme.palette.customGrey.dark,
    },
}))

export enum ESlateDetailsAction {
    SELECT = "select",
    EDIT = "edit",
    /** The slate cannot be selected on this ballot: only Back is offered. */
    UNAVAILABLE = "unavailable",
}

export interface SlateDetailsDialogProps {
    open: boolean
    slate: IResolvedSlate
    slateName: string
    /** What the primary answer does: select the slate or edit its selections. */
    action: ESlateDetailsAction
    /** Called with true for the primary answer, false for Back, the X or Escape. */
    handleClose: (confirmed: boolean) => void
}

/** Every candidate of a slate, office by office, before selecting it. */
export const SlateDetailsDialog: React.FC<SlateDetailsDialogProps> = ({
    open,
    slate,
    slateName,
    action,
    handleClose,
}) => {
    const {t, i18n} = useTranslation()
    const candidates = slate.contests.reduce((total, entry) => total + entry.candidates.length, 0)
    const contests = slate.contests.map((entry) => entry.contest)
    const coverage = (() => {
        if (!slate.coverage) {
            return undefined
        }
        const {label, contest} = getSlateCoverageLabel(slate.coverage, contests)
        if (label === ESlateCoverageLabel.FULL) {
            return t("slates.coverage.full")
        }
        if (label === ESlateCoverageLabel.SINGLE_CONTEST && contest) {
            return t("slates.coverage.singleContest", {
                contest: translate(contest, "name", i18n.language) ?? "",
            })
        }
        return t("slates.coverage.partial")
    })()
    const count = t("slates.coverage.candidates", {count: candidates})

    return (
        <SlateDialog
            className="slate-details-dialog"
            open={open}
            handleClose={handleClose}
            title={slateName}
            ok={
                action === ESlateDetailsAction.EDIT
                    ? t("slates.selection.edit")
                    : action === ESlateDetailsAction.SELECT
                      ? t("slates.view.select")
                      : undefined
            }
            cancel={t("slates.view.back")}
        >
            <Typography
                className="slate-details-summary"
                sx={{
                    margin: "0 0 12px",
                    fontSize: "inherit",
                    lineHeight: "inherit",
                    color: "customGrey.dark",
                }}
            >
                {coverage ? t("slates.view.summary", {coverage, candidates: count}) : count}
            </Typography>
            <OfficeList className="slate-details-offices">
                {slate.contests.map(({contest, candidates: members}) => (
                    <li
                        key={contest.id}
                        className="slate-details-office"
                        data-contest-id={contest.id}
                    >
                        <span className="slate-details-office-name">
                            {translate(contest, "name", i18n.language) ?? contest.id}
                        </span>
                        {members.map((candidate) => (
                            <span key={candidate.id} className="slate-details-candidate">
                                {translate(candidate, "name", i18n.language) ?? candidate.id}
                            </span>
                        ))}
                    </li>
                ))}
            </OfficeList>
        </SlateDialog>
    )
}
