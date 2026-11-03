// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {IContest, ISlateCoverage, translate} from "@sequentech/ui-core"

import {ESlateCoverageLabel, getSlateCoverageLabel} from "../../services/SlateCoverage"
import type {IResolvedSlate} from "../../services/Slates"

const Chip = styled("span")(({theme}) => ({
    display: "inline-flex",
    padding: "4px 8px",
    borderRadius: "4px",
    background: theme.palette.lightBackground,
    color: theme.palette.customGrey.dark,
    fontSize: "12px",
    lineHeight: 1.35,
    textAlign: "end",
    overflowWrap: "anywhere",
    [theme.breakpoints.down("sm")]: {
        fontSize: "11px",
        padding: "4px 6px",
    },
}))

const Sentence = styled("p")(({theme}) => ({
    margin: "6px 0 13px",
    fontSize: "14px",
    lineHeight: 1.55,
    color: theme.palette.customGrey.dark,
    overflowWrap: "anywhere",
    [theme.breakpoints.down("sm")]: {
        fontSize: "13px",
        margin: "7px 0 0",
    },
}))

const Count = styled("p")(({theme}) => ({
    margin: "4px 0 12px",
    paddingTop: "12px",
    borderTop: `1px solid ${theme.palette.customGrey.light}`,
    fontSize: "13px",
    lineHeight: 1.55,
    color: theme.palette.customGrey.dark,
    [theme.breakpoints.down("sm")]: {
        fontSize: "12px",
        border: 0,
        padding: 0,
        margin: "8px 0 1px",
    },
}))

export interface ISlateCoverageChipProps {
    coverage: ISlateCoverage
    /** The contests of the ballot the slate has candidates in. */
    contests: Array<IContest>
}

/** Whether a slate is full, covers one office only, or is partial. */
export const SlateCoverageChip: React.FC<ISlateCoverageChipProps> = ({coverage, contests}) => {
    const {t, i18n} = useTranslation()
    const {label, contest} = getSlateCoverageLabel(coverage, contests)
    const text =
        label === ESlateCoverageLabel.FULL
            ? t("slates.coverage.full")
            : label === ESlateCoverageLabel.SINGLE_CONTEST && contest
              ? t("slates.coverage.singleContest", {
                    contest: translate(contest, "name", i18n.language) ?? "",
                })
              : t("slates.coverage.partial")

    return (
        <Chip className="slate-coverage-chip" data-coverage={coverage.kind}>
            {text}
        </Chip>
    )
}

export interface ISlateCoverageSummaryProps {
    slate: IResolvedSlate
}

/**
 * Which offices a slate has candidates for, and how many candidates and
 * offices that makes.
 */
export const SlateCoverageSummary: React.FC<ISlateCoverageSummaryProps> = ({slate}) => {
    const {t, i18n} = useTranslation()
    const candidates = slate.contests.reduce((total, entry) => total + entry.candidates.length, 0)
    const offices = slate.contests.length
    const coversEveryOffice = !slate.coverage || slate.coverage.uncovered_contest_ids.length === 0
    const officeNames = slate.contests
        .map(({contest}) => (translate(contest, "name", i18n.language) ?? "").toLocaleLowerCase())
        .join(", ")

    return (
        <>
            {slate.coverage ? (
                <Sentence className="slate-coverage-sentence">
                    {coversEveryOffice
                        ? t("slates.coverage.everyOffice")
                        : t("slates.coverage.someOffices", {offices: officeNames})}
                </Sentence>
            ) : null}
            <Count className="slate-count">
                {t("slates.coverage.candidates", {count: candidates})}
                {" · "}
                {t("slates.coverage.offices", {count: offices})}
            </Count>
        </>
    )
}
